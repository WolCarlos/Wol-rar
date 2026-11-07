use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use blake3::Hasher;
use chacha20poly1305::{
    KeyInit, XChaCha20Poly1305, XNonce,
    aead::{Aead, Payload},
};
use walkdir::WalkDir;
use zeroize::Zeroizing;

use crate::safe::{safe_destination, validate_member_path};
use crate::{ArchiveEntry, ArchiveError, CreateOptions};

const ARCHIVE_MAGIC: &[u8; 8] = b"WOLRAR01";
const VOLUME_MAGIC: &[u8; 8] = b"WOLVOL02";
const VERSION: u16 = 2;
const HEADER_LEN: u64 = 144;
const BODY_LEN_OFFSET: u64 = 96;
const ARCHIVE_DIGEST_OFFSET: u64 = 104;
const VOLUME_HEADER_LEN: u64 = 84;
const VOLUME_DIGEST_OFFSET: u64 = 52;
const ENTRY_FIXED_LEN: u64 = 48;
const ENTRY_PACKED_OFFSET: u64 = 12;
const ENTRY_COMPRESSED_OFFSET: u64 = 20;
const ENTRY_CHUNKS_OFFSET: u64 = 40;
const CHUNK_SIZE: u64 = 1024 * 1024;
const MAX_COMMENT_SIZE: usize = 16 * 1024 * 1024;
const MIN_VOLUME_SIZE: u64 = 64 * 1024;
const ARGON2_MEMORY_KIB: u32 = 32 * 1024;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;

#[derive(Debug, Clone)]
pub struct WolArchiveInfoV2 {
    pub comment: String,
    pub entry_count: u64,
    pub encrypted: bool,
    pub body_len: u64,
    pub volume_count: u32,
    pub volume_size: Option<u64>,
}

#[derive(Debug, Clone)]
struct V2Entry {
    path: String,
    directory: bool,
    encrypted: bool,
    size: u64,
    packed_size: u64,
    compressed_size: u64,
    modified: i64,
    mode: u32,
    chunk_count: u32,
    digest: [u8; 32],
    payload_offset: u64,
    index: u32,
    duplicate_of: Option<u32>,
}

#[derive(Debug)]
struct PlannedEntry {
    source: PathBuf,
    path: String,
    directory: bool,
    size: u64,
    modified: i64,
    mode: u32,
}

#[derive(Debug, Clone)]
struct VolumeMeta {
    path: PathBuf,
    index: u32,
    logical_offset: u64,
    payload_len: u64,
}

#[derive(Debug)]
struct VolumeHeader {
    index: u32,
    count: u32,
    archive_id: [u8; 16],
    logical_offset: u64,
    payload_len: u64,
}

pub fn create(
    path: &Path,
    sources: &[PathBuf],
    options: &CreateOptions,
    volume_size: Option<u64>,
) -> Result<WolArchiveInfoV2, ArchiveError> {
    if path.exists() {
        return Err(ArchiveError::DestinationExists(path.to_owned()));
    }
    if sources.is_empty() {
        return Err(ArchiveError::MissingSource(path.to_owned()));
    }
    if let Some(size) = volume_size
        && size < MIN_VOLUME_SIZE
    {
        return Err(ArchiveError::Message(format!(
            "Volume troppo piccolo: minimo {MIN_VOLUME_SIZE} byte"
        )));
    }
    if options.recovery_percent > 0 && volume_size.is_none() {
        return Err(ArchiveError::Message(
            "La recovery richiede una dimensione volume esplicita".into(),
        ));
    }
    let comment = options.comment.clone().unwrap_or_default();
    if comment.len() > MAX_COMMENT_SIZE {
        return Err(ArchiveError::Message("Commento troppo grande".into()));
    }
    let planned = collect_entries(sources, path)?;
    if planned.len() > u32::MAX as usize {
        return Err(ArchiveError::Message("Troppe entry per .wolrar v2".into()));
    }
    let entry_count = planned.len() as u64;
    let mut salt = [0u8; 16];
    let mut base_nonce = [0u8; 24];
    let mut archive_id = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|error| ArchiveError::Message(error.to_string()))?;
    getrandom::fill(&mut base_nonce).map_err(|error| ArchiveError::Message(error.to_string()))?;
    getrandom::fill(&mut archive_id).map_err(|error| ArchiveError::Message(error.to_string()))?;
    let encrypted = options.password.is_some();
    let cipher = options
        .password
        .as_ref()
        .map(|password| derive_cipher(password.as_bytes(), &salt))
        .transpose()?;

    let mut writer = VolumeWriter::new(path, volume_size, archive_id)?;
    write_archive_header(
        &mut writer,
        options.level,
        encrypted,
        volume_size,
        &salt,
        &base_nonce,
        &archive_id,
        entry_count,
        comment.len() as u32,
        0,
        &[0u8; 32],
    )?;
    writer.write_all(comment.as_bytes())?;

    let mut seen: HashMap<(u64, [u8; 32]), u32> = HashMap::new();
    for (index, item) in planned.iter().enumerate() {
        let mut entry = V2Entry {
            path: item.path.clone(),
            directory: item.directory,
            encrypted,
            size: item.size,
            packed_size: 0,
            compressed_size: 0,
            modified: item.modified,
            mode: item.mode,
            chunk_count: 0,
            digest: [0u8; 32],
            payload_offset: 0,
            index: index as u32,
            duplicate_of: None,
        };
        if entry.directory {
            write_entry_metadata(&mut writer, &entry)?;
            continue;
        }
        if encrypted || options.deduplicate {
            entry.digest = digest_file(&item.source)?;
        }
        if options.deduplicate {
            if let Some(source_index) = seen.get(&(entry.size, entry.digest)) {
                entry.duplicate_of = Some(*source_index);
                write_entry_metadata(&mut writer, &entry)?;
                continue;
            }
            seen.insert((entry.size, entry.digest), entry.index);
        }
        let metadata_offset = writer.position;
        write_entry_metadata(&mut writer, &entry)?;
        entry.payload_offset = writer.position;
        let mut input = HashReader::new(File::open(&item.source)?);
        let expected_digest = entry.digest;
        let (compressed_size, packed_size, chunk_count) = {
            let mut payload = ChunkWriter::new(&mut writer, cipher.as_ref(), &base_nonce, &entry);
            {
                let mut encoder =
                    zstd::stream::write::Encoder::new(&mut payload, options.level as i32)?;
                encoder.set_pledged_src_size(Some(entry.size))?;
                encoder.include_checksum(true)?;
                std::io::copy(&mut input, &mut encoder)?;
                encoder.finish()?;
            }
            payload.finish()?
        };
        entry.compressed_size = compressed_size;
        entry.packed_size = packed_size;
        entry.chunk_count = chunk_count;
        entry.digest = input.finish();
        if (encrypted || options.deduplicate) && entry.digest != expected_digest {
            return Err(ArchiveError::Message(format!(
                "Il file è cambiato durante la creazione: {}",
                entry.path
            )));
        }
        writer.write_at(
            metadata_offset + ENTRY_PACKED_OFFSET,
            &entry.packed_size.to_le_bytes(),
        )?;
        writer.write_at(
            metadata_offset + ENTRY_COMPRESSED_OFFSET,
            &entry.compressed_size.to_le_bytes(),
        )?;
        writer.write_at(
            metadata_offset + ENTRY_CHUNKS_OFFSET,
            &entry.chunk_count.to_le_bytes(),
        )?;
        writer.write_at(
            metadata_offset + ENTRY_FIXED_LEN + entry.path.len() as u64,
            &entry.digest,
        )?;
    }

    let body_len = writer.position - HEADER_LEN;
    writer.write_at(BODY_LEN_OFFSET, &body_len.to_le_bytes())?;
    writer.seal_volumes()?;
    let digest = archive_digest(path, body_len)?;
    writer.write_at(ARCHIVE_DIGEST_OFFSET, &digest)?;
    let volume_count = writer.metas.len() as u32;
    writer.finish()?;
    if options.recovery_percent > 0
        && let Err(error) = crate::wolrecovery::create_recovery(
            path,
            volume_size.expect("volume recovery verificato"),
            archive_id,
            options.recovery_percent,
        )
    {
        for index in 1..=volume_count {
            let _ = fs::remove_file(if index == 1 {
                path.to_owned()
            } else {
                path.with_extension(format!("w{index:02}"))
            });
        }
        return Err(error);
    }
    Ok(WolArchiveInfoV2 {
        comment,
        entry_count,
        encrypted,
        body_len,
        volume_count,
        volume_size,
    })
}

pub fn inspect(path: &Path) -> Result<WolArchiveInfoV2, ArchiveError> {
    let mut reader = VolumeReader::new(path, false)?;
    let stream_len = reader.total_len;
    let header = read_archive_header(&mut reader)?;
    validate_header(&header, stream_len)?;
    let mut comment = vec![0u8; header.comment_len as usize];
    reader.read_exact(&mut comment)?;
    let comment =
        String::from_utf8(comment).map_err(|error| ArchiveError::Message(error.to_string()))?;
    Ok(WolArchiveInfoV2 {
        comment,
        entry_count: header.entry_count,
        encrypted: header.flags & 1 != 0,
        body_len: header.body_len,
        volume_count: reader.metas.len() as u32,
        volume_size: (header.volume_size != 0).then_some(header.volume_size),
    })
}

pub fn list(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let (_, entries) = read_entries(path)?;
    Ok(entries
        .into_iter()
        .map(|entry| ArchiveEntry {
            path: entry.path,
            size: entry.size,
            packed_size: entry.packed_size,
            modified: entry.modified.to_string(),
            directory: entry.directory,
            encrypted: entry.encrypted,
        })
        .collect())
}

pub fn extract(
    path: &Path,
    destination: &Path,
    password: Option<&str>,
) -> Result<(), ArchiveError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(ArchiveError::Message(
                "La destinazione non è una cartella sicura".into(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(destination)?
        }
        Err(error) => return Err(error.into()),
    }
    let (info, entries) = read_entries(path)?;
    if info.encrypted && password.is_none() {
        return Err(ArchiveError::Message(
            "Password richiesta per .wolrar".into(),
        ));
    }
    let header = read_archive_header(&mut VolumeReader::new(path, false)?)?;
    let cipher = password
        .map(|value| derive_cipher(value.as_bytes(), &header.salt))
        .transpose()?;
    preflight_destinations(destination, &entries)?;
    let staging = tempfile::tempdir_in(destination)?;
    for entry in &entries {
        let output = safe_destination(staging.path(), &entry.path)?;
        if entry.directory {
            fs::create_dir_all(output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(source_index) = entry.duplicate_of {
            let source = &entries[source_index as usize];
            let source_path = safe_destination(staging.path(), &source.path)?;
            let copied = fs::copy(source_path, &output)?;
            if copied != entry.size || digest_file(&output)? != entry.digest {
                return Err(ArchiveError::Message(format!(
                    "Digest duplicato non valido per {}",
                    entry.path
                )));
            }
            continue;
        }
        decode_entry(path, entry, &output, cipher.as_ref(), &header.base_nonce)?;
    }

    let mut published = Vec::new();
    let result = (|| -> Result<(), ArchiveError> {
        for entry in &entries {
            let staged = safe_destination(staging.path(), &entry.path)?;
            let output = safe_destination(destination, &entry.path)?;
            if entry.directory {
                create_secure_directories(destination, &output)?;
                continue;
            }
            if let Some(parent) = output.parent() {
                create_secure_directories(destination, parent)?;
            }
            if let Err(error) = fs::hard_link(&staged, &output) {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(ArchiveError::DestinationExists(output));
                }
                return Err(error.into());
            }
            published.push(output.clone());
            fs::remove_file(staged)?;
            apply_metadata(&output, entry)?;
        }
        for entry in entries.iter().rev().filter(|entry| entry.directory) {
            apply_metadata(&safe_destination(destination, &entry.path)?, entry)?;
        }
        Ok(())
    })();
    if result.is_err() {
        for output in published.into_iter().rev() {
            let _ = fs::remove_file(output);
        }
    }
    result
}

fn read_entries(path: &Path) -> Result<(WolArchiveInfoV2, Vec<V2Entry>), ArchiveError> {
    let info = inspect(path)?;
    verify_archive_digest(path, info.body_len)?;
    let body_end = HEADER_LEN + info.body_len;
    let mut reader = VolumeReader::new(path, false)?;
    reader.seek(SeekFrom::Start(HEADER_LEN + info.comment.len() as u64))?;
    let mut entries = Vec::new();
    let mut paths = HashSet::new();
    for index in 0..info.entry_count {
        let path_len = read_u16(&mut reader)?;
        let flags = read_u16(&mut reader)?;
        if flags & !7 != 0 {
            return Err(ArchiveError::UnsupportedFormat(format!(
                "Flags entry non supportate: {flags}"
            )));
        }
        let size = read_u64(&mut reader)?;
        let packed_size = read_u64(&mut reader)?;
        let compressed_size = read_u64(&mut reader)?;
        let modified = read_i64(&mut reader)?;
        let mode = read_u32(&mut reader)?;
        let chunk_count = read_u32(&mut reader)?;
        let duplicate_index = read_u32(&mut reader)?;
        let duplicate_of = (flags & 4 != 0).then_some(duplicate_index);
        if duplicate_of.is_none() && duplicate_index != u32::MAX {
            return Err(ArchiveError::UnsupportedFormat(
                "Riferimento deduplicazione non valido".into(),
            ));
        }
        let mut path_bytes = vec![0u8; path_len as usize];
        reader.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)
            .map_err(|error| ArchiveError::Message(error.to_string()))?;
        validate_member_path(&path)?;
        if !paths.insert(path_key(&path)) {
            return Err(ArchiveError::Message(format!("Percorso duplicato: {path}")));
        }
        let mut digest = [0u8; 32];
        reader.read_exact(&mut digest)?;
        let payload_offset = reader.position;
        let directory = flags & 1 != 0;
        let encrypted = flags & 2 != 0;
        if encrypted != info.encrypted {
            return Err(ArchiveError::Message(
                "Flags di cifratura non coerenti".into(),
            ));
        }
        if directory
            && (size != 0
                || packed_size != 0
                || compressed_size != 0
                || chunk_count != 0
                || duplicate_of.is_some()
                || digest != [0u8; 32])
        {
            return Err(ArchiveError::Message(
                "Metadati directory non validi".into(),
            ));
        }
        if !directory {
            if let Some(source_index) = duplicate_of {
                if packed_size != 0 || compressed_size != 0 || chunk_count != 0 {
                    return Err(ArchiveError::Message("Payload duplicato non valido".into()));
                }
                let source = entries
                    .get(source_index as usize)
                    .filter(|_| source_index < index as u32)
                    .ok_or_else(|| {
                        ArchiveError::Message("Riferimento duplicato fuori sequenza".into())
                    })?;
                let source: &V2Entry = source;
                if source.directory
                    || source.size != size
                    || source.digest != digest
                    || source.encrypted != encrypted
                {
                    return Err(ArchiveError::Message(
                        "Riferimento duplicato non coerente".into(),
                    ));
                }
            } else {
                let expected_chunks = chunks_for(compressed_size);
                if compressed_size == 0 || chunk_count as u64 != expected_chunks {
                    return Err(ArchiveError::Message(format!(
                        "Chunk non coerenti per {path}"
                    )));
                }
                let expected_packed =
                    compressed_size + if encrypted { expected_chunks * 16 } else { 0 };
                if packed_size != expected_packed {
                    return Err(ArchiveError::Message(format!(
                        "Payload non coerente per {path}"
                    )));
                }
            }
        }
        let next = payload_offset
            .checked_add(packed_size)
            .ok_or_else(|| ArchiveError::Message("Offset payload non valido".into()))?;
        if next > body_end {
            return Err(ArchiveError::Message(
                "Payload oltre la fine dell'archivio".into(),
            ));
        }
        reader.seek(SeekFrom::Start(next))?;
        entries.push(V2Entry {
            path,
            directory,
            encrypted,
            size,
            packed_size,
            compressed_size,
            modified,
            mode,
            chunk_count,
            digest,
            payload_offset,
            index: index as u32,
            duplicate_of,
        });
    }
    if reader.position != body_end {
        return Err(ArchiveError::Message(
            "Dati oltre l'ultima entry dichiarata".into(),
        ));
    }
    Ok((info, entries))
}

struct ArchiveHeader {
    flags: u16,
    volume_size: u64,
    salt: [u8; 16],
    base_nonce: [u8; 24],
    comment_len: u32,
    entry_count: u64,
    body_len: u64,
}

#[allow(clippy::too_many_arguments)]
fn write_archive_header(
    writer: &mut VolumeWriter,
    level: u32,
    encrypted: bool,
    volume_size: Option<u64>,
    salt: &[u8; 16],
    base_nonce: &[u8; 24],
    archive_id: &[u8; 16],
    entry_count: u64,
    comment_len: u32,
    body_len: u64,
    digest: &[u8; 32],
) -> Result<(), ArchiveError> {
    writer.write_all(ARCHIVE_MAGIC)?;
    writer.write_all(&VERSION.to_le_bytes())?;
    writer.write_all(&u16::from(encrypted).to_le_bytes())?;
    writer.write_all(&(level as i32).to_le_bytes())?;
    writer.write_all(&(CHUNK_SIZE as u32).to_le_bytes())?;
    writer.write_all(&comment_len.to_le_bytes())?;
    writer.write_all(&volume_size.unwrap_or(0).to_le_bytes())?;
    writer.write_all(salt)?;
    writer.write_all(base_nonce)?;
    writer.write_all(archive_id)?;
    writer.write_all(&entry_count.to_le_bytes())?;
    writer.write_all(&body_len.to_le_bytes())?;
    writer.write_all(digest)?;
    writer.write_all(&[0u8; 8])?;
    Ok(())
}

fn read_archive_header(reader: &mut VolumeReader) -> Result<ArchiveHeader, ArchiveError> {
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic != ARCHIVE_MAGIC {
        return Err(ArchiveError::UnsupportedFormat(
            "Magic .wolrar non valido".into(),
        ));
    }
    let version = read_u16(reader)?;
    if version != VERSION {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Versione .wolrar {version}"
        )));
    }
    let flags = read_u16(reader)?;
    let _level = read_i32(reader)?;
    let chunk_size = read_u32(reader)?;
    if chunk_size as u64 != CHUNK_SIZE {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Chunk size non supportata: {chunk_size}"
        )));
    }
    let comment_len = read_u32(reader)?;
    let volume_size = read_u64(reader)?;
    let mut salt = [0u8; 16];
    reader.read_exact(&mut salt)?;
    let mut base_nonce = [0u8; 24];
    reader.read_exact(&mut base_nonce)?;
    let mut archive_id = [0u8; 16];
    reader.read_exact(&mut archive_id)?;
    let entry_count = read_u64(reader)?;
    let body_len = read_u64(reader)?;
    let mut digest = [0u8; 32];
    reader.read_exact(&mut digest)?;
    let mut reserved = [0u8; 8];
    reader.read_exact(&mut reserved)?;
    if reserved != [0u8; 8] {
        return Err(ArchiveError::UnsupportedFormat(
            "Flags header v2 non supportate".into(),
        ));
    }
    Ok(ArchiveHeader {
        flags,
        volume_size,
        salt,
        base_nonce,
        comment_len,
        entry_count,
        body_len,
    })
}

fn validate_header(header: &ArchiveHeader, stream_len: u64) -> Result<(), ArchiveError> {
    if header.flags & !1 != 0 {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Flags .wolrar non supportate: {}",
            header.flags
        )));
    }
    if header.entry_count > u32::MAX as u64 {
        return Err(ArchiveError::UnsupportedFormat(
            "Troppe entry per .wolrar v2".into(),
        ));
    }
    if header.comment_len as usize > MAX_COMMENT_SIZE || header.comment_len as u64 > header.body_len
    {
        return Err(ArchiveError::Message("Commento .wolrar non valido".into()));
    }
    if header.volume_size != 0 && header.volume_size < MIN_VOLUME_SIZE {
        return Err(ArchiveError::Message("Dimensione volume non valida".into()));
    }
    let expected = HEADER_LEN
        .checked_add(header.body_len)
        .ok_or_else(|| ArchiveError::Message("Lunghezza .wolrar non valida".into()))?;
    if stream_len != expected {
        return Err(ArchiveError::Message(
            "Lunghezza stream .wolrar non coerente".into(),
        ));
    }
    Ok(())
}

fn write_entry_metadata(writer: &mut VolumeWriter, entry: &V2Entry) -> Result<(), ArchiveError> {
    let path_len = u16::try_from(entry.path.len())
        .map_err(|_| ArchiveError::UnsafePath(entry.path.clone()))?;
    writer.write_all(&path_len.to_le_bytes())?;
    let flags = u16::from(entry.directory)
        | (u16::from(entry.encrypted) << 1)
        | (u16::from(entry.duplicate_of.is_some()) << 2);
    writer.write_all(&flags.to_le_bytes())?;
    writer.write_all(&entry.size.to_le_bytes())?;
    writer.write_all(&entry.packed_size.to_le_bytes())?;
    writer.write_all(&entry.compressed_size.to_le_bytes())?;
    writer.write_all(&entry.modified.to_le_bytes())?;
    writer.write_all(&entry.mode.to_le_bytes())?;
    writer.write_all(&entry.chunk_count.to_le_bytes())?;
    writer.write_all(&entry.duplicate_of.unwrap_or(u32::MAX).to_le_bytes())?;
    writer.write_all(entry.path.as_bytes())?;
    writer.write_all(&entry.digest)?;
    Ok(())
}

struct VolumeWriter {
    first_path: PathBuf,
    volume_size: Option<u64>,
    archive_id: [u8; 16],
    metas: Vec<VolumeMeta>,
    current: Option<File>,
    position: u64,
    keep: bool,
}

impl VolumeWriter {
    fn new(
        path: &Path,
        volume_size: Option<u64>,
        archive_id: [u8; 16],
    ) -> Result<Self, ArchiveError> {
        let mut writer = Self {
            first_path: path.to_owned(),
            volume_size,
            archive_id,
            metas: Vec::new(),
            current: None,
            position: 0,
            keep: false,
        };
        writer.open_next()?;
        Ok(writer)
    }

    fn open_next(&mut self) -> Result<(), ArchiveError> {
        if let Some(file) = self.current.take() {
            file.sync_all()?;
        }
        let index = self.metas.len() as u32 + 1;
        let path = volume_path(&self.first_path, index);
        let logical_offset = self.position;
        let mut file = File::create_new(&path)?;
        write_volume_header(
            &mut file,
            index,
            0,
            &self.archive_id,
            logical_offset,
            0,
            &[0u8; 32],
        )?;
        self.metas.push(VolumeMeta {
            path,
            index,
            logical_offset,
            payload_len: 0,
        });
        self.current = Some(file);
        Ok(())
    }

    fn write_at(&self, logical_offset: u64, bytes: &[u8]) -> Result<(), ArchiveError> {
        let mut remaining = bytes;
        let mut offset = logical_offset;
        while !remaining.is_empty() {
            let meta = self
                .metas
                .iter()
                .rev()
                .find(|meta| {
                    offset >= meta.logical_offset && offset < meta.logical_offset + meta.payload_len
                })
                .ok_or_else(|| ArchiveError::Message("Patch oltre il payload scritto".into()))?;
            let relative = offset - meta.logical_offset;
            let available = (meta.payload_len - relative) as usize;
            let amount = available.min(remaining.len());
            let mut file = fs::OpenOptions::new().write(true).open(&meta.path)?;
            file.seek(SeekFrom::Start(VOLUME_HEADER_LEN + relative))?;
            file.write_all(&remaining[..amount])?;
            file.sync_all()?;
            offset += amount as u64;
            remaining = &remaining[amount..];
        }
        Ok(())
    }

    fn seal_volumes(&mut self) -> Result<(), ArchiveError> {
        if let Some(file) = self.current.take() {
            file.sync_all()?;
        }
        let count = self.metas.len() as u32;
        for meta in &self.metas {
            let digest = volume_digest(&meta.path, meta.payload_len)?;
            let mut file = fs::OpenOptions::new().write(true).open(&meta.path)?;
            file.seek(SeekFrom::Start(16))?;
            file.write_all(&count.to_le_bytes())?;
            file.seek(SeekFrom::Start(44))?;
            file.write_all(&meta.payload_len.to_le_bytes())?;
            file.seek(SeekFrom::Start(VOLUME_DIGEST_OFFSET))?;
            file.write_all(&digest)?;
            file.sync_all()?;
        }
        Ok(())
    }

    fn finish(mut self) -> Result<(), ArchiveError> {
        self.seal_volumes()?;
        self.keep = true;
        Ok(())
    }
}

impl Write for VolumeWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let mut written = 0;
        while written < buffer.len() {
            let index = self.metas.len() - 1;
            let capacity = self.volume_size.map(|size| size - VOLUME_HEADER_LEN);
            let remaining = capacity.map(|capacity| capacity - self.metas[index].payload_len);
            if remaining == Some(0) {
                self.open_next()
                    .map_err(|error| io_error(error.to_string()))?;
                continue;
            }
            let amount = remaining.map_or(buffer.len() - written, |remaining| {
                remaining.min((buffer.len() - written) as u64) as usize
            });
            let file = self
                .current
                .as_mut()
                .ok_or_else(|| io_error("Volume non aperto"))?;
            file.write_all(&buffer[written..written + amount])?;
            self.metas[index].payload_len += amount as u64;
            self.position += amount as u64;
            written += amount;
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self.current.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl Drop for VolumeWriter {
    fn drop(&mut self) {
        if !self.keep {
            for meta in &self.metas {
                let _ = fs::remove_file(&meta.path);
            }
        }
    }
}

struct VolumeReader {
    metas: Vec<VolumeMeta>,
    current: Option<(u32, File)>,
    verified: HashSet<u32>,
    position: u64,
    total_len: u64,
    verify: bool,
}

impl VolumeReader {
    fn new(path: &Path, verify: bool) -> Result<Self, ArchiveError> {
        let first = read_volume_header(path)?;
        if first.count == 0 {
            return Err(ArchiveError::Message(
                "Volume .wolrar non finalizzato".into(),
            ));
        }
        let mut metas = Vec::new();
        let mut expected_offset = 0u64;
        let archive_id = first.archive_id;
        for index in 1..=first.count {
            let volume = volume_path(path, index);
            let header = read_volume_header(&volume)?;
            if header.index != index
                || header.count != first.count
                || header.archive_id != archive_id
                || header.logical_offset != expected_offset
            {
                return Err(ArchiveError::Message("Manifest volume non coerente".into()));
            }
            metas.push(VolumeMeta {
                path: volume,
                index,
                logical_offset: expected_offset,
                payload_len: header.payload_len,
            });
            expected_offset = expected_offset
                .checked_add(header.payload_len)
                .ok_or_else(|| ArchiveError::Message("Lunghezza volumi non valida".into()))?;
        }
        Ok(Self {
            metas,
            current: None,
            verified: HashSet::new(),
            position: 0,
            total_len: expected_offset,
            verify,
        })
    }

    fn open(&mut self, index: u32) -> Result<(), ArchiveError> {
        if self
            .current
            .as_ref()
            .is_some_and(|(current, _)| *current == index)
        {
            return Ok(());
        }
        let meta = self
            .metas
            .get(index as usize - 1)
            .ok_or_else(|| ArchiveError::Message("Volume oltre la sequenza".into()))?;
        let mut file = File::open(&meta.path)?;
        if self.verify && !self.verified.contains(&index) {
            let expected = read_volume_header(&meta.path)?;
            let actual = volume_digest(&meta.path, meta.payload_len)?;
            let stored = read_volume_digest(&meta.path)?;
            let _ = expected;
            if actual != stored {
                return Err(ArchiveError::Message(format!(
                    "Digest volume {index} non valido"
                )));
            }
            self.verified.insert(index);
        }
        file.seek(SeekFrom::Start(VOLUME_HEADER_LEN))?;
        self.current = Some((index, file));
        Ok(())
    }
}

impl Read for VolumeReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() || self.position >= self.total_len {
            return Ok(0);
        }
        let index = self
            .metas
            .iter()
            .rev()
            .find(|meta| {
                self.position >= meta.logical_offset
                    && self.position < meta.logical_offset + meta.payload_len
            })
            .map(|meta| meta.index)
            .ok_or_else(|| io_error("Offset oltre i volumi"))?;
        self.open(index)
            .map_err(|error| io_error(error.to_string()))?;
        let meta = &self.metas[index as usize - 1];
        let relative = self.position - meta.logical_offset;
        let available = meta.payload_len - relative;
        let amount = available.min(buffer.len() as u64) as usize;
        let (_, file) = self
            .current
            .as_mut()
            .ok_or_else(|| io_error("Volume non aperto"))?;
        file.seek(SeekFrom::Start(VOLUME_HEADER_LEN + relative))?;
        let read = file.read(&mut buffer[..amount])?;
        self.position += read as u64;
        Ok(read)
    }
}

impl Seek for VolumeReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        let next = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.position as i128 + value as i128,
            SeekFrom::End(value) => self.total_len as i128 + value as i128,
        };
        if next < 0 || next > self.total_len as i128 {
            return Err(io_error("Seek oltre lo stream .wolrar"));
        }
        self.position = next as u64;
        Ok(self.position)
    }
}

fn write_volume_header(
    writer: &mut File,
    index: u32,
    count: u32,
    archive_id: &[u8; 16],
    logical_offset: u64,
    payload_len: u64,
    digest: &[u8; 32],
) -> Result<(), ArchiveError> {
    writer.write_all(VOLUME_MAGIC)?;
    writer.write_all(&VERSION.to_le_bytes())?;
    writer.write_all(&0u16.to_le_bytes())?;
    writer.write_all(&index.to_le_bytes())?;
    writer.write_all(&count.to_le_bytes())?;
    writer.write_all(archive_id)?;
    writer.write_all(&logical_offset.to_le_bytes())?;
    writer.write_all(&payload_len.to_le_bytes())?;
    writer.write_all(digest)?;
    Ok(())
}

fn read_volume_header(path: &Path) -> Result<VolumeHeader, ArchiveError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != VOLUME_MAGIC {
        return Err(ArchiveError::UnsupportedFormat(
            "Volume .wolrar non valido".into(),
        ));
    }
    let version = read_u16(&mut file)?;
    if version != VERSION {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Versione volume {version}"
        )));
    }
    let flags = read_u16(&mut file)?;
    if flags != 0 {
        return Err(ArchiveError::UnsupportedFormat(
            "Flags volume non supportate".into(),
        ));
    }
    let index = read_u32(&mut file)?;
    let count = read_u32(&mut file)?;
    let mut archive_id = [0u8; 16];
    file.read_exact(&mut archive_id)?;
    let logical_offset = read_u64(&mut file)?;
    let payload_len = read_u64(&mut file)?;
    let actual_len = file.metadata()?.len();
    if actual_len != VOLUME_HEADER_LEN + payload_len {
        return Err(ArchiveError::Message(
            "Lunghezza volume non coerente".into(),
        ));
    }
    Ok(VolumeHeader {
        index,
        count,
        archive_id,
        logical_offset,
        payload_len,
    })
}

fn read_volume_digest(path: &Path) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(VOLUME_DIGEST_OFFSET))?;
    let mut digest = [0u8; 32];
    file.read_exact(&mut digest)?;
    Ok(digest)
}

fn volume_digest(path: &Path, payload_len: u64) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut (&mut file).take(VOLUME_DIGEST_OFFSET))?;
    file.seek(SeekFrom::Start(VOLUME_HEADER_LEN))?;
    hasher.update_reader(&mut file.take(payload_len))?;
    Ok(hasher.finalize().into())
}

fn archive_digest(path: &Path, body_len: u64) -> Result<[u8; 32], ArchiveError> {
    let mut reader = VolumeReader::new(path, false)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut (&mut reader).take(ARCHIVE_DIGEST_OFFSET))?;
    reader.seek(SeekFrom::Start(HEADER_LEN))?;
    hasher.update_reader(&mut reader.take(body_len))?;
    Ok(hasher.finalize().into())
}

fn verify_archive_digest(path: &Path, body_len: u64) -> Result<(), ArchiveError> {
    let mut reader = VolumeReader::new(path, true)?;
    reader.seek(SeekFrom::Start(ARCHIVE_DIGEST_OFFSET))?;
    let mut expected = [0u8; 32];
    reader.read_exact(&mut expected)?;
    let actual = archive_digest(path, body_len)?;
    if actual != expected {
        return Err(ArchiveError::Message(
            "Digest archivio .wolrar non valido".into(),
        ));
    }
    Ok(())
}

struct ChunkWriter<'a> {
    writer: &'a mut VolumeWriter,
    cipher: Option<&'a XChaCha20Poly1305>,
    base_nonce: &'a [u8; 24],
    entry: &'a V2Entry,
    buffer: Vec<u8>,
    compressed_size: u64,
    packed_size: u64,
    chunk_count: u32,
}

impl<'a> ChunkWriter<'a> {
    fn new(
        writer: &'a mut VolumeWriter,
        cipher: Option<&'a XChaCha20Poly1305>,
        base_nonce: &'a [u8; 24],
        entry: &'a V2Entry,
    ) -> Self {
        Self {
            writer,
            cipher,
            base_nonce,
            entry,
            buffer: Vec::with_capacity(CHUNK_SIZE as usize),
            compressed_size: 0,
            packed_size: 0,
            chunk_count: 0,
        }
    }

    fn seal_chunk(&mut self) -> std::io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        if let Some(cipher) = self.cipher {
            let aad = entry_aad(self.entry, self.chunk_count);
            let nonce = chunk_nonce(self.base_nonce, self.entry.index, self.chunk_count);
            let ciphertext = cipher
                .encrypt(
                    &nonce,
                    Payload {
                        msg: &self.buffer,
                        aad: &aad,
                    },
                )
                .map_err(|error| io_error(error.to_string()))?;
            self.writer.write_all(&ciphertext)?;
            self.packed_size += ciphertext.len() as u64;
        } else {
            self.writer.write_all(&self.buffer)?;
            self.packed_size += self.buffer.len() as u64;
        }
        self.buffer.clear();
        self.chunk_count += 1;
        Ok(())
    }

    fn finish(mut self) -> std::io::Result<(u64, u64, u32)> {
        self.seal_chunk()?;
        Ok((self.compressed_size, self.packed_size, self.chunk_count))
    }
}

impl Write for ChunkWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.compressed_size += buffer.len() as u64;
        let mut remaining = buffer;
        while !remaining.is_empty() {
            let amount = (CHUNK_SIZE as usize - self.buffer.len()).min(remaining.len());
            self.buffer.extend_from_slice(&remaining[..amount]);
            remaining = &remaining[amount..];
            if self.buffer.len() == CHUNK_SIZE as usize {
                self.seal_chunk()?;
            }
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

struct ChunkReader<'a> {
    reader: VolumeReader,
    entry: &'a V2Entry,
    cipher: &'a XChaCha20Poly1305,
    base_nonce: &'a [u8; 24],
    chunk_index: u32,
    compressed_offset: u64,
    buffer: Zeroizing<Vec<u8>>,
    buffer_offset: usize,
}

impl Read for ChunkReader<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.buffer_offset == self.buffer.len() && self.chunk_index < self.entry.chunk_count {
            let compressed_remaining = self.entry.compressed_size - self.compressed_offset;
            let compressed_len = compressed_remaining.min(CHUNK_SIZE) as usize;
            let mut ciphertext = vec![0u8; compressed_len + 16];
            self.reader.read_exact(&mut ciphertext)?;
            let aad = entry_aad(self.entry, self.chunk_index);
            let nonce = chunk_nonce(self.base_nonce, self.entry.index, self.chunk_index);
            self.buffer = Zeroizing::new(
                self.cipher
                    .decrypt(
                        &nonce,
                        Payload {
                            msg: &ciphertext,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| {
                        io_error(format!("Autenticazione non valida per {}", self.entry.path))
                    })?,
            );
            if self.buffer.len() != compressed_len {
                return Err(io_error("Chunk decifrato con lunghezza non valida"));
            }
            self.buffer_offset = 0;
            self.compressed_offset += compressed_len as u64;
            self.chunk_index += 1;
        }
        if self.buffer_offset == self.buffer.len() {
            return Ok(0);
        }
        let amount = output.len().min(self.buffer.len() - self.buffer_offset);
        output[..amount]
            .copy_from_slice(&self.buffer[self.buffer_offset..self.buffer_offset + amount]);
        self.buffer_offset += amount;
        Ok(amount)
    }
}

fn decode_entry(
    path: &Path,
    entry: &V2Entry,
    output: &Path,
    cipher: Option<&XChaCha20Poly1305>,
    base_nonce: &[u8; 24],
) -> Result<(), ArchiveError> {
    let target = File::options().write(true).create_new(true).open(output)?;
    let mut target = HashWriter::new(target, entry.size);
    let mut reader = VolumeReader::new(path, false)?;
    reader.seek(SeekFrom::Start(entry.payload_offset))?;
    if entry.encrypted {
        let cipher =
            cipher.ok_or_else(|| ArchiveError::Message("Password richiesta per .wolrar".into()))?;
        let chunks = ChunkReader {
            reader,
            entry,
            cipher,
            base_nonce,
            chunk_index: 0,
            compressed_offset: 0,
            buffer: Zeroizing::new(Vec::new()),
            buffer_offset: 0,
        };
        zstd::stream::copy_decode(chunks, &mut target)?;
    } else {
        zstd::stream::copy_decode(reader.take(entry.packed_size), &mut target)?;
    }
    let (written, digest) = target.finish()?;
    if written != entry.size || digest != entry.digest {
        return Err(ArchiveError::Message(format!(
            "Digest non valido per {}",
            entry.path
        )));
    }
    Ok(())
}

fn entry_aad(entry: &V2Entry, chunk_index: u32) -> Vec<u8> {
    let mut aad = Vec::with_capacity(72 + entry.path.len());
    aad.extend_from_slice(&(entry.path.len() as u16).to_le_bytes());
    aad.extend_from_slice(entry.path.as_bytes());
    aad.extend_from_slice(&entry.size.to_le_bytes());
    aad.extend_from_slice(&entry.modified.to_le_bytes());
    aad.extend_from_slice(&entry.mode.to_le_bytes());
    aad.extend_from_slice(&entry.digest);
    aad.extend_from_slice(&entry.index.to_le_bytes());
    aad.extend_from_slice(&chunk_index.to_le_bytes());
    aad.push(u8::from(entry.directory));
    aad.push(u8::from(entry.encrypted));
    aad
}

fn chunk_nonce(base: &[u8; 24], entry_index: u32, chunk_index: u32) -> XNonce {
    let mut nonce = *base;
    for (offset, byte) in entry_index.to_le_bytes().into_iter().enumerate() {
        nonce[offset] ^= byte;
    }
    for (offset, byte) in chunk_index.to_le_bytes().into_iter().enumerate() {
        nonce[4 + offset] ^= byte;
    }
    *XNonce::from_slice(&nonce)
}

fn derive_cipher(password: &[u8], salt: &[u8; 16]) -> Result<XChaCha20Poly1305, ArchiveError> {
    let params = Params::new(ARGON2_MEMORY_KIB, ARGON2_ITERATIONS, ARGON2_LANES, Some(32))
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into(password, salt, key.as_mut())
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    Ok(XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(
        key.as_ref(),
    )))
}

fn collect_entries(sources: &[PathBuf], archive: &Path) -> Result<Vec<PlannedEntry>, ArchiveError> {
    let mut planned = Vec::new();
    let mut paths = HashSet::new();
    for source in sources {
        if !source.exists() {
            return Err(ArchiveError::MissingSource(source.to_owned()));
        }
        if source.is_dir() && archive.starts_with(source) {
            return Err(ArchiveError::ArchiveInsideSource);
        }
        let base = source.parent().unwrap_or(source);
        for item in WalkDir::new(source).follow_links(false) {
            let item = item?;
            if item.file_type().is_symlink()
                || (!item.file_type().is_dir() && !item.file_type().is_file())
            {
                continue;
            }
            let relative = item
                .path()
                .strip_prefix(base)?
                .to_string_lossy()
                .replace('\\', "/");
            validate_member_path(&relative)?;
            if !paths.insert(path_key(&relative)) {
                return Err(ArchiveError::Message(format!(
                    "Percorso duplicato: {relative}"
                )));
            }
            let metadata = item.metadata()?;
            planned.push(PlannedEntry {
                source: item.path().to_owned(),
                path: relative,
                directory: metadata.is_dir(),
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                modified: modification_time(&metadata),
                mode: permissions(&metadata),
            });
        }
    }
    if planned.is_empty() {
        return Err(ArchiveError::Message(
            "Nessun file o directory regolare da archiviare".into(),
        ));
    }
    Ok(planned)
}

fn preflight_destinations(destination: &Path, entries: &[V2Entry]) -> Result<(), ArchiveError> {
    for entry in entries {
        let output = safe_destination(destination, &entry.path)?;
        if fs::symlink_metadata(&output).is_ok() {
            return Err(ArchiveError::DestinationExists(output));
        }
        if let Some(parent) = output.parent() {
            check_secure_parents(destination, parent)?;
        }
    }
    Ok(())
}

fn check_secure_parents(base: &Path, parent: &Path) -> Result<(), ArchiveError> {
    let relative = parent.strip_prefix(base)?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(ArchiveError::UnsafePath(current.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn create_secure_directories(base: &Path, directory: &Path) -> Result<(), ArchiveError> {
    check_secure_parents(base, directory)?;
    let relative = directory.strip_prefix(base)?;
    let mut current = base.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(ArchiveError::UnsafePath(current.display().to_string()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&current)?,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn apply_metadata(path: &Path, entry: &V2Entry) -> Result<(), ArchiveError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(entry.mode & 0o777))?;
    }
    #[cfg(windows)]
    if !entry.directory {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(entry.mode & 0o200 == 0);
        fs::set_permissions(path, permissions)?;
    }
    filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(entry.modified, 0))?;
    Ok(())
}

fn modification_time(metadata: &fs::Metadata) -> i64 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        metadata.mtime()
    }
    #[cfg(not(unix))]
    {
        metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|value| value.as_secs() as i64)
            .unwrap_or(0)
    }
}

fn permissions(metadata: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        if metadata.permissions().readonly() {
            0o444
        } else {
            0o644
        }
    }
}

fn path_key(path: &str) -> String {
    #[cfg(windows)]
    {
        path.to_lowercase()
    }
    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

fn volume_path(first: &Path, index: u32) -> PathBuf {
    if index == 1 {
        first.to_owned()
    } else {
        first.with_extension(format!("w{index:02}"))
    }
}

fn chunks_for(compressed_size: u64) -> u64 {
    if compressed_size == 0 {
        0
    } else {
        (compressed_size - 1) / CHUNK_SIZE + 1
    }
}

fn digest_file(path: &Path) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut file)?;
    Ok(hasher.finalize().into())
}

struct HashReader<R: Read> {
    inner: R,
    hasher: Hasher,
}

impl<R: Read> HashReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
        }
    }

    fn finish(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}

impl<R: Read> Read for HashReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.hasher.update(&buffer[..read]);
        Ok(read)
    }
}

struct HashWriter {
    inner: File,
    hasher: Hasher,
    written: u64,
    limit: u64,
}

impl HashWriter {
    fn new(inner: File, limit: u64) -> Self {
        Self {
            inner,
            hasher: Hasher::new(),
            written: 0,
            limit,
        }
    }

    fn finish(mut self) -> std::io::Result<(u64, [u8; 32])> {
        self.inner.flush()?;
        Ok((self.written, self.hasher.finalize().into()))
    }
}

impl Write for HashWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if buffer.len() as u64 > self.limit.saturating_sub(self.written) {
            return Err(io_error("Contenuto oltre la dimensione dichiarata"));
        }
        let written = self.inner.write(buffer)?;
        self.hasher.update(&buffer[..written]);
        self.written += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn read_u16<R: Read>(reader: &mut R) -> Result<u16, ArchiveError> {
    let mut bytes = [0u8; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, ArchiveError> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32<R: Read>(reader: &mut R) -> Result<i32, ArchiveError> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u64<R: Read>(reader: &mut R) -> Result<u64, ArchiveError> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64<R: Read>(reader: &mut R) -> Result<i64, ArchiveError> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(i64::from_le_bytes(bytes))
}

fn io_error(message: impl Into<String>) -> std::io::Error {
    std::io::Error::other(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(password: Option<&str>) -> CreateOptions {
        CreateOptions {
            level: 5,
            password: password.map(|value| Zeroizing::new(value.into())),
            solid: true,
            comment: Some("Volume v2 日本語".into()),
            volume_size: None,
            recovery_percent: 0,
            deduplicate: false,
        }
    }

    fn assert_no_files(root: &Path) {
        assert_eq!(
            WalkDir::new(root)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file())
                .count(),
            0
        );
    }

    #[test]
    fn streaming_roundtrip_single_and_multivolume() {
        for volume_size in [None, Some(MIN_VOLUME_SIZE)] {
            for password in [None, Some("test-password")] {
                let temp = tempfile::tempdir().unwrap();
                let source = temp.path().join("dati");
                fs::create_dir(&source).unwrap();
                let mut content = vec![0u8; 3 * 1024 * 1024];
                getrandom::fill(&mut content).unwrap();
                fs::write(source.join("documento àèé.bin"), &content).unwrap();
                let archive = temp.path().join("prova.wolrar");
                let info = create(&archive, &[source], &options(password), volume_size).unwrap();
                assert_eq!(info.volume_count > 1, volume_size.is_some());
                assert_eq!(inspect(&archive).unwrap().comment, "Volume v2 日本語");
                let destination = temp.path().join("estratto");
                extract(&archive, &destination, password).unwrap();
                assert_eq!(
                    fs::read(destination.join("dati/documento àèé.bin")).unwrap(),
                    content
                );
            }
        }
    }

    #[test]
    fn rejects_wrong_password_and_missing_volume_without_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("grande.bin");
        let mut content = vec![0u8; 2 * 1024 * 1024];
        getrandom::fill(&mut content).unwrap();
        fs::write(&source, content).unwrap();
        let archive = temp.path().join("cifrato.wolrar");
        create(
            &archive,
            std::slice::from_ref(&source),
            &options(Some("giusta")),
            Some(MIN_VOLUME_SIZE),
        )
        .unwrap();
        let wrong = temp.path().join("errato");
        assert!(extract(&archive, &wrong, Some("sbagliata")).is_err());
        assert_no_files(&wrong);
        fs::rename(
            volume_path(&archive, 2),
            temp.path().join("volume-mancante"),
        )
        .unwrap();
        let missing = temp.path().join("mancante");
        assert!(extract(&archive, &missing, Some("giusta")).is_err());
        assert_no_files(&missing);
    }

    #[test]
    fn detects_volume_tampering() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        let mut content = vec![0u8; 2 * 1024 * 1024];
        getrandom::fill(&mut content).unwrap();
        fs::write(&source, content).unwrap();
        let archive = temp.path().join("alterato.wolrar");
        create(
            &archive,
            std::slice::from_ref(&source),
            &options(None),
            Some(MIN_VOLUME_SIZE),
        )
        .unwrap();
        let second = volume_path(&archive, 2);
        let mut file = fs::OpenOptions::new().write(true).open(second).unwrap();
        file.seek(SeekFrom::Start(VOLUME_HEADER_LEN + 20)).unwrap();
        file.write_all(&[0xff]).unwrap();
        assert!(list(&archive).is_err());
    }

    #[test]
    fn deduplicates_identical_content_and_restores_it() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("dati");
        fs::create_dir(&source).unwrap();
        let mut shared = vec![0u8; 1024 * 1024];
        getrandom::fill(&mut shared).unwrap();
        fs::write(source.join("primo.bin"), &shared).unwrap();
        fs::write(source.join("secondo.bin"), &shared).unwrap();
        fs::write(source.join("terzo.bin"), b"contenuto diverso").unwrap();
        let archive = temp.path().join("dedup.wolrar");
        let mut options = options(None);
        options.deduplicate = true;
        create(&archive, &[source], &options, None).unwrap();
        let entries = read_entries(&archive).unwrap().1;
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.duplicate_of.is_some())
                .count(),
            1
        );
        let destination = temp.path().join("estratto");
        extract(&archive, &destination, None).unwrap();
        assert_eq!(
            fs::read(destination.join("dati/primo.bin")).unwrap(),
            shared
        );
        assert_eq!(
            fs::read(destination.join("dati/secondo.bin")).unwrap(),
            shared
        );
        assert_eq!(
            fs::read_to_string(destination.join("dati/terzo.bin")).unwrap(),
            "contenuto diverso"
        );
    }
}

use std::collections::HashSet;
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

const MAGIC: &[u8; 8] = b"WOLRAR01";
const VERSION: u16 = 1;
const HEADER_LEN: u64 = 108;
const BODY_LEN_OFFSET: u64 = 68;
const DIGEST_OFFSET: u64 = 76;
const ENTRY_PACKED_SIZE_OFFSET: u64 = 12;
const ENTRY_FIXED_LEN: u64 = 32;
const MAX_ENCRYPTED_FILE_SIZE: u64 = 512 * 1024 * 1024;
const MAX_COMMENT_SIZE: usize = 16 * 1024 * 1024;
const ARGON2_MEMORY_KIB: u32 = 32 * 1024;
const ARGON2_ITERATIONS: u32 = 3;
const ARGON2_LANES: u32 = 1;

#[derive(Debug, Clone)]
pub struct WolArchiveInfo {
    pub comment: String,
    pub entry_count: u64,
    pub encrypted: bool,
    pub body_len: u64,
}

#[derive(Debug, Clone)]
struct WolEntry {
    path: String,
    directory: bool,
    encrypted: bool,
    size: u64,
    packed_size: u64,
    modified: i64,
    mode: u32,
    digest: [u8; 32],
    payload_offset: u64,
    index: u32,
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

struct CreateGuard {
    path: PathBuf,
    keep: bool,
}

impl Drop for CreateGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn create(
    path: &Path,
    sources: &[PathBuf],
    options: &CreateOptions,
) -> Result<WolArchiveInfo, ArchiveError> {
    if path.exists() {
        return Err(ArchiveError::DestinationExists(path.to_owned()));
    }
    if sources.is_empty() {
        return Err(ArchiveError::MissingSource(path.to_owned()));
    }
    let comment = options.comment.clone().unwrap_or_default();
    if comment.len() > MAX_COMMENT_SIZE {
        return Err(ArchiveError::Message("Commento troppo grande".into()));
    }
    let comment_len = u32::try_from(comment.len())
        .map_err(|_| ArchiveError::Message("Commento troppo grande".into()))?;
    let planned = collect_entries(sources, path)?;
    let entry_count = planned.len() as u64;
    if entry_count > u32::MAX as u64 {
        return Err(ArchiveError::Message("Troppe entry per .wolrar v1".into()));
    }

    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom::fill(&mut salt).map_err(|error| ArchiveError::Message(error.to_string()))?;
    getrandom::fill(&mut nonce).map_err(|error| ArchiveError::Message(error.to_string()))?;
    let encrypted = options.password.is_some();
    let cipher = options
        .password
        .as_ref()
        .map(|password| derive_cipher(password.as_bytes(), &salt))
        .transpose()?;

    let mut writer = File::create_new(path)?;
    let mut guard = CreateGuard {
        path: path.to_owned(),
        keep: false,
    };
    write_header(
        &mut writer,
        options.level,
        encrypted,
        comment_len,
        &salt,
        &nonce,
        entry_count,
        0,
        &[0u8; 32],
    )?;
    writer.write_all(comment.as_bytes())?;

    for (index, item) in planned.iter().enumerate() {
        let mut entry = WolEntry {
            path: item.path.clone(),
            directory: item.directory,
            encrypted,
            size: item.size,
            packed_size: 0,
            modified: item.modified,
            mode: item.mode,
            digest: [0u8; 32],
            payload_offset: 0,
            index: index as u32,
        };
        if entry.directory {
            write_entry_metadata(&mut writer, &entry)?;
            continue;
        }
        if encrypted && entry.size > MAX_ENCRYPTED_FILE_SIZE {
            return Err(ArchiveError::Message(format!(
                "File cifrato oltre il limite: {}",
                entry.path
            )));
        }

        let mut input = HashReader::new(File::open(&item.source)?);
        if let Some(cipher) = &cipher {
            let compressed =
                Zeroizing::new(zstd::stream::encode_all(&mut input, options.level as i32)?);
            entry.digest = input.finish();
            let ciphertext = encrypt_payload(cipher, &nonce, &entry, &compressed)?;
            entry.packed_size = ciphertext.len() as u64;
            write_entry_metadata(&mut writer, &entry)?;
            writer.write_all(&ciphertext)?;
        } else {
            let metadata_offset = writer.stream_position()?;
            write_entry_metadata(&mut writer, &entry)?;
            entry.payload_offset = writer.stream_position()?;
            zstd::stream::copy_encode(&mut input, &mut writer, options.level as i32)?;
            entry.packed_size = writer.stream_position()? - entry.payload_offset;
            entry.digest = input.finish();
            let end = writer.stream_position()?;
            writer.seek(SeekFrom::Start(metadata_offset + ENTRY_PACKED_SIZE_OFFSET))?;
            writer.write_all(&entry.packed_size.to_le_bytes())?;
            writer.seek(SeekFrom::Start(
                metadata_offset + ENTRY_FIXED_LEN + entry.path.len() as u64,
            ))?;
            writer.write_all(&entry.digest)?;
            writer.seek(SeekFrom::Start(end))?;
        }
    }

    let body_len = writer.stream_position()? - HEADER_LEN;
    writer.seek(SeekFrom::Start(BODY_LEN_OFFSET))?;
    writer.write_all(&body_len.to_le_bytes())?;
    let digest = archive_digest(path, body_len)?;
    writer.seek(SeekFrom::Start(DIGEST_OFFSET))?;
    writer.write_all(&digest)?;
    writer.sync_all()?;
    guard.keep = true;
    Ok(WolArchiveInfo {
        comment,
        entry_count,
        encrypted,
        body_len,
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

pub fn inspect(path: &Path) -> Result<WolArchiveInfo, ArchiveError> {
    let file = File::open(path)?;
    let total = file.metadata()?.len();
    let (magic, version, flags, _, comment_len, _, _, entry_count, body_len, _) =
        read_header(file)?;
    if magic != *MAGIC {
        return Err(ArchiveError::UnsupportedFormat(
            "Magic .wolrar non valido".into(),
        ));
    }
    if version != VERSION {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Versione .wolrar {version}"
        )));
    }
    if flags & !1 != 0 {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Flags .wolrar non supportate: {flags}"
        )));
    }
    if entry_count > u32::MAX as u64 {
        return Err(ArchiveError::UnsupportedFormat(
            "Troppe entry per .wolrar v1".into(),
        ));
    }
    if comment_len as usize > MAX_COMMENT_SIZE || comment_len as u64 > body_len {
        return Err(ArchiveError::Message(
            "Lunghezza commento .wolrar non valida".into(),
        ));
    }
    let expected_total = HEADER_LEN
        .checked_add(body_len)
        .ok_or_else(|| ArchiveError::Message("Lunghezza .wolrar non valida".into()))?;
    if total != expected_total {
        return Err(ArchiveError::Message(
            "Lunghezza .wolrar non coerente".into(),
        ));
    }
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(HEADER_LEN))?;
    let mut comment = vec![0u8; comment_len as usize];
    file.read_exact(&mut comment)?;
    let comment =
        String::from_utf8(comment).map_err(|error| ArchiveError::Message(error.to_string()))?;
    Ok(WolArchiveInfo {
        comment,
        entry_count,
        encrypted: flags & 1 != 0,
        body_len,
    })
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
    let (salt, nonce) = read_crypto_header(path)?;
    let cipher = password
        .map(|value| derive_cipher(value.as_bytes(), &salt))
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
        decode_entry(path, entry, &output, cipher.as_ref(), &nonce)?;
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
            let output = safe_destination(destination, &entry.path)?;
            apply_metadata(&output, entry)?;
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
            if item.file_type().is_symlink() {
                continue;
            }
            if !item.file_type().is_dir() && !item.file_type().is_file() {
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
                    "Percorso duplicato nell'archivio: {relative}"
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

fn read_entries(path: &Path) -> Result<(WolArchiveInfo, Vec<WolEntry>), ArchiveError> {
    let info = inspect(path)?;
    verify_archive_digest(path, info.body_len)?;
    let body_end = HEADER_LEN + info.body_len;
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(HEADER_LEN + info.comment.len() as u64))?;
    let mut entries = Vec::new();
    let mut paths = HashSet::new();
    for index in 0..info.entry_count {
        let path_len = read_u16(&mut file)?;
        let flags = read_u16(&mut file)?;
        if flags & !3 != 0 {
            return Err(ArchiveError::UnsupportedFormat(format!(
                "Flags entry non supportate: {flags}"
            )));
        }
        let size = read_u64(&mut file)?;
        let packed_size = read_u64(&mut file)?;
        let modified = read_i64(&mut file)?;
        let mode = read_u32(&mut file)?;
        let mut path_bytes = vec![0u8; path_len as usize];
        file.read_exact(&mut path_bytes)?;
        let path = String::from_utf8(path_bytes)
            .map_err(|error| ArchiveError::Message(error.to_string()))?;
        validate_member_path(&path)?;
        if !paths.insert(path_key(&path)) {
            return Err(ArchiveError::Message(format!(
                "Percorso duplicato nell'archivio: {path}"
            )));
        }
        let mut digest = [0u8; 32];
        file.read_exact(&mut digest)?;
        let payload_offset = file.stream_position()?;
        let directory = flags & 1 != 0;
        let encrypted = flags & 2 != 0;
        if encrypted != info.encrypted {
            return Err(ArchiveError::Message(
                "Flags di cifratura non coerenti".into(),
            ));
        }
        if directory && (size != 0 || packed_size != 0 || digest != [0u8; 32]) {
            return Err(ArchiveError::Message(
                "Metadati directory non validi".into(),
            ));
        }
        if !directory && packed_size == 0 {
            return Err(ArchiveError::Message(
                "Payload file vuoto non valido".into(),
            ));
        }
        if !directory && encrypted && (size > MAX_ENCRYPTED_FILE_SIZE || packed_size < 16) {
            return Err(ArchiveError::Message(
                "Dimensione file cifrato non valida".into(),
            ));
        }
        let remaining = body_end
            .checked_sub(payload_offset)
            .ok_or_else(|| ArchiveError::Message("Payload oltre la fine dell'archivio".into()))?;
        if packed_size > remaining {
            return Err(ArchiveError::Message(
                "Payload oltre la fine dell'archivio".into(),
            ));
        }
        let next = payload_offset
            .checked_add(packed_size)
            .ok_or_else(|| ArchiveError::Message("Offset payload non valido".into()))?;
        file.seek(SeekFrom::Start(next))?;
        entries.push(WolEntry {
            path,
            directory,
            encrypted,
            size,
            packed_size,
            modified,
            mode,
            digest,
            payload_offset,
            index: index as u32,
        });
    }
    if file.stream_position()? != body_end {
        return Err(ArchiveError::Message(
            "Dati entry non coerenti con il numero dichiarato".into(),
        ));
    }
    Ok((info, entries))
}

#[allow(clippy::too_many_arguments)]
fn write_header(
    writer: &mut File,
    level: u32,
    encrypted: bool,
    comment_len: u32,
    salt: &[u8; 16],
    nonce: &[u8; 24],
    entry_count: u64,
    body_len: u64,
    digest: &[u8; 32],
) -> Result<(), ArchiveError> {
    writer.write_all(MAGIC)?;
    writer.write_all(&VERSION.to_le_bytes())?;
    writer.write_all(&(u16::from(encrypted)).to_le_bytes())?;
    writer.write_all(&(level as i32).to_le_bytes())?;
    writer.write_all(&comment_len.to_le_bytes())?;
    writer.write_all(salt)?;
    writer.write_all(nonce)?;
    writer.write_all(&entry_count.to_le_bytes())?;
    writer.write_all(&body_len.to_le_bytes())?;
    writer.write_all(digest)?;
    Ok(())
}

fn write_entry_metadata(writer: &mut File, entry: &WolEntry) -> Result<(), ArchiveError> {
    let path_bytes = entry.path.as_bytes();
    let path_len = u16::try_from(path_bytes.len())
        .map_err(|_| ArchiveError::UnsafePath(entry.path.clone()))?;
    writer.write_all(&path_len.to_le_bytes())?;
    let flags = u16::from(entry.directory) | (u16::from(entry.encrypted) << 1);
    writer.write_all(&flags.to_le_bytes())?;
    writer.write_all(&entry.size.to_le_bytes())?;
    writer.write_all(&entry.packed_size.to_le_bytes())?;
    writer.write_all(&entry.modified.to_le_bytes())?;
    writer.write_all(&entry.mode.to_le_bytes())?;
    writer.write_all(path_bytes)?;
    writer.write_all(&entry.digest)?;
    Ok(())
}

#[allow(clippy::type_complexity)]
fn read_header(
    mut file: File,
) -> Result<
    (
        [u8; 8],
        u16,
        u16,
        i32,
        u32,
        [u8; 16],
        [u8; 24],
        u64,
        u64,
        [u8; 32],
    ),
    ArchiveError,
> {
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    let version = read_u16(&mut file)?;
    let flags = read_u16(&mut file)?;
    let level = read_i32(&mut file)?;
    let comment_len = read_u32(&mut file)?;
    let mut salt = [0u8; 16];
    file.read_exact(&mut salt)?;
    let mut nonce = [0u8; 24];
    file.read_exact(&mut nonce)?;
    let entry_count = read_u64(&mut file)?;
    let body_len = read_u64(&mut file)?;
    let mut digest = [0u8; 32];
    file.read_exact(&mut digest)?;
    Ok((
        magic,
        version,
        flags,
        level,
        comment_len,
        salt,
        nonce,
        entry_count,
        body_len,
        digest,
    ))
}

fn read_crypto_header(path: &Path) -> Result<([u8; 16], [u8; 24]), ArchiveError> {
    let (_, _, _, _, _, salt, nonce, _, _, _) = read_header(File::open(path)?)?;
    Ok((salt, nonce))
}

fn archive_digest(path: &Path, body_len: u64) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut (&mut file).take(DIGEST_OFFSET))?;
    file.seek(SeekFrom::Start(HEADER_LEN))?;
    hasher.update_reader(&mut file.take(body_len))?;
    Ok(hasher.finalize().into())
}

fn verify_archive_digest(path: &Path, body_len: u64) -> Result<(), ArchiveError> {
    let expected = read_header(File::open(path)?)?.9;
    let actual = archive_digest(path, body_len)?;
    if actual != expected {
        return Err(ArchiveError::Message(
            "Digest dell'archivio .wolrar non valido".into(),
        ));
    }
    Ok(())
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

fn encrypt_payload(
    cipher: &XChaCha20Poly1305,
    base_nonce: &[u8; 24],
    entry: &WolEntry,
    plaintext: &[u8],
) -> Result<Vec<u8>, ArchiveError> {
    let nonce = entry_nonce(base_nonce, entry.index);
    let aad = entry_aad(entry);
    cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|error| ArchiveError::Message(error.to_string()))
}

fn decrypt_payload(
    cipher: &XChaCha20Poly1305,
    base_nonce: &[u8; 24],
    entry: &WolEntry,
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, ArchiveError> {
    let nonce = entry_nonce(base_nonce, entry.index);
    let aad = entry_aad(entry);
    let compressed = Zeroizing::new(
        cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| {
                ArchiveError::Message(format!("Autenticazione non valida per {}", entry.path))
            })?,
    );
    let decoder = zstd::stream::read::Decoder::new(compressed.as_slice())?;
    let mut plaintext = Zeroizing::new(Vec::with_capacity(entry.size.min(1024 * 1024) as usize));
    decoder.take(entry.size + 1).read_to_end(&mut plaintext)?;
    Ok(plaintext)
}

fn entry_aad(entry: &WolEntry) -> Vec<u8> {
    let mut aad = Vec::with_capacity(64 + entry.path.len());
    aad.extend_from_slice(&(entry.path.len() as u16).to_le_bytes());
    aad.extend_from_slice(entry.path.as_bytes());
    aad.extend_from_slice(&entry.size.to_le_bytes());
    aad.extend_from_slice(&entry.modified.to_le_bytes());
    aad.extend_from_slice(&entry.mode.to_le_bytes());
    aad.extend_from_slice(&entry.digest);
    aad.extend_from_slice(&entry.index.to_le_bytes());
    aad.push(u8::from(entry.directory));
    aad.push(u8::from(entry.encrypted));
    aad
}

fn entry_nonce(base: &[u8; 24], index: u32) -> XNonce {
    let mut nonce = *base;
    for (offset, byte) in index.to_le_bytes().into_iter().enumerate() {
        nonce[offset] ^= byte;
    }
    *XNonce::from_slice(&nonce)
}

fn decode_entry(
    path: &Path,
    entry: &WolEntry,
    output: &Path,
    cipher: Option<&XChaCha20Poly1305>,
    nonce: &[u8; 24],
) -> Result<(), ArchiveError> {
    let target = File::options().write(true).create_new(true).open(output)?;
    let mut target = HashWriter::new(target, entry.size);
    if entry.encrypted {
        let cipher =
            cipher.ok_or_else(|| ArchiveError::Message("Password richiesta per .wolrar".into()))?;
        let ciphertext_len = usize::try_from(entry.packed_size)
            .map_err(|_| ArchiveError::Message("Payload cifrato troppo grande".into()))?;
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(entry.payload_offset))?;
        let mut ciphertext = vec![0u8; ciphertext_len];
        file.read_exact(&mut ciphertext)?;
        let plaintext = decrypt_payload(cipher, nonce, entry, &ciphertext)?;
        target.write_all(&plaintext)?;
    } else {
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(entry.payload_offset))?;
        zstd::stream::copy_decode(file.take(entry.packed_size), &mut target)?;
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

fn preflight_destinations(destination: &Path, entries: &[WolEntry]) -> Result<(), ArchiveError> {
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

fn apply_metadata(path: &Path, entry: &WolEntry) -> Result<(), ArchiveError> {
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
        return metadata.mtime();
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
        return metadata.permissions().mode();
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
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Contenuto oltre la dimensione dichiarata",
            ));
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

fn read_u16(file: &mut File) -> Result<u16, ArchiveError> {
    let mut bytes = [0u8; 2];
    file.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(file: &mut File) -> Result<u32, ArchiveError> {
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32(file: &mut File) -> Result<i32, ArchiveError> {
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes)?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_u64(file: &mut File) -> Result<u64, ArchiveError> {
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_i64(file: &mut File) -> Result<i64, ArchiveError> {
    let mut bytes = [0u8; 8];
    file.read_exact(&mut bytes)?;
    Ok(i64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(password: Option<&str>) -> CreateOptions {
        CreateOptions {
            level: 5,
            password: password.map(|value| Zeroizing::new(value.into())),
            solid: false,
            comment: Some("Commento aperto àèé 日本語".into()),
        }
    }

    fn assert_no_files(root: &Path) {
        let files = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .count();
        assert_eq!(
            files, 0,
            "Sono stati lasciati file dopo un'estrazione fallita"
        );
    }

    fn rewrite_archive_digest(path: &Path) {
        let body_len = read_header(File::open(path).unwrap()).unwrap().8;
        let digest = archive_digest(path, body_len).unwrap();
        let mut file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.seek(SeekFrom::Start(DIGEST_OFFSET)).unwrap();
        file.write_all(&digest).unwrap();
    }

    #[test]
    fn plain_and_encrypted_roundtrip() {
        for password in [None, Some("password-test")] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("dati");
            fs::create_dir(&source).unwrap();
            fs::write(
                source.join("documento àèé.txt"),
                vec![b'x'; 2 * 1024 * 1024],
            )
            .unwrap();
            let archive = temp.path().join("prova.wolrar");
            create(&archive, std::slice::from_ref(&source), &options(password)).unwrap();
            assert_eq!(
                inspect(&archive).unwrap().comment,
                "Commento aperto àèé 日本語"
            );
            assert!(
                list(&archive)
                    .unwrap()
                    .iter()
                    .any(|entry| entry.path.ends_with("documento àèé.txt"))
            );
            let destination = temp.path().join("estratto");
            extract(&archive, &destination, password).unwrap();
            assert_eq!(
                fs::read(destination.join("dati/documento àèé.txt")).unwrap(),
                vec![b'x'; 2 * 1024 * 1024]
            );
        }
    }

    #[test]
    fn large_plain_file_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("grande.bin");
        let content: Vec<u8> = (0..16 * 1024 * 1024)
            .map(|index| (index % 251) as u8)
            .collect();
        fs::write(&source, &content).unwrap();
        let archive = temp.path().join("grande.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let destination = temp.path().join("estratto");
        extract(&archive, &destination, None).unwrap();
        assert_eq!(fs::read(destination.join("grande.bin")).unwrap(), content);
    }

    #[test]
    fn rejects_wrong_password_without_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("segreto.txt");
        fs::write(&source, "contenuto segreto").unwrap();
        let archive = temp.path().join("cifrato.wolrar");
        create(&archive, &[source], &options(Some("giusta"))).unwrap();
        let destination = temp.path().join("errata");
        assert!(extract(&archive, &destination, Some("sbagliata")).is_err());
        assert_no_files(&destination);
    }

    #[test]
    fn rejects_payload_tampering_without_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        fs::write(&source, "contenuto originale".repeat(1000)).unwrap();
        let archive = temp.path().join("alterato.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let entry = read_entries(&archive)
            .unwrap()
            .1
            .into_iter()
            .find(|entry| !entry.directory)
            .unwrap();
        let mut file = fs::OpenOptions::new().write(true).open(&archive).unwrap();
        file.seek(SeekFrom::Start(
            entry.payload_offset + entry.packed_size / 2,
        ))
        .unwrap();
        file.write_all(&[0xff]).unwrap();
        drop(file);
        let destination = temp.path().join("alterato");
        assert!(extract(&archive, &destination, None).is_err());
        assert_no_files(&destination);
    }

    #[test]
    fn rejects_file_digest_tampering_without_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        fs::write(&source, "contenuto originale").unwrap();
        let archive = temp.path().join("digest.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let entry = read_entries(&archive)
            .unwrap()
            .1
            .into_iter()
            .find(|entry| !entry.directory)
            .unwrap();
        let mut file = fs::OpenOptions::new().write(true).open(&archive).unwrap();
        file.seek(SeekFrom::Start(entry.payload_offset - 32))
            .unwrap();
        file.write_all(&[0xff]).unwrap();
        drop(file);
        rewrite_archive_digest(&archive);
        let destination = temp.path().join("digest");
        assert!(extract(&archive, &destination, None).is_err());
        assert_no_files(&destination);
    }

    #[test]
    fn rejects_truncated_zstd_frame_without_output() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        fs::write(&source, "contenuto originale".repeat(100)).unwrap();
        let archive = temp.path().join("troncato.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let (info, entries) = read_entries(&archive).unwrap();
        let entry = entries.into_iter().find(|entry| !entry.directory).unwrap();
        let metadata_offset = entry.payload_offset - ENTRY_FIXED_LEN - entry.path.len() as u64;
        let mut file = fs::OpenOptions::new().write(true).open(&archive).unwrap();
        file.seek(SeekFrom::Start(metadata_offset + ENTRY_PACKED_SIZE_OFFSET))
            .unwrap();
        file.write_all(&(entry.packed_size - 1).to_le_bytes())
            .unwrap();
        file.seek(SeekFrom::Start(BODY_LEN_OFFSET)).unwrap();
        file.write_all(&(info.body_len - 1).to_le_bytes()).unwrap();
        file.set_len(HEADER_LEN + info.body_len - 1).unwrap();
        drop(file);
        rewrite_archive_digest(&archive);
        let destination = temp.path().join("troncato");
        assert!(extract(&archive, &destination, None).is_err());
        assert_no_files(&destination);
    }

    #[test]
    fn rejects_unknown_flags_and_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("aa.txt");
        fs::write(&source, "contenuto").unwrap();
        let archive = temp.path().join("flags.wolrar");
        create(&archive, std::slice::from_ref(&source), &options(None)).unwrap();
        let mut file = fs::OpenOptions::new().write(true).open(&archive).unwrap();
        file.seek(SeekFrom::Start(10)).unwrap();
        file.write_all(&0x8000u16.to_le_bytes()).unwrap();
        assert!(inspect(&archive).is_err());

        let archive = temp.path().join("traversal.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let entry = read_entries(&archive).unwrap().1.remove(0);
        let path_offset = entry.payload_offset - 32 - entry.path.len() as u64;
        let mut file = fs::OpenOptions::new().write(true).open(&archive).unwrap();
        file.seek(SeekFrom::Start(path_offset)).unwrap();
        file.write_all(b"../x/y").unwrap();
        drop(file);
        rewrite_archive_digest(&archive);
        let destination = temp.path().join("traversal");
        assert!(extract(&archive, &destination, None).is_err());
        assert!(!temp.path().join("x/y").exists());
        assert_no_files(&destination);
    }

    #[test]
    fn refuses_overwrite_and_removes_failed_creation() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        fs::write(&source, "contenuto").unwrap();
        let archive = temp.path().join("prova.wolrar");
        create(&archive, std::slice::from_ref(&source), &options(None)).unwrap();
        let destination = temp.path().join("estratto");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("documento.txt"), "non toccare").unwrap();
        assert!(extract(&archive, &destination, None).is_err());
        assert_eq!(
            fs::read_to_string(destination.join("documento.txt")).unwrap(),
            "non toccare"
        );

        let duplicate = temp.path().join("documento.txt");
        let incomplete = temp.path().join("incompleto.wolrar");
        assert!(create(&incomplete, &[source, duplicate], &options(None)).is_err());
        assert!(!incomplete.exists());
    }

    #[cfg(unix)]
    #[test]
    fn restores_unix_metadata() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("eseguibile");
        fs::write(&source, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o751)).unwrap();
        filetime::set_file_mtime(
            &source,
            filetime::FileTime::from_unix_time(1_600_000_000, 0),
        )
        .unwrap();
        let archive = temp.path().join("metadati.wolrar");
        create(&archive, &[source], &options(None)).unwrap();
        let destination = temp.path().join("estratto");
        extract(&archive, &destination, None).unwrap();
        let output = destination.join("eseguibile");
        assert_eq!(
            fs::metadata(&output).unwrap().permissions().mode() & 0o777,
            0o751
        );
        assert_eq!(
            filetime::FileTime::from_last_modification_time(&fs::metadata(&output).unwrap())
                .unix_seconds(),
            1_600_000_000
        );
    }
}

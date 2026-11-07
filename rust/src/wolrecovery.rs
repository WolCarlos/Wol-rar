use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use blake3::Hasher;
use reed_solomon_erasure::galois_8::ReedSolomon;

use crate::ArchiveError;

const RECOVERY_MAGIC: &[u8; 8] = b"WOLREC02";
const VOLUME_MAGIC: &[u8; 8] = b"WOLVOL02";
const VERSION: u16 = 2;
const RECOVERY_HEADER_LEN: u64 = 80;
const RECOVERY_DIGEST_OFFSET: u64 = 48;
const VOLUME_HEADER_LEN: u64 = 84;
const VOLUME_DIGEST_OFFSET: u64 = 52;

#[derive(Debug, serde::Serialize)]
pub struct RepairReport {
    pub archive: PathBuf,
    pub destination: PathBuf,
    pub recovered_volumes: Vec<u32>,
    pub parity_count: u32,
    pub verified: bool,
}

#[derive(Debug)]
struct DataHeader {
    count: u32,
    archive_id: [u8; 16],
}

#[derive(Debug)]
struct RecoveryHeader {
    index: u32,
    data_count: u32,
    parity_count: u32,
    archive_id: [u8; 16],
    volume_size: u64,
}

pub fn create_recovery(
    archive: &Path,
    volume_size: u64,
    archive_id: [u8; 16],
    percent: u8,
) -> Result<u32, ArchiveError> {
    if percent == 0 {
        return Ok(0);
    }
    if percent > 100 {
        return Err(ArchiveError::Message("Recovery oltre 100%".into()));
    }
    let data_header = read_data_header(archive)?;
    if data_header.archive_id != archive_id {
        return Err(ArchiveError::Message(
            "Identità archivio recovery non coerente".into(),
        ));
    }
    let data_count = data_header.count as usize;
    let parity_count = (data_count * percent as usize).div_ceil(100);
    let parity_count = parity_count.max(1);
    let codec = ReedSolomon::new(data_count, parity_count)
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    let mut shards = Vec::with_capacity(data_count + parity_count);
    for index in 1..=data_header.count {
        shards.push(read_padded(&volume_path(archive, index), volume_size)?);
    }
    for _ in 0..parity_count {
        shards.push(vec![0u8; volume_size as usize]);
    }
    codec
        .encode(&mut shards)
        .map_err(|error| ArchiveError::Message(error.to_string()))?;

    let mut written = Vec::new();
    let result = (|| -> Result<u32, ArchiveError> {
        for parity_index in 1..=parity_count as u32 {
            let path = recovery_path(archive, parity_index);
            written.push(path.clone());
            let payload = &shards[data_count + parity_index as usize - 1];
            let mut file = File::create_new(&path)?;
            write_recovery_header(
                &mut file,
                parity_index,
                data_header.count,
                parity_count as u32,
                &archive_id,
                volume_size,
                &[0u8; 32],
            )?;
            file.write_all(payload)?;
            let digest = recovery_digest(&path, volume_size)?;
            file.seek(SeekFrom::Start(RECOVERY_DIGEST_OFFSET))?;
            file.write_all(&digest)?;
            file.sync_all()?;
        }
        Ok(parity_count as u32)
    })();
    if result.is_err() {
        for path in written {
            let _ = fs::remove_file(path);
        }
    }
    result
}

pub fn repair(archive: &Path, destination: &Path) -> Result<RepairReport, ArchiveError> {
    let parity = discover_recovery(archive)?;
    let first = read_recovery_header(&parity[0])?;
    let data_count = first.data_count;
    let parity_count = first.parity_count;
    let volume_size = first.volume_size;
    let codec = ReedSolomon::new(data_count as usize, parity_count as usize)
        .map_err(|error| ArchiveError::Message(error.to_string()))?;

    let mut shards = Vec::with_capacity((data_count + parity_count) as usize);
    let mut recovered = Vec::new();
    for index in 1..=data_count {
        let path = volume_path(archive, index);
        if let Some(bytes) = read_valid_data(&path, volume_size) {
            shards.push(Some(bytes));
        } else {
            shards.push(None);
            recovered.push(index);
        }
    }
    if recovered.is_empty() {
        return Err(ArchiveError::Message("Nessun volume da riparare".into()));
    }
    if recovered.len() > parity_count as usize {
        return Err(ArchiveError::Message(format!(
            "Volumi mancanti o corrotti: {}; parity disponibili: {parity_count}",
            recovered.len()
        )));
    }
    for (position, path) in parity.iter().enumerate() {
        let header = read_recovery_header(path)?;
        if header.index as usize != position + 1
            || header.data_count != data_count
            || header.parity_count != parity_count
            || header.archive_id != first.archive_id
            || header.volume_size != volume_size
        {
            return Err(ArchiveError::Message(
                "Manifest recovery non coerente".into(),
            ));
        }
        if let Some(bytes) = read_valid_recovery(path, volume_size) {
            shards.push(Some(bytes));
        } else {
            shards.push(None);
        }
    }
    codec
        .reconstruct(&mut shards)
        .map_err(|error| ArchiveError::Message(error.to_string()))?;

    if destination.exists() && !destination.is_dir() {
        return Err(ArchiveError::Message(
            "La destinazione non è una cartella".into(),
        ));
    }
    fs::create_dir_all(destination)?;
    let staging = tempfile::tempdir_in(destination)?;
    let first_name = archive
        .file_name()
        .ok_or_else(|| ArchiveError::MissingSource(archive.to_owned()))?;
    let staged_archive = staging.path().join(first_name);
    for index in 1..=data_count {
        let output = volume_path(&staged_archive, index);
        let mut file = File::create_new(output)?;
        let shard = shards[index as usize - 1]
            .as_ref()
            .expect("volume ricostruito");
        let payload_len = u64::from_le_bytes(shard[44..52].try_into().expect("header volume"));
        let volume_len = VOLUME_HEADER_LEN + payload_len;
        if volume_len > shard.len() as u64 {
            return Err(ArchiveError::Message(
                "Lunghezza volume ricostruito non valida".into(),
            ));
        }
        file.write_all(&shard[..volume_len as usize])?;
        file.sync_all()?;
    }
    crate::wolformat_v2::list(&staged_archive)?;

    let mut published = Vec::new();
    let result = (|| -> Result<(), ArchiveError> {
        for index in 1..=data_count {
            let staged = volume_path(&staged_archive, index);
            let output = volume_path(&destination.join(first_name), index);
            if let Err(error) = fs::hard_link(&staged, &output) {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(ArchiveError::DestinationExists(output));
                }
                return Err(error.into());
            }
            published.push(output);
        }
        Ok(())
    })();
    if result.is_err() {
        for output in published {
            let _ = fs::remove_file(output);
        }
    }
    result?;
    Ok(RepairReport {
        archive: archive.to_owned(),
        destination: destination.to_owned(),
        recovered_volumes: recovered,
        parity_count,
        verified: true,
    })
}

pub fn recovery_count(archive: &Path) -> u32 {
    let mut count = 0;
    while recovery_path(archive, count + 1).is_file() {
        count += 1;
    }
    count
}

fn discover_recovery(archive: &Path) -> Result<Vec<PathBuf>, ArchiveError> {
    let mut paths = Vec::new();
    while recovery_path(archive, paths.len() as u32 + 1).is_file() {
        paths.push(recovery_path(archive, paths.len() as u32 + 1));
    }
    if paths.is_empty() {
        return Err(ArchiveError::Message(
            "Nessun volume recovery .r01 trovato".into(),
        ));
    }
    Ok(paths)
}

fn read_valid_data(path: &Path, volume_size: u64) -> Option<Vec<u8>> {
    let header = read_data_header(path).ok()?;
    let digest = volume_digest(path).ok()?;
    let stored = read_digest(path, VOLUME_DIGEST_OFFSET).ok()?;
    if digest != stored {
        return None;
    }
    let _ = header;
    read_padded(path, volume_size).ok()
}

fn read_valid_recovery(path: &Path, volume_size: u64) -> Option<Vec<u8>> {
    let digest = recovery_digest(path, volume_size).ok()?;
    let stored = read_digest(path, RECOVERY_DIGEST_OFFSET).ok()?;
    if digest != stored {
        return None;
    }
    let mut file = File::open(path).ok()?;
    file.seek(SeekFrom::Start(RECOVERY_HEADER_LEN)).ok()?;
    let mut bytes = vec![0u8; volume_size as usize];
    file.read_exact(&mut bytes).ok()?;
    Some(bytes)
}

fn read_padded(path: &Path, volume_size: u64) -> Result<Vec<u8>, ArchiveError> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    if length > volume_size {
        return Err(ArchiveError::Message(
            "Volume oltre la dimensione recovery".into(),
        ));
    }
    let mut bytes = vec![0u8; volume_size as usize];
    file.read_exact(&mut bytes[..length as usize])?;
    Ok(bytes)
}

fn read_data_header(path: &Path) -> Result<DataHeader, ArchiveError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != VOLUME_MAGIC {
        return Err(ArchiveError::UnsupportedFormat(
            "Volume .wolrar non valido".into(),
        ));
    }
    let mut version = [0u8; 2];
    file.read_exact(&mut version)?;
    if u16::from_le_bytes(version) != VERSION {
        return Err(ArchiveError::UnsupportedFormat(
            "Versione volume non supportata".into(),
        ));
    }
    let mut flags = [0u8; 2];
    file.read_exact(&mut flags)?;
    if flags != [0u8; 2] {
        return Err(ArchiveError::UnsupportedFormat(
            "Flags volume non supportate".into(),
        ));
    }
    let mut index = [0u8; 4];
    file.read_exact(&mut index)?;
    let mut count = [0u8; 4];
    file.read_exact(&mut count)?;
    let mut archive_id = [0u8; 16];
    file.read_exact(&mut archive_id)?;
    Ok(DataHeader {
        count: u32::from_le_bytes(count),
        archive_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn write_recovery_header(
    file: &mut File,
    index: u32,
    data_count: u32,
    parity_count: u32,
    archive_id: &[u8; 16],
    volume_size: u64,
    digest: &[u8; 32],
) -> Result<(), ArchiveError> {
    file.write_all(RECOVERY_MAGIC)?;
    file.write_all(&VERSION.to_le_bytes())?;
    file.write_all(&0u16.to_le_bytes())?;
    file.write_all(&index.to_le_bytes())?;
    file.write_all(&data_count.to_le_bytes())?;
    file.write_all(&parity_count.to_le_bytes())?;
    file.write_all(archive_id)?;
    file.write_all(&volume_size.to_le_bytes())?;
    file.write_all(digest)?;
    Ok(())
}

fn read_recovery_header(path: &Path) -> Result<RecoveryHeader, ArchiveError> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != RECOVERY_MAGIC {
        return Err(ArchiveError::UnsupportedFormat(
            "Volume recovery non valido".into(),
        ));
    }
    let mut version = [0u8; 2];
    file.read_exact(&mut version)?;
    if u16::from_le_bytes(version) != VERSION {
        return Err(ArchiveError::UnsupportedFormat(
            "Versione recovery non supportata".into(),
        ));
    }
    let mut flags = [0u8; 2];
    file.read_exact(&mut flags)?;
    if flags != [0u8; 2] {
        return Err(ArchiveError::UnsupportedFormat(
            "Flags recovery non supportate".into(),
        ));
    }
    let index = read_u32(&mut file)?;
    let data_count = read_u32(&mut file)?;
    let parity_count = read_u32(&mut file)?;
    let mut archive_id = [0u8; 16];
    file.read_exact(&mut archive_id)?;
    let volume_size = read_u64(&mut file)?;
    let actual_len = file.metadata()?.len();
    if actual_len != RECOVERY_HEADER_LEN + volume_size {
        return Err(ArchiveError::Message(
            "Lunghezza recovery non coerente".into(),
        ));
    }
    Ok(RecoveryHeader {
        index,
        data_count,
        parity_count,
        archive_id,
        volume_size,
    })
}

fn volume_digest(path: &Path) -> Result<[u8; 32], ArchiveError> {
    let payload_len = fs::metadata(path)?
        .len()
        .checked_sub(VOLUME_HEADER_LEN)
        .ok_or_else(|| ArchiveError::Message("Volume troppo piccolo".into()))?;
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut (&mut file).take(VOLUME_DIGEST_OFFSET))?;
    file.seek(SeekFrom::Start(VOLUME_HEADER_LEN))?;
    hasher.update_reader(&mut file.take(payload_len))?;
    Ok(hasher.finalize().into())
}

fn recovery_digest(path: &Path, volume_size: u64) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    hasher.update_reader(&mut (&mut file).take(RECOVERY_DIGEST_OFFSET))?;
    file.seek(SeekFrom::Start(RECOVERY_HEADER_LEN))?;
    hasher.update_reader(&mut file.take(volume_size))?;
    Ok(hasher.finalize().into())
}

fn read_digest(path: &Path, offset: u64) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(offset))?;
    let mut digest = [0u8; 32];
    file.read_exact(&mut digest)?;
    Ok(digest)
}

fn read_u32(reader: &mut File) -> Result<u32, ArchiveError> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut File) -> Result<u64, ArchiveError> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn volume_path(first: &Path, index: u32) -> PathBuf {
    if index == 1 {
        first.to_owned()
    } else {
        first.with_extension(format!("w{index:02}"))
    }
}

fn recovery_path(first: &Path, index: u32) -> PathBuf {
    first.with_extension(format!("r{index:02}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CreateOptions;
    use zeroize::Zeroizing;

    #[test]
    fn reconstructs_missing_and_corrupted_volumes() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("dati.bin");
        let mut content = vec![0u8; 3 * 1024 * 1024];
        getrandom::fill(&mut content).unwrap();
        fs::write(&source, &content).unwrap();
        let archive = temp.path().join("recovery.wolrar");
        let options = CreateOptions {
            level: 1,
            password: Some(Zeroizing::new("test".into())),
            solid: true,
            comment: None,
            volume_size: Some(64 * 1024),
            recovery_percent: 10,
            deduplicate: false,
        };
        crate::wolformat_v2::create(&archive, &[source], &options, options.volume_size).unwrap();
        assert!(recovery_count(&archive) >= 4);
        fs::remove_file(volume_path(&archive, 2)).unwrap();
        let mut corrupted = fs::OpenOptions::new()
            .write(true)
            .open(volume_path(&archive, 3))
            .unwrap();
        corrupted
            .seek(SeekFrom::Start(VOLUME_HEADER_LEN + 100))
            .unwrap();
        corrupted.write_all(&[0x55]).unwrap();
        drop(corrupted);

        let repaired = temp.path().join("riparato");
        let report = repair(&archive, &repaired).unwrap();
        assert_eq!(report.recovered_volumes, vec![2, 3]);
        let repaired_archive = repaired.join("recovery.wolrar");
        let extracted = temp.path().join("estratto");
        crate::wolformat_v2::extract(&repaired_archive, &extracted, Some("test")).unwrap();
        assert_eq!(fs::read(extracted.join("dati.bin")).unwrap(), content);
    }

    #[test]
    fn refuses_reconstruction_beyond_parity() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("dati.bin");
        let mut content = vec![0u8; 1024 * 1024];
        getrandom::fill(&mut content).unwrap();
        fs::write(&source, content).unwrap();
        let archive = temp.path().join("recovery.wolrar");
        let options = CreateOptions {
            level: 1,
            password: None,
            solid: true,
            comment: None,
            volume_size: Some(64 * 1024),
            recovery_percent: 1,
            deduplicate: false,
        };
        crate::wolformat_v2::create(&archive, &[source], &options, options.volume_size).unwrap();
        for index in 2..=5 {
            fs::remove_file(volume_path(&archive, index)).unwrap();
        }
        assert!(repair(&archive, &temp.path().join("riparato")).is_err());
    }
}

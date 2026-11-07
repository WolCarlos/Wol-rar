use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::ArchiveError;

const TRAILER_MAGIC: &[u8; 8] = b"WOLSFX02";
const TRAILER_LEN: u64 = 16;

#[derive(Debug, serde::Serialize)]
pub struct SfxReport {
    pub output: PathBuf,
    pub archive_bytes: u64,
}

pub struct EmbeddedArchive {
    pub path: PathBuf,
    _temporary: tempfile::NamedTempFile,
}

pub fn create(archive: &Path, output: &Path) -> Result<SfxReport, ArchiveError> {
    let launcher = std::env::current_exe()?;
    create_with_launcher(archive, output, &launcher)
}

pub fn create_with_launcher(
    archive: &Path,
    output: &Path,
    launcher: &Path,
) -> Result<SfxReport, ArchiveError> {
    let info = crate::wolformat_v2::inspect(archive)?;
    if info.volume_count != 1 || info.volume_size.is_some() {
        return Err(ArchiveError::UnsupportedFormat(
            "SFX richiede un archivio .wolrar v2 a volume unico".into(),
        ));
    }
    crate::wolformat_v2::list(archive)?;
    if output.exists() {
        return Err(ArchiveError::DestinationExists(output.to_owned()));
    }
    let archive_bytes = fs::metadata(archive)?.len();
    let result = (|| -> Result<(), ArchiveError> {
        let mut target = File::create_new(output)?;
        let mut source = File::open(launcher)?;
        std::io::copy(&mut source, &mut target)?;
        let mut source = File::open(archive)?;
        std::io::copy(&mut source, &mut target)?;
        target.write_all(TRAILER_MAGIC)?;
        target.write_all(&archive_bytes.to_le_bytes())?;
        target.sync_all()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(output, fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(output);
    }
    result?;
    Ok(SfxReport {
        output: output.to_owned(),
        archive_bytes,
    })
}

pub fn materialize_current() -> Result<Option<EmbeddedArchive>, ArchiveError> {
    let launcher = std::env::current_exe()?;
    materialize(&launcher)
}

pub fn materialize(launcher: &Path) -> Result<Option<EmbeddedArchive>, ArchiveError> {
    let mut file = File::open(launcher)?;
    let total = file.metadata()?.len();
    if total < TRAILER_LEN + 84 {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-(TRAILER_LEN as i64)))?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)?;
    if &magic != TRAILER_MAGIC {
        return Ok(None);
    }
    let archive_bytes = read_u64(&mut file)?;
    if archive_bytes < 84 || archive_bytes > total - TRAILER_LEN {
        return Err(ArchiveError::Message("Trailer SFX non valido".into()));
    }
    let start = total - TRAILER_LEN - archive_bytes;
    file.seek(SeekFrom::Start(start))?;
    let mut volume_magic = [0u8; 8];
    file.read_exact(&mut volume_magic)?;
    if &volume_magic != b"WOLVOL02" {
        return Err(ArchiveError::UnsupportedFormat(
            "Payload SFX non .wolrar v2".into(),
        ));
    }
    file.seek(SeekFrom::Start(start))?;
    let mut temporary = tempfile::NamedTempFile::new()?;
    std::io::copy(&mut file.take(archive_bytes), temporary.as_file_mut())?;
    temporary.as_file_mut().sync_all()?;
    let path = temporary.path().to_owned();
    crate::wolformat_v2::list(&path)?;
    Ok(Some(EmbeddedArchive {
        path,
        _temporary: temporary,
    }))
}

fn read_u64(reader: &mut File) -> Result<u64, ArchiveError> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CreateOptions;

    #[test]
    fn sfx_roundtrip_uses_embedded_single_volume() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("documento.txt");
        fs::write(&source, "contenuto SFX 日本語").unwrap();
        let archive = temp.path().join("prova.wolrar");
        crate::wolformat_v2::create(&archive, &[source], &CreateOptions::default(), None).unwrap();
        let launcher = temp.path().join("launcher");
        fs::write(&launcher, b"launcher-originale").unwrap();
        let output = temp.path().join("estrai.exe");
        create_with_launcher(&archive, &output, &launcher).unwrap();
        let embedded = materialize(&output).unwrap().unwrap();
        let destination = temp.path().join("estratto");
        crate::wolformat_v2::extract(&embedded.path, &destination, None).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("documento.txt")).unwrap(),
            "contenuto SFX 日本語"
        );
    }

    #[test]
    fn rejects_multivolume_sfx() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("dati.bin");
        let mut content = vec![0u8; 1024 * 1024];
        getrandom::fill(&mut content).unwrap();
        fs::write(&source, content).unwrap();
        let archive = temp.path().join("volumi.wolrar");
        crate::wolformat_v2::create(
            &archive,
            &[source],
            &CreateOptions::default(),
            Some(64 * 1024),
        )
        .unwrap();
        assert!(create_with_launcher(&archive, &temp.path().join("sfx.exe"), &archive).is_err());
    }
}

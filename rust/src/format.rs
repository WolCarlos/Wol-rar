use std::path::Path;

use crate::ArchiveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    SevenZip,
    Zip,
    Rar,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
}

impl ArchiveFormat {
    pub fn detect(path: &Path) -> Result<Self, ArchiveError> {
        let name = path.file_name().and_then(|v| v.to_str()).unwrap_or_default().to_ascii_lowercase();
        if name.ends_with(".7z") { return Ok(Self::SevenZip); }
        if name.ends_with(".zip") || name.ends_with(".jar") { return Ok(Self::Zip); }
        if name.ends_with(".rar") { return Ok(Self::Rar); }
        if name.ends_with(".tar") { return Ok(Self::Tar); }
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") { return Ok(Self::TarGz); }
        if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") { return Ok(Self::TarBz2); }
        if name.ends_with(".tar.xz") || name.ends_with(".txz") { return Ok(Self::TarXz); }
        Err(ArchiveError::UnsupportedFormat(name))
    }
}

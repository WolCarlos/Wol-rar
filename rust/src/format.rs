use std::path::Path;

use crate::ArchiveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    WolRar,
    SevenZip,
    Zip,
    Rar,
    Tar,
    TarGz,
    TarBz2,
    TarXz,
    Iso,
    Cab,
    Arj,
    Lzh,
    Uue,
    Z,
}

impl ArchiveFormat {
    pub fn detect(path: &Path) -> Result<Self, ArchiveError> {
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.ends_with(".wolrar") {
            return Ok(Self::WolRar);
        }
        if name.ends_with(".7z") {
            return Ok(Self::SevenZip);
        }
        if name.ends_with(".zip") || name.ends_with(".jar") {
            return Ok(Self::Zip);
        }
        if name.ends_with(".rar") {
            return Ok(Self::Rar);
        }
        if name.ends_with(".tar") {
            return Ok(Self::Tar);
        }
        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            return Ok(Self::TarGz);
        }
        if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
            return Ok(Self::TarBz2);
        }
        if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            return Ok(Self::TarXz);
        }
        if name.ends_with(".iso") {
            return Ok(Self::Iso);
        }
        if name.ends_with(".cab") {
            return Ok(Self::Cab);
        }
        if name.ends_with(".arj") {
            return Ok(Self::Arj);
        }
        if name.ends_with(".lzh") || name.ends_with(".lha") {
            return Ok(Self::Lzh);
        }
        if name.ends_with(".uue") || name.ends_with(".uu") {
            return Ok(Self::Uue);
        }
        if name.ends_with(".z") {
            return Ok(Self::Z);
        }
        Err(ArchiveError::UnsupportedFormat(name))
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::WolRar => ".wolrar",
            Self::SevenZip => "7z",
            Self::Zip => "ZIP",
            Self::Rar => "RAR",
            Self::Tar => "TAR",
            Self::TarGz => "TAR.GZ",
            Self::TarBz2 => "TAR.BZ2",
            Self::TarXz => "TAR.XZ",
            Self::Iso => "ISO",
            Self::Cab => "CAB",
            Self::Arj => "ARJ",
            Self::Lzh => "LHA/LZH",
            Self::Uue => "UUE",
            Self::Z => "Unix compress .Z",
        }
    }

    pub fn supports_create(self) -> bool {
        matches!(
            self,
            Self::WolRar
                | Self::SevenZip
                | Self::Zip
                | Self::Tar
                | Self::TarGz
                | Self::TarBz2
                | Self::TarXz
        )
    }
}

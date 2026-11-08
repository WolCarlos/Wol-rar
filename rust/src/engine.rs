use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::format::ArchiveFormat;
use crate::safe::{safe_destination, validate_member_path};
use crate::{ArchiveEntry, ArchiveError, CreateOptions};

pub struct RustArchiveEngine;

impl RustArchiveEngine {
    pub fn list(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        match ArchiveFormat::detect(path)? {
            ArchiveFormat::WolRar => match crate::wolformat::version(path)? {
                1 => crate::wolformat::list(path),
                2 => crate::wolformat_v2::list(path),
                other => Err(ArchiveError::UnsupportedFormat(format!(
                    "Versione .wolrar {other}"
                ))),
            },
            ArchiveFormat::Zip => list_zip(path, password),
            ArchiveFormat::SevenZip => list_7z(path, password),
            ArchiveFormat::Rar => list_rar(path, password),
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz => list_tar(path),
            format @ (ArchiveFormat::Iso
            | ArchiveFormat::Cab
            | ArchiveFormat::Arj
            | ArchiveFormat::Lzh
            | ArchiveFormat::Uue
            | ArchiveFormat::Z) => crate::legacy::list(path, format, password),
        }
    }

    pub fn create(
        path: &Path,
        sources: &[PathBuf],
        options: &CreateOptions,
    ) -> Result<(), ArchiveError> {
        if path.exists() {
            return Err(ArchiveError::DestinationExists(path.to_owned()));
        }
        if sources.is_empty() {
            return Err(ArchiveError::MissingSource(path.to_owned()));
        }
        if options.level > 9 {
            return Err(ArchiveError::InvalidCompressionLevel(options.level));
        }
        for source in sources {
            if !source.exists() {
                return Err(ArchiveError::MissingSource(source.to_owned()));
            }
            if source.is_dir() && path.starts_with(source) {
                return Err(ArchiveError::ArchiveInsideSource);
            }
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let format = ArchiveFormat::detect(path)?;
        if options.volume_size.is_some() && format != ArchiveFormat::WolRar {
            return Err(ArchiveError::UnsupportedFormat(
                "I volumi sono disponibili solo per .wolrar v2".into(),
            ));
        }
        match format {
            ArchiveFormat::WolRar => {
                crate::wolformat_v2::create(path, sources, options, options.volume_size).map(|_| ())
            }
            ArchiveFormat::Zip => create_zip(path, sources, options),
            ArchiveFormat::SevenZip => create_7z(path, sources, options),
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz => create_tar(path, sources),
            ArchiveFormat::Rar => Err(ArchiveError::UnsupportedFormat(
                "La creazione RAR non è consentita".into(),
            )),
            ArchiveFormat::Iso
            | ArchiveFormat::Cab
            | ArchiveFormat::Arj
            | ArchiveFormat::Lzh
            | ArchiveFormat::Uue
            | ArchiveFormat::Z => Err(ArchiveError::UnsupportedFormat(format!(
                "{} è disponibile soltanto in lettura",
                format.name()
            ))),
        }
    }

    pub fn extract(
        path: &Path,
        destination: &Path,
        password: Option<&str>,
    ) -> Result<(), ArchiveError> {
        if destination.exists() && !destination.is_dir() {
            return Err(ArchiveError::Message(
                "La destinazione non è una cartella".into(),
            ));
        }
        fs::create_dir_all(destination)?;
        match ArchiveFormat::detect(path)? {
            ArchiveFormat::WolRar => match crate::wolformat::version(path)? {
                1 => crate::wolformat::extract(path, destination, password),
                2 => crate::wolformat_v2::extract(path, destination, password),
                other => Err(ArchiveError::UnsupportedFormat(format!(
                    "Versione .wolrar {other}"
                ))),
            },
            ArchiveFormat::Zip => extract_zip(path, destination, password),
            ArchiveFormat::SevenZip => {
                for entry in list_7z(path, password)? {
                    validate_member_path(&entry.path)?;
                }
                match password {
                    Some(value) => sevenz_rust2::decompress_file_with_password(
                        path,
                        destination,
                        value.into(),
                    )?,
                    None => sevenz_rust2::decompress_file(path, destination)?,
                }
                Ok(())
            }
            ArchiveFormat::Rar => extract_rar(path, destination, password),
            ArchiveFormat::Tar
            | ArchiveFormat::TarGz
            | ArchiveFormat::TarBz2
            | ArchiveFormat::TarXz => extract_tar(path, destination),
            format @ (ArchiveFormat::Iso
            | ArchiveFormat::Cab
            | ArchiveFormat::Arj
            | ArchiveFormat::Lzh
            | ArchiveFormat::Uue
            | ArchiveFormat::Z) => crate::legacy::extract(path, destination, format, password),
        }
    }

    pub fn benchmark(size_mib: u64) -> Result<crate::benchmark::BenchmarkReport, ArchiveError> {
        crate::benchmark::run(size_mib)
    }

    pub fn create_sfx(path: &Path, output: &Path) -> Result<crate::sfx::SfxReport, ArchiveError> {
        if ArchiveFormat::detect(path)? != ArchiveFormat::WolRar
            || crate::wolformat::version(path)? != 2
        {
            return Err(ArchiveError::UnsupportedFormat(
                "SFX è disponibile solo per .wolrar v2".into(),
            ));
        }
        crate::sfx::create(path, output)
    }

    pub fn repair(
        path: &Path,
        destination: &Path,
    ) -> Result<crate::wolrecovery::RepairReport, ArchiveError> {
        if ArchiveFormat::detect(path)? != ArchiveFormat::WolRar
            || crate::wolformat::version(path)? != 2
        {
            return Err(ArchiveError::UnsupportedFormat(
                "La riparazione recovery è disponibile solo per .wolrar v2".into(),
            ));
        }
        crate::wolrecovery::repair(path, destination)
    }

    pub fn test(path: &Path, password: Option<&str>) -> Result<(), ArchiveError> {
        let temp = tempfile::tempdir()?;
        Self::extract(path, temp.path(), password)
    }
}

fn list_zip(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let mut entries = Vec::new();
    for index in 0..archive.len() {
        let file = if let Some(password) = password {
            archive.by_index_decrypt(index, password.as_bytes())?
        } else {
            archive.by_index(index)?
        };
        let name = file.name()?.into_owned();
        validate_member_path(&name)?;
        entries.push(ArchiveEntry {
            path: name,
            size: file.size(),
            packed_size: file.compressed_size(),
            modified: format!("{:?}", file.last_modified()),
            directory: file.is_dir(),
            encrypted: file.encrypted(),
            metadata: std::collections::BTreeMap::from([
                ("compression".into(), format!("{:?}", file.compression())),
                ("crc32".into(), format!("{:08x}", file.crc32())),
                (
                    "unix_mode".into(),
                    file.unix_mode()
                        .map(|mode| format!("{mode:o}"))
                        .unwrap_or_default(),
                ),
            ]),
        });
    }
    Ok(entries)
}

fn create_zip(
    path: &Path,
    sources: &[PathBuf],
    options: &CreateOptions,
) -> Result<(), ArchiveError> {
    let file = File::create_new(path)?;
    let mut writer = zip::ZipWriter::new(file);
    let mut options_zip = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(options.level as i64));
    if let Some(password) = &options.password {
        options_zip = options_zip.with_aes_encryption(zip::AesMode::Aes256, password.as_str());
    }
    for source in sources {
        let base = source.parent().unwrap_or(source);
        for item in WalkDir::new(source).follow_links(false) {
            let item = item?;
            let relative = item
                .path()
                .strip_prefix(base)?
                .to_string_lossy()
                .replace('\\', "/");
            if item.file_type().is_dir() {
                writer.add_directory(format!("{relative}/"), options_zip)?;
            } else if item.file_type().is_file() {
                writer.start_file(relative, options_zip)?;
                let mut input = File::open(item.path())?;
                std::io::copy(&mut input, &mut writer)?;
            }
        }
    }
    writer.finish()?;
    Ok(())
}

fn extract_zip(
    path: &Path,
    destination: &Path,
    password: Option<&str>,
) -> Result<(), ArchiveError> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    for index in 0..archive.len() {
        let mut file = if let Some(password) = password {
            archive.by_index_decrypt(index, password.as_bytes())?
        } else {
            archive.by_index(index)?
        };
        let relative = file
            .enclosed_name()
            .ok_or_else(|| ArchiveError::UnsafePath("nome ZIP non sicuro".into()))?;
        validate_member_path(&relative.to_string_lossy())?;
        let output = safe_destination(destination, &relative.to_string_lossy())?;
        if file.is_dir() {
            fs::create_dir_all(output)?;
        } else {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut target = File::options().write(true).create_new(true).open(output)?;
            std::io::copy(&mut file, &mut target)?;
        }
    }
    Ok(())
}

fn list_7z(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let password = sevenz_rust2::Password::from(password.unwrap_or_default());
    let reader = sevenz_rust2::ArchiveReader::open(path, password)?;
    Ok(reader
        .archive()
        .files
        .iter()
        .map(|entry| ArchiveEntry {
            path: entry.name.clone(),
            size: entry.size,
            packed_size: entry.compressed_size,
            modified: format!("{:?}", entry.last_modified_date),
            directory: entry.is_directory,
            encrypted: false,
            metadata: Default::default(),
        })
        .collect())
}

fn create_7z(
    path: &Path,
    sources: &[PathBuf],
    options: &CreateOptions,
) -> Result<(), ArchiveError> {
    let mut writer = sevenz_rust2::ArchiveWriter::create(path)?;
    if let Some(password) = &options.password {
        writer.set_content_methods(vec![
            sevenz_rust2::encoder_options::AesEncoderOptions::new(password.as_str().into()).into(),
            sevenz_rust2::encoder_options::Lzma2Options::from_level(options.level).into(),
        ]);
    }
    for source in sources {
        let base = source.parent().unwrap_or(source);
        for item in WalkDir::new(source).follow_links(false) {
            let item = item?;
            let relative = item
                .path()
                .strip_prefix(base)?
                .to_string_lossy()
                .replace('\\', "/");
            let entry = sevenz_rust2::ArchiveEntry::from_path(item.path(), relative);
            if item.file_type().is_dir() {
                writer.push_archive_entry(entry, None::<File>)?;
            } else if item.file_type().is_file() {
                writer.push_archive_entry(entry, Some(File::open(item.path())?))?;
            }
        }
    }
    writer.finish()?;
    Ok(())
}

fn list_rar(path: &Path, password: Option<&str>) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let archive = match password {
        Some(value) => unrar::Archive::with_password(path, value).open_for_listing()?,
        None => unrar::Archive::new(path).open_for_listing()?,
    };
    let mut entries = Vec::new();
    for item in archive {
        let entry = item?;
        let name = entry.filename.to_string_lossy().into_owned();
        validate_member_path(&name)?;
        entries.push(ArchiveEntry {
            path: name,
            size: entry.unpacked_size,
            packed_size: 0,
            modified: String::new(),
            directory: entry.is_directory(),
            encrypted: entry.is_encrypted(),
            metadata: Default::default(),
        });
    }
    Ok(entries)
}

fn extract_rar(
    path: &Path,
    destination: &Path,
    password: Option<&str>,
) -> Result<(), ArchiveError> {
    let mut archive = match password {
        Some(value) => unrar::Archive::with_password(path, value).open_for_processing()?,
        None => unrar::Archive::new(path).open_for_processing()?,
    };
    while let Some(header) = archive.read_header()? {
        let name = header.entry().filename.to_string_lossy().into_owned();
        validate_member_path(&name)?;
        archive = if header.entry().is_file() {
            header.extract_with_base(destination)?
        } else {
            header.skip()?
        };
    }
    Ok(())
}

fn list_tar(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let mut archive = tar::Archive::new(open_tar_reader(path)?);
    let mut entries = Vec::new();
    for item in archive.entries()? {
        let entry = item?;
        let name = entry.path()?.to_string_lossy().into_owned();
        validate_member_path(&name)?;
        entries.push(ArchiveEntry {
            path: name,
            size: entry.size(),
            packed_size: entry.raw_file_position(),
            modified: entry.header().mtime().unwrap_or_default().to_string(),
            directory: entry.header().entry_type().is_dir(),
            encrypted: false,
            metadata: std::collections::BTreeMap::from([
                (
                    "mode".into(),
                    format!("{:o}", entry.header().mode().unwrap_or_default()),
                ),
                (
                    "uid".into(),
                    entry.header().uid().unwrap_or_default().to_string(),
                ),
                (
                    "gid".into(),
                    entry.header().gid().unwrap_or_default().to_string(),
                ),
            ]),
        });
    }
    Ok(entries)
}

fn create_tar(path: &Path, sources: &[PathBuf]) -> Result<(), ArchiveError> {
    let file = File::create_new(path)?;
    let writer = tar_compression_writer(path, file)?;
    let mut builder = tar::Builder::new(writer);
    for source in sources {
        let base = source.parent().unwrap_or(source);
        for item in WalkDir::new(source).follow_links(false) {
            let item = item?;
            let relative = item.path().strip_prefix(base)?;
            if item.file_type().is_dir() {
                builder.append_dir(relative, item.path())?;
            } else if item.file_type().is_file() {
                builder.append_path_with_name(item.path(), relative)?;
            }
        }
    }
    builder.finish()?;
    Ok(())
}

fn extract_tar(path: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let mut archive = tar::Archive::new(open_tar_reader(path)?);
    for item in archive.entries()? {
        let mut entry = item?;
        let relative = entry.path()?.into_owned();
        validate_member_path(&relative.to_string_lossy())?;
        let output = safe_destination(destination, &relative.to_string_lossy())?;
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(output)?;
        } else if entry.header().entry_type().is_file() {
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut target = File::options().write(true).create_new(true).open(output)?;
            std::io::copy(&mut entry, &mut target)?;
        }
    }
    Ok(())
}

fn open_tar_reader(path: &Path) -> Result<Box<dyn Read>, ArchiveError> {
    let file = File::open(path)?;
    Ok(match ArchiveFormat::detect(path)? {
        ArchiveFormat::Tar => Box::new(file),
        ArchiveFormat::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
        ArchiveFormat::TarBz2 => Box::new(bzip2::read::BzDecoder::new(file)),
        ArchiveFormat::TarXz => Box::new(xz2::read::XzDecoder::new(file)),
        _ => return Err(ArchiveError::UnsupportedFormat(path.display().to_string())),
    })
}

fn tar_compression_writer(path: &Path, file: File) -> Result<Box<dyn Write>, ArchiveError> {
    Ok(match ArchiveFormat::detect(path)? {
        ArchiveFormat::Tar => Box::new(file),
        ArchiveFormat::TarGz => Box::new(flate2::write::GzEncoder::new(
            file,
            flate2::Compression::default(),
        )),
        ArchiveFormat::TarBz2 => Box::new(bzip2::write::BzEncoder::new(
            file,
            bzip2::Compression::default(),
        )),
        ArchiveFormat::TarXz => Box::new(xz2::write::XzEncoder::new(file, 6)),
        _ => return Err(ArchiveError::UnsupportedFormat(path.display().to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_and_7z_roundtrip() {
        for extension in ["zip", "7z"] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("dati");
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("documento àèé.txt"), "contenuto riservato").unwrap();
            let archive = temp.path().join(format!("prova.{extension}"));
            RustArchiveEngine::create(
                &archive,
                std::slice::from_ref(&source),
                &CreateOptions::default(),
            )
            .unwrap();
            RustArchiveEngine::test(&archive, None).unwrap();
            let entries = RustArchiveEngine::list(&archive, None).unwrap();
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.path.ends_with("documento àèé.txt"))
            );
            let destination = temp.path().join("estratto");
            RustArchiveEngine::extract(&archive, &destination, None).unwrap();
            assert_eq!(
                std::fs::read_to_string(destination.join("dati/documento àèé.txt")).unwrap(),
                "contenuto riservato"
            );
        }
    }

    #[test]
    fn aes_roundtrip() {
        for extension in ["zip", "7z"] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("segreto.txt");
            std::fs::write(&source, "contenuto AES").unwrap();
            let archive = temp.path().join(format!("cifrato.{extension}"));
            let options = CreateOptions {
                level: 5,
                password: Some(zeroize::Zeroizing::new("password-di-test".into())),
                solid: true,
                comment: None,
                volume_size: None,
                recovery_percent: 0,
                deduplicate: false,
            };
            RustArchiveEngine::create(&archive, std::slice::from_ref(&source), &options).unwrap();
            assert!(
                RustArchiveEngine::list(&archive, None).is_err()
                    || RustArchiveEngine::test(&archive, None).is_err()
            );
            let destination = temp.path().join("estratto");
            RustArchiveEngine::extract(&archive, &destination, Some("password-di-test")).unwrap();
            assert_eq!(
                std::fs::read_to_string(destination.join("segreto.txt")).unwrap(),
                "contenuto AES"
            );
        }
    }

    #[test]
    fn tar_roundtrip() {
        for extension in ["tar", "tar.gz", "tar.bz2", "tar.xz"] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("dati");
            std::fs::create_dir(&source).unwrap();
            std::fs::write(source.join("documento.txt"), "contenuto TAR").unwrap();
            let archive = temp.path().join(format!("prova.{extension}"));
            RustArchiveEngine::create(
                &archive,
                std::slice::from_ref(&source),
                &CreateOptions::default(),
            )
            .unwrap();
            RustArchiveEngine::test(&archive, None).unwrap();
            let entries = RustArchiveEngine::list(&archive, None).unwrap();
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.path.ends_with("documento.txt"))
            );
            let destination = temp.path().join("estratto");
            RustArchiveEngine::extract(&archive, &destination, None).unwrap();
            assert_eq!(
                std::fs::read_to_string(destination.join("dati/documento.txt")).unwrap(),
                "contenuto TAR"
            );
        }
    }

    #[test]
    fn wolrar_v1_and_v2_use_the_same_engine_contract() {
        for version in [1, 2] {
            let temp = tempfile::tempdir().unwrap();
            let source = temp.path().join("documento àèé.txt");
            std::fs::write(&source, format!("contenuto v{version}")).unwrap();
            let archive = temp.path().join(format!("prova-v{version}.wolrar"));
            if version == 1 {
                crate::wolformat::create(
                    &archive,
                    std::slice::from_ref(&source),
                    &CreateOptions::default(),
                )
                .unwrap();
            } else {
                RustArchiveEngine::create(
                    &archive,
                    std::slice::from_ref(&source),
                    &CreateOptions::default(),
                )
                .unwrap();
            }
            assert_eq!(crate::wolformat::version(&archive).unwrap(), version);
            assert_eq!(RustArchiveEngine::list(&archive, None).unwrap().len(), 1);
            RustArchiveEngine::test(&archive, None).unwrap();
            let destination = temp.path().join("estratto");
            RustArchiveEngine::extract(&archive, &destination, None).unwrap();
            assert_eq!(
                std::fs::read_to_string(destination.join("documento àèé.txt")).unwrap(),
                format!("contenuto v{version}")
            );
        }
    }

    #[test]
    fn refuses_zip_traversal() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("malevolo.zip");
        let file = File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("../fuori.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"non scrivere").unwrap();
        writer.finish().unwrap();
        let destination = temp.path().join("destinazione");
        assert!(RustArchiveEngine::extract(&archive, &destination, None).is_err());
        assert!(!temp.path().join("fuori.txt").exists());
    }

    #[test]
    fn refuses_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("origine.txt");
        std::fs::write(&source, "origine").unwrap();
        let archive = temp.path().join("esistente.zip");
        std::fs::write(&archive, "non toccare").unwrap();
        assert!(RustArchiveEngine::create(&archive, &[source], &CreateOptions::default()).is_err());
        assert_eq!(std::fs::read_to_string(archive).unwrap(), "non toccare");
    }
}

use std::fs::{self, File};
use std::io::{BufReader, Write};
use std::path::Path;

use hadris_iso::directory::DirectoryRef;
use hadris_iso::read::{DirEntry, IsoImage};
use unarc_rs::ArchiveOptions;
use unarc_rs::unified::{ArchiveFormat as UnarcFormat, UnifiedArchive};

use crate::format::ArchiveFormat;
use crate::safe::{safe_destination, validate_member_path};
use crate::{ArchiveEntry, ArchiveError};

const MAX_UUE_INPUT: u64 = 512 * 1024 * 1024;
const MAX_ISO_DEPTH: usize = 64;

type IsoReader = IsoImage<BufReader<File>>;

pub fn list(
    path: &Path,
    format: ArchiveFormat,
    password: Option<&str>,
) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    match format {
        ArchiveFormat::Iso => list_iso(path),
        ArchiveFormat::Uue => list_uue(path),
        ArchiveFormat::Cab | ArchiveFormat::Arj | ArchiveFormat::Lzh | ArchiveFormat::Z => {
            list_unarc(path, format, password)
        }
        _ => Err(ArchiveError::UnsupportedFormat(path.display().to_string())),
    }
}

pub fn extract(
    path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    password: Option<&str>,
) -> Result<(), ArchiveError> {
    match format {
        ArchiveFormat::Iso => extract_iso(path, destination),
        ArchiveFormat::Uue => extract_uue(path, destination),
        ArchiveFormat::Cab | ArchiveFormat::Arj | ArchiveFormat::Lzh | ArchiveFormat::Z => {
            extract_unarc(path, destination, format, password)
        }
        _ => Err(ArchiveError::UnsupportedFormat(path.display().to_string())),
    }
}

fn unarc_format(format: ArchiveFormat) -> UnarcFormat {
    match format {
        ArchiveFormat::Cab => UnarcFormat::Cab,
        ArchiveFormat::Arj => UnarcFormat::Arj,
        ArchiveFormat::Lzh => UnarcFormat::Lha,
        ArchiveFormat::Z => UnarcFormat::Z,
        _ => unreachable!("formato non legacy"),
    }
}

fn open_unarc(
    path: &Path,
    format: ArchiveFormat,
    password: Option<&str>,
) -> Result<UnifiedArchive<BufReader<File>>, ArchiveError> {
    let mut options = ArchiveOptions::new()
        .with_verify_crc(true)
        .with_max_entry_size(Some(4 * 1024 * 1024 * 1024))
        .with_max_total_size(Some(16 * 1024 * 1024 * 1024));
    if let Some(password) = password {
        options = options.with_password(password);
    }
    if format == ArchiveFormat::Z && fs::metadata(path)?.len() < 4 {
        return Err(ArchiveError::Message(
            "Stream .Z troppo corto per contenere dati compressi".into(),
        ));
    }
    let file = BufReader::new(File::open(path)?);
    let mut archive = unarc_format(format)
        .open_with_options(file, options)
        .map_err(|error| ArchiveError::Message(format!("Apertura {}: {error}", format.name())))?;
    if format == ArchiveFormat::Z {
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("contenuto")
            .to_owned();
        archive.set_single_file_name(name);
    }
    Ok(archive)
}

fn list_unarc(
    path: &Path,
    format: ArchiveFormat,
    password: Option<&str>,
) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    if format == ArchiveFormat::Lzh {
        validate_lha_raw_paths(path)?;
    }
    let mut archive = open_unarc(path, format, password)?;
    let mut entries = Vec::new();
    while let Some(entry) = archive
        .next_entry()
        .map_err(|error| ArchiveError::Message(error.to_string()))?
    {
        let name = entry.name().replace('\\', "/");
        validate_member_path(&name)?;
        let size = if format == ArchiveFormat::Z && !entry.is_directory() {
            archive
                .read_to(&entry, &mut std::io::sink())
                .map_err(|error| ArchiveError::Message(error.to_string()))?
        } else {
            entry.original_size()
        };
        entries.push(ArchiveEntry {
            path: name,
            size,
            packed_size: entry.compressed_size(),
            modified: entry
                .modified_time()
                .map(|value| format!("{value:?}"))
                .unwrap_or_default(),
            directory: entry.is_directory(),
            encrypted: entry.is_encrypted(),
            metadata: std::collections::BTreeMap::from([
                ("compression".into(), entry.compression_method().to_owned()),
                ("crc".into(), format!("{:x}", entry.crc())),
            ]),
        });
    }
    Ok(entries)
}

fn extract_unarc(
    path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    password: Option<&str>,
) -> Result<(), ArchiveError> {
    let entries = list_unarc(path, format, password)?;
    prepare_destination(destination, &entries)?;
    let staging = tempfile::tempdir_in(destination)?;
    let mut archive = open_unarc(path, format, password)?;
    while let Some(entry) = archive
        .next_entry()
        .map_err(|error| ArchiveError::Message(error.to_string()))?
    {
        let name = entry.name().replace('\\', "/");
        let output = safe_destination(staging.path(), &name)?;
        if entry.is_directory() {
            fs::create_dir_all(output)?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut target = File::options().write(true).create_new(true).open(output)?;
        let written = archive
            .read_to(&entry, &mut target)
            .map_err(|error| ArchiveError::Message(error.to_string()))?;
        if format != ArchiveFormat::Z && written != entry.original_size() {
            return Err(ArchiveError::Message(format!(
                "Dimensione non valida per {name}: attesa {}, ottenuta {written}",
                entry.original_size()
            )));
        }
    }
    publish(destination, staging.path(), &entries)
}

fn validate_lha_raw_paths(path: &Path) -> Result<(), ArchiveError> {
    let mut reader = delharc::LhaDecodeReader::new(File::open(path)?)
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    loop {
        let header = reader.header();
        validate_lha_raw_name(&header.filename)?;
        for extra in header.iter_extra() {
            if matches!(extra.first(), Some(0x01 | 0x02)) {
                validate_lha_raw_name(&extra[1..])?;
            }
        }
        if !reader
            .seek_next_file()
            .map_err(|error| ArchiveError::Message(error.to_string()))?
        {
            break;
        }
    }
    Ok(())
}

fn validate_lha_raw_name(bytes: &[u8]) -> Result<(), ArchiveError> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let value = String::from_utf8_lossy(&bytes[..end]).replace('\\', "/");
    if value.starts_with('/')
        || value.contains(':')
        || value.split('/').any(|component| component == "..")
    {
        return Err(ArchiveError::UnsafePath(value));
    }
    Ok(())
}

fn list_uue(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_UUE_INPUT {
        return Err(ArchiveError::Message(
            "File UUE oltre il limite di 512 MiB".into(),
        ));
    }
    let data = fs::read(path)?;
    let mut entries = Vec::new();
    for block in uuencoding::scan(&data) {
        let block = block.map_err(|error| ArchiveError::Message(error.to_string()))?;
        if block.is_truncated {
            return Err(ArchiveError::Message(
                "Blocco UUE troncato: terminatore end mancante".into(),
            ));
        }
        let name = block.metadata.filename.replace('\\', "/");
        validate_member_path(&name)?;
        entries.push(ArchiveEntry {
            path: name,
            size: block.data.len() as u64,
            packed_size: 0,
            modified: String::new(),
            directory: false,
            encrypted: false,
            metadata: std::collections::BTreeMap::from([(
                "mode".into(),
                format!("{:o}", block.metadata.mode),
            )]),
        });
    }
    if entries.is_empty() {
        return Err(ArchiveError::Message(
            "Nessun blocco UUE valido trovato".into(),
        ));
    }
    Ok(entries)
}

fn extract_uue(path: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let entries = list_uue(path)?;
    prepare_destination(destination, &entries)?;
    let staging = tempfile::tempdir_in(destination)?;
    let data = fs::read(path)?;
    for block in uuencoding::scan(&data) {
        let block = block.map_err(|error| ArchiveError::Message(error.to_string()))?;
        let name = block.metadata.filename.replace('\\', "/");
        let output = safe_destination(staging.path(), &name)?;
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut target = File::options().write(true).create_new(true).open(output)?;
        target.write_all(&block.data)?;
    }
    publish(destination, staging.path(), &entries)
}

fn list_iso(path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
    let image = IsoImage::open(BufReader::new(File::open(path)?))
        .map_err(|error| ArchiveError::Message(format!("ISO non valida: {error}")))?;
    let mut entries = Vec::new();
    walk_iso(
        &image,
        image.root_dir().dir_ref(),
        "",
        0,
        &mut |_, entry, name| {
            entries.push(ArchiveEntry {
                path: name,
                size: entry.total_size(),
                packed_size: entry.total_size(),
                modified: String::new(),
                directory: entry.is_directory(),
                encrypted: false,
                metadata: Default::default(),
            });
            Ok(())
        },
    )?;
    Ok(entries)
}

fn extract_iso(path: &Path, destination: &Path) -> Result<(), ArchiveError> {
    let entries = list_iso(path)?;
    prepare_destination(destination, &entries)?;
    let staging = tempfile::tempdir_in(destination)?;
    let image = IsoImage::open(BufReader::new(File::open(path)?))
        .map_err(|error| ArchiveError::Message(format!("ISO non valida: {error}")))?;
    walk_iso(
        &image,
        image.root_dir().dir_ref(),
        "",
        0,
        &mut |image, entry, name| {
            let output = safe_destination(staging.path(), &name)?;
            if entry.is_directory() {
                fs::create_dir_all(output)?;
                return Ok(());
            }
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut target = File::options().write(true).create_new(true).open(output)?;
            let mut chunks = image
                .read_file_chunked::<65536>(entry)
                .map_err(|error| ArchiveError::Message(error.to_string()))?;
            let mut written = 0u64;
            while let Some(chunk) = chunks
                .next_chunk()
                .map_err(|error| ArchiveError::Message(error.to_string()))?
            {
                target.write_all(&chunk)?;
                written += chunk.len() as u64;
            }
            if written != entry.total_size() {
                return Err(ArchiveError::Message(format!(
                    "Dimensione non valida per {name}: attesa {}, ottenuta {written}",
                    entry.total_size()
                )));
            }
            Ok(())
        },
    )?;
    publish(destination, staging.path(), &entries)
}

fn walk_iso(
    image: &IsoReader,
    directory: DirectoryRef,
    prefix: &str,
    depth: usize,
    visitor: &mut impl FnMut(&IsoReader, &DirEntry, String) -> Result<(), ArchiveError>,
) -> Result<(), ArchiveError> {
    if depth > MAX_ISO_DEPTH {
        return Err(ArchiveError::Message(
            "Annidamento ISO oltre il limite di 64 directory".into(),
        ));
    }
    for item in image.open_dir(directory).entries() {
        let entry = item.map_err(|error| ArchiveError::Message(error.to_string()))?;
        if entry.is_special() {
            continue;
        }
        let display = if entry.name().contains(&0) {
            hadris_iso::joliet::decode_joliet_name(entry.name())
        } else {
            entry.display_name().into_owned()
        };
        let name = if prefix.is_empty() {
            display
        } else {
            format!("{prefix}/{display}")
        };
        validate_member_path(&name)?;
        if entry.is_directory() {
            visitor(image, &entry, name.clone())?;
            walk_iso(
                image,
                entry
                    .as_dir_ref(image)
                    .map_err(|error| ArchiveError::Message(error.to_string()))?,
                &name,
                depth + 1,
                visitor,
            )?;
        } else if entry.is_file() {
            visitor(image, &entry, name)?;
        }
    }
    Ok(())
}

fn prepare_destination(destination: &Path, entries: &[ArchiveEntry]) -> Result<(), ArchiveError> {
    match fs::symlink_metadata(destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(ArchiveError::Message(
                "La destinazione non è una cartella sicura".into(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(destination)?;
        }
        Err(error) => return Err(error.into()),
    }
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

fn publish(
    destination: &Path,
    staging: &Path,
    entries: &[ArchiveEntry],
) -> Result<(), ArchiveError> {
    let mut published = Vec::new();
    let result = (|| -> Result<(), ArchiveError> {
        for entry in entries {
            let staged = safe_destination(staging, &entry.path)?;
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
            published.push(output);
            fs::remove_file(staged)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uue_unicode_roundtrip_is_transactional() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("dati.uue");
        let payload = "Contenuto Unicode 日本語".as_bytes();
        fs::write(
            &archive,
            uuencoding::encode(payload, "cartella/documento àèé.txt", 0o644),
        )
        .unwrap();
        let entries = list(&archive, ArchiveFormat::Uue, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, payload.len() as u64);
        let destination = temp.path().join("estratto");
        extract(&archive, &destination, ArchiveFormat::Uue, None).unwrap();
        assert_eq!(
            fs::read(destination.join("cartella/documento àèé.txt")).unwrap(),
            payload
        );
        let second = temp.path().join("secondo");
        extract(&archive, &second, ArchiveFormat::Uue, None).unwrap();
        assert!(extract(&archive, &second, ArchiveFormat::Uue, None).is_err());
        assert_eq!(
            fs::read(second.join("cartella/documento àèé.txt")).unwrap(),
            payload
        );
    }

    #[test]
    fn uue_rejects_traversal_and_truncation_without_external_writes() {
        let temp = tempfile::tempdir().unwrap();
        let traversal = temp.path().join("traversal.uue");
        fs::write(
            &traversal,
            uuencoding::encode(b"non scrivere", "../fuori.txt", 0o644),
        )
        .unwrap();
        let destination = temp.path().join("destinazione");
        assert!(extract(&traversal, &destination, ArchiveFormat::Uue, None).is_err());
        assert!(!temp.path().join("fuori.txt").exists());

        let truncated = temp.path().join("troncato.uue");
        let mut data = uuencoding::encode(b"contenuto", "file.txt", 0o644);
        data.truncate(data.len() - 4);
        fs::write(&truncated, data).unwrap();
        assert!(list(&truncated, ArchiveFormat::Uue, None).is_err());
    }

    fn crc16(data: &[u8]) -> u16 {
        let mut crc = 0u16;
        for byte in data {
            crc ^= u16::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xa001
                } else {
                    crc >> 1
                };
            }
        }
        crc
    }

    fn lha_stored(name: &str, payload: &[u8]) -> Vec<u8> {
        let name = name.as_bytes();
        let size = u32::try_from(payload.len()).unwrap();
        let mut header = vec![0, 0];
        header.extend_from_slice(b"-lh0-");
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&0u32.to_le_bytes());
        header.push(0x20);
        header.push(0);
        header.push(u8::try_from(name.len()).unwrap());
        header.extend_from_slice(name);
        header.extend_from_slice(&crc16(payload).to_le_bytes());
        header[0] = u8::try_from(header.len() - 2).unwrap();
        header[1] = header[2..]
            .iter()
            .fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        header.extend_from_slice(payload);
        header.push(0);
        header
    }

    #[test]
    fn arj_cab_lzh_and_z_match_an_independent_expected_file() {
        let expected = include_bytes!("../fixtures/legacy/expected-license.txt");
        let lha = lha_stored("LICENSE", expected);
        let cases = [
            (
                ArchiveFormat::Arj,
                "arj",
                include_bytes!("../fixtures/legacy/stored.arj").as_slice(),
            ),
            (
                ArchiveFormat::Cab,
                "cab",
                include_bytes!("../fixtures/legacy/license_none.cab").as_slice(),
            ),
            (ArchiveFormat::Lzh, "lzh", lha.as_slice()),
            (
                ArchiveFormat::Z,
                "Z",
                include_bytes!("../fixtures/legacy/LICENSE.Z").as_slice(),
            ),
        ];
        for (format, extension, data) in cases {
            let temp = tempfile::tempdir().unwrap();
            let archive = temp.path().join(format!("fixture.{extension}"));
            fs::write(&archive, data).unwrap();
            let entries = crate::RustArchiveEngine::list(&archive, None).unwrap();
            let expected_name = if format == ArchiveFormat::Z {
                "fixture"
            } else {
                "LICENSE"
            };
            assert!(
                entries.iter().any(|entry| entry.path == expected_name),
                "entry {expected_name} mancante in {}: {entries:?}",
                format.name()
            );
            crate::RustArchiveEngine::test(&archive, None).unwrap();
            let destination = temp.path().join("estratto");
            crate::RustArchiveEngine::extract(&archive, &destination, None).unwrap();
            assert_eq!(fs::read(destination.join(expected_name)).unwrap(), expected);
        }
    }

    #[test]
    fn legacy_backends_reject_traversal_and_truncated_input() {
        let expected = include_bytes!("../fixtures/legacy/expected-license.txt");
        for (extension, original) in [
            (
                "arj",
                include_bytes!("../fixtures/legacy/stored.arj").as_slice(),
            ),
            (
                "cab",
                include_bytes!("../fixtures/legacy/license_none.cab").as_slice(),
            ),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let mut malicious = original.to_vec();
            let position = malicious
                .windows(7)
                .position(|window| window == b"LICENSE")
                .unwrap();
            malicious[position..position + 7].copy_from_slice(b"../evil");
            let archive = temp.path().join(format!("traversal.{extension}"));
            fs::write(&archive, malicious).unwrap();
            assert!(crate::RustArchiveEngine::list(&archive, None).is_err());
            assert!(!temp.path().join("evil").exists());

            let truncated = temp.path().join(format!("troncato.{extension}"));
            fs::write(&truncated, &original[..original.len() / 2]).unwrap();
            assert!(crate::RustArchiveEngine::test(&truncated, None).is_err());
        }

        let temp = tempfile::tempdir().unwrap();
        let traversal = temp.path().join("traversal.lzh");
        fs::write(&traversal, lha_stored("../evil", expected)).unwrap();
        assert!(crate::RustArchiveEngine::list(&traversal, None).is_err());
        let truncated = temp.path().join("troncato.lzh");
        let valid = lha_stored("LICENSE", expected);
        fs::write(&truncated, &valid[..valid.len() / 2]).unwrap();
        assert!(crate::RustArchiveEngine::test(&truncated, None).is_err());

        let truncated_z = temp.path().join("troncato.Z");
        let valid_z = include_bytes!("../fixtures/legacy/LICENSE.Z");
        // .Z non ha digest né lunghezza originale: soltanto alcuni troncamenti sono rilevabili.
        fs::write(&truncated_z, &valid_z[..3]).unwrap();
        assert!(crate::RustArchiveEngine::test(&truncated_z, None).is_err());
    }

    #[test]
    fn iso_joliet_unicode_roundtrip_and_truncation() {
        use std::io::Cursor;
        use std::sync::Arc;

        use hadris_iso::joliet::JolietLevel;
        use hadris_iso::read::PathSeparator;
        use hadris_iso::write::options::{CreationFeatures, IsoFormatOptions};
        use hadris_iso::write::{File as IsoFile, InputFiles, IsoImageWriter};

        let payload = b"Contenuto ISO Unicode";
        let files = InputFiles {
            path_separator: PathSeparator::ForwardSlash,
            files: vec![IsoFile::File {
                name: Arc::new("cartella/documento àèé.txt".into()),
                contents: payload.to_vec(),
            }],
        };
        let options = IsoFormatOptions {
            volume_name: "WOLRAR_TEST".into(),
            system_id: None,
            volume_set_id: None,
            publisher_id: None,
            preparer_id: None,
            application_id: None,
            sector_size: 2048,
            path_separator: PathSeparator::ForwardSlash,
            features: CreationFeatures {
                joliet: Some(JolietLevel::Level3),
                ..Default::default()
            },
            strict_charset: false,
        };
        let mut buffer = Cursor::new(vec![0u8; 1024 * 1024]);
        IsoImageWriter::create(&mut buffer, files, options).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("unicode.iso");
        fs::write(&archive, buffer.into_inner()).unwrap();
        let entries = crate::RustArchiveEngine::list(&archive, None).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.path == "cartella/documento àèé.txt"),
            "nome Joliet non trovato: {entries:?}"
        );
        crate::RustArchiveEngine::test(&archive, None).unwrap();
        let destination = temp.path().join("estratto");
        crate::RustArchiveEngine::extract(&archive, &destination, None).unwrap();
        assert_eq!(
            fs::read(destination.join("cartella/documento àèé.txt")).unwrap(),
            payload
        );

        let truncated = temp.path().join("troncata.iso");
        fs::write(&truncated, vec![0u8; 2048]).unwrap();
        assert!(crate::RustArchiveEngine::list(&truncated, None).is_err());
    }
}

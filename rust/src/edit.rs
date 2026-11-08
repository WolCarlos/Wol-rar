use std::fs::{self, File};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zeroize::Zeroizing;

use crate::format::ArchiveFormat;
use crate::safe::{safe_destination, validate_member_path};
use crate::{ArchiveError, CreateOptions, RustArchiveEngine};

#[derive(Debug, Clone)]
pub enum EditOperation {
    Delete { entry: String },
    Rename { entry: String, new_path: String },
    Add { source: PathBuf, entry: String },
    Update { source: PathBuf, entry: String },
    Convert,
}

#[derive(Debug, Serialize)]
pub struct EditReport {
    pub operation: String,
    pub archive: PathBuf,
    pub output: PathBuf,
    pub entry_count: usize,
    pub verified: bool,
}

pub fn rewrite(
    archive: &Path,
    output: Option<&Path>,
    operation: EditOperation,
    mut options: CreateOptions,
    password: Option<Zeroizing<String>>,
) -> Result<EditReport, ArchiveError> {
    let output = output.unwrap_or(archive);
    let operation_name = operation_name(&operation);
    let source_format = ArchiveFormat::detect(archive)?;
    let output_format = ArchiveFormat::detect(output)?;
    if !output_format.supports_create() {
        return Err(ArchiveError::UnsupportedFormat(format!(
            "Conversione verso {} non supportata",
            output_format.name()
        )));
    }
    if options.volume_size.is_some() || options.recovery_percent != 0 {
        return Err(ArchiveError::UnsupportedFormat(
            "Volumi e recovery non sono modificabili/transcodificabili; creare un nuovo archivio"
                .into(),
        ));
    }

    let source_entries =
        RustArchiveEngine::list(archive, password.as_ref().map(|value| value.as_str()))?;
    let source_encrypted = source_entries.iter().any(|entry| entry.encrypted);
    if source_encrypted
        && matches!(
            output_format,
            ArchiveFormat::Tar
                | ArchiveFormat::TarGz
                | ArchiveFormat::TarBz2
                | ArchiveFormat::TarXz
        )
    {
        return Err(ArchiveError::UnsupportedFormat(
            "La cifratura non può essere preservata nel formato TAR scelto".into(),
        ));
    }
    if source_encrypted {
        let value = password.clone().ok_or_else(|| {
            ArchiveError::Message("Password richiesta per preservare la cifratura".into())
        })?;
        options.password = Some(value);
    }

    let mut source_comment = String::new();
    if source_format == ArchiveFormat::WolRar {
        match crate::wolformat::version(archive)? {
            1 => {}
            2 => {
                let info = crate::wolformat_v2::inspect(archive)?;
                if info.volume_count != 1 || crate::wolrecovery::recovery_count(archive) != 0 {
                    return Err(ArchiveError::UnsupportedFormat(
                        "Volumi e recovery .wolrar non sono modificabili; creare un nuovo archivio"
                            .into(),
                    ));
                }
                if crate::wolformat_v2::has_deduplication(archive)? {
                    options.deduplicate = true;
                }
            }
            other => {
                return Err(ArchiveError::UnsupportedFormat(format!(
                    "Versione .wolrar {other}"
                )));
            }
        }
        source_comment = crate::wolformat::comment(archive)?;
        if output_format == ArchiveFormat::WolRar && options.comment.is_none() {
            options.comment = Some(source_comment.clone());
        }
    }
    if output_format != ArchiveFormat::WolRar && !source_comment.is_empty() {
        return Err(ArchiveError::UnsupportedFormat(
            "Il commento .wolrar andrebbe perso nella conversione scelta".into(),
        ));
    }

    let destination_parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(destination_parent)?;
    let workspace = tempfile::tempdir_in(destination_parent)?;
    let extracted = tempfile::tempdir()?;
    RustArchiveEngine::extract(
        archive,
        extracted.path(),
        password.as_ref().map(|value| value.as_str()),
    )?;
    apply_operation(extracted.path(), &operation)?;

    let file_name = output.file_name().ok_or_else(|| {
        ArchiveError::Message("Il percorso di output non contiene un nome file".into())
    })?;
    let rewritten = workspace.path().join(file_name);
    let sources = fs::read_dir(extracted.path())?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    RustArchiveEngine::create(&rewritten, &sources, &options)?;
    RustArchiveEngine::test(
        &rewritten,
        options.password.as_ref().map(|value| value.as_str()),
    )?;
    let entry_count = RustArchiveEngine::list(
        &rewritten,
        options.password.as_ref().map(|value| value.as_str()),
    )?
    .len();
    publish_verified(archive, output, &rewritten, workspace.path())?;

    Ok(EditReport {
        operation: operation_name.into(),
        archive: archive.to_path_buf(),
        output: output.to_path_buf(),
        entry_count,
        verified: true,
    })
}

fn operation_name(operation: &EditOperation) -> &'static str {
    match operation {
        EditOperation::Delete { .. } => "delete",
        EditOperation::Rename { .. } => "rename",
        EditOperation::Add { .. } => "add",
        EditOperation::Update { .. } => "update",
        EditOperation::Convert => "convert",
    }
}

fn apply_operation(root: &Path, operation: &EditOperation) -> Result<(), ArchiveError> {
    match operation {
        EditOperation::Convert => Ok(()),
        EditOperation::Delete { entry } => {
            let path = safe_destination(root, entry)?;
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else if path.is_file() {
                fs::remove_file(path)?;
            } else {
                return Err(ArchiveError::MissingSource(path));
            }
            Ok(())
        }
        EditOperation::Rename { entry, new_path } => {
            validate_member_path(new_path)?;
            let source = safe_destination(root, entry)?;
            let destination = safe_destination(root, new_path)?;
            if !source.exists() {
                return Err(ArchiveError::MissingSource(source));
            }
            if destination.exists() {
                return Err(ArchiveError::DestinationExists(destination));
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(source, destination)?;
            Ok(())
        }
        EditOperation::Add { source, entry } => {
            if !source.is_file() {
                return Err(ArchiveError::MissingSource(source.clone()));
            }
            let destination = safe_destination(root, entry)?;
            if destination.exists() {
                return Err(ArchiveError::DestinationExists(destination));
            }
            copy_with_metadata(source, &destination, false)
        }
        EditOperation::Update { source, entry } => {
            if !source.is_file() {
                return Err(ArchiveError::MissingSource(source.clone()));
            }
            let destination = safe_destination(root, entry)?;
            if !destination.is_file() {
                return Err(ArchiveError::MissingSource(destination));
            }
            copy_with_metadata(source, &destination, true)
        }
    }
}

fn copy_with_metadata(
    source: &Path,
    destination: &Path,
    replace: bool,
) -> Result<(), ArchiveError> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if replace {
        fs::copy(source, destination)?;
    } else {
        let mut input = File::open(source)?;
        let mut output = File::options()
            .write(true)
            .create_new(true)
            .open(destination)?;
        std::io::copy(&mut input, &mut output)?;
    }
    if let Ok(modified) = source.metadata().and_then(|metadata| metadata.modified()) {
        let timestamp = filetime::FileTime::from_system_time(modified);
        filetime::set_file_mtime(destination, timestamp)?;
    }
    Ok(())
}

fn publish_verified(
    original: &Path,
    output: &Path,
    rewritten: &Path,
    workspace: &Path,
) -> Result<(), ArchiveError> {
    let same = original == output
        || (output.exists() && fs::canonicalize(original)? == fs::canonicalize(output)?);
    if !same {
        if output.exists() {
            return Err(ArchiveError::DestinationExists(output.to_path_buf()));
        }
        fs::rename(rewritten, output)?;
        return Ok(());
    }

    let backup = workspace.join("originale-precedente");
    fs::rename(original, &backup)?;
    if let Err(error) = fs::rename(rewritten, output) {
        fs::rename(&backup, original)?;
        return Err(error.into());
    }
    fs::remove_file(backup)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive_with_files(root: &Path) -> PathBuf {
        let source = root.join("sorgente");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("uno.txt"), "uno").unwrap();
        fs::write(source.join("due.txt"), "due").unwrap();
        let archive = root.join("archivio.zip");
        RustArchiveEngine::create(&archive, &[source], &CreateOptions::default()).unwrap();
        archive
    }

    #[test]
    fn delete_rename_add_update_and_convert_are_verified() {
        let temp = tempfile::tempdir().unwrap();
        let archive = archive_with_files(temp.path());
        let original = fs::read(&archive).unwrap();
        RustArchiveEngine::test(&archive, None).unwrap();

        rewrite(
            &archive,
            None,
            EditOperation::Delete {
                entry: "sorgente/uno.txt".into(),
            },
            CreateOptions::default(),
            None,
        )
        .unwrap();
        let entries = RustArchiveEngine::list(&archive, None).unwrap();
        assert!(!entries.iter().any(|entry| entry.path == "sorgente/uno.txt"));
        assert_ne!(fs::read(&archive).unwrap(), original);

        rewrite(
            &archive,
            None,
            EditOperation::Rename {
                entry: "sorgente/due.txt".into(),
                new_path: "sorgente/rinominato àèé.txt".into(),
            },
            CreateOptions::default(),
            None,
        )
        .unwrap();
        assert!(fs::read(&archive).is_ok());

        let added = temp.path().join("aggiunto.txt");
        fs::write(&added, "nuovo").unwrap();
        rewrite(
            &archive,
            None,
            EditOperation::Add {
                source: added.clone(),
                entry: "sorgente/aggiunto.txt".into(),
            },
            CreateOptions::default(),
            None,
        )
        .unwrap();
        fs::write(&added, "aggiornato").unwrap();
        rewrite(
            &archive,
            None,
            EditOperation::Update {
                source: added,
                entry: "sorgente/aggiunto.txt".into(),
            },
            CreateOptions::default(),
            None,
        )
        .unwrap();
        let converted = temp.path().join("convertito.wolrar");
        rewrite(
            &archive,
            Some(&converted),
            EditOperation::Convert,
            CreateOptions::default(),
            None,
        )
        .unwrap();
        RustArchiveEngine::test(&converted, None).unwrap();
        let destination = temp.path().join("estratto");
        RustArchiveEngine::extract(&converted, &destination, None).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("sorgente/aggiunto.txt")).unwrap(),
            "aggiornato"
        );
        assert!(destination.join("sorgente/rinominato àèé.txt").is_file());
    }

    #[test]
    fn failed_operation_preserves_original_and_does_not_create_output() {
        let temp = tempfile::tempdir().unwrap();
        let archive = archive_with_files(temp.path());
        let original = fs::read(&archive).unwrap();
        assert!(
            rewrite(
                &archive,
                None,
                EditOperation::Delete {
                    entry: "mancante.txt".into(),
                },
                CreateOptions::default(),
                None,
            )
            .is_err()
        );
        assert_eq!(fs::read(&archive).unwrap(), original);

        let output = temp.path().join("mancante.wolrar");
        assert!(
            rewrite(
                &archive,
                Some(&output),
                EditOperation::Delete {
                    entry: "mancante.txt".into(),
                },
                CreateOptions::default(),
                None,
            )
            .is_err()
        );
        assert!(!output.exists());
        assert_eq!(fs::read(&archive).unwrap(), original);
    }

    #[test]
    fn preserves_encryption_comment_and_deduplication() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("dati");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("uno.txt"), "uguale").unwrap();
        fs::write(source.join("due.txt"), "uguale").unwrap();
        fs::write(source.join("terzo.txt"), "diverso").unwrap();
        let archive = temp.path().join("cifrato.wolrar");
        let options = CreateOptions {
            password: Some(Zeroizing::new("password-di-test".into())),
            comment: Some("Commento da preservare".into()),
            deduplicate: true,
            ..CreateOptions::default()
        };
        RustArchiveEngine::create(&archive, &[source], &options).unwrap();
        let password = Some(Zeroizing::new("password-di-test".into()));
        rewrite(
            &archive,
            None,
            EditOperation::Delete {
                entry: "dati/terzo.txt".into(),
            },
            CreateOptions::default(),
            password.clone(),
        )
        .unwrap();
        let info = crate::wolformat_v2::inspect(&archive).unwrap();
        assert!(info.encrypted);
        assert_eq!(info.comment, "Commento da preservare");
        assert!(crate::wolformat_v2::has_deduplication(&archive).unwrap());
        RustArchiveEngine::test(&archive, Some("password-di-test")).unwrap();
        assert!(RustArchiveEngine::test(&archive, Some("errata")).is_err());

        let original = fs::read(&archive).unwrap();
        let zip = temp.path().join("perderebbe-commento.zip");
        assert!(
            rewrite(
                &archive,
                Some(&zip),
                EditOperation::Convert,
                CreateOptions::default(),
                password,
            )
            .is_err()
        );
        assert_eq!(fs::read(&archive).unwrap(), original);
        assert!(!zip.exists());
    }

    #[test]
    fn rejects_volume_editing_before_touching_the_original() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("grande.bin");
        let mut random_data = vec![0u8; 256 * 1024];
        getrandom::fill(&mut random_data).unwrap();
        fs::write(&source, random_data).unwrap();
        let archive = temp.path().join("volumi.wolrar");
        let options = CreateOptions {
            volume_size: Some(65536),
            recovery_percent: 10,
            ..CreateOptions::default()
        };
        RustArchiveEngine::create(&archive, &[source], &options).unwrap();
        let original = fs::read(&archive).unwrap();
        assert!(
            rewrite(
                &archive,
                None,
                EditOperation::Convert,
                CreateOptions::default(),
                None,
            )
            .is_err()
        );
        assert_eq!(fs::read(&archive).unwrap(), original);
        assert!(temp.path().join("volumi.w02").exists());
    }

    #[test]
    fn converts_to_every_writable_format() {
        let temp = tempfile::tempdir().unwrap();
        let archive = archive_with_files(temp.path());
        for extension in ["wolrar", "7z", "tar", "tar.gz", "tar.bz2", "tar.xz"] {
            let output = temp.path().join(format!("convertito.{extension}"));
            rewrite(
                &archive,
                Some(&output),
                EditOperation::Convert,
                CreateOptions::default(),
                None,
            )
            .unwrap();
            RustArchiveEngine::test(&output, None).unwrap();
            let destination = temp.path().join(format!("estratto-{extension}"));
            RustArchiveEngine::extract(&output, &destination, None).unwrap();
            assert_eq!(
                fs::read_to_string(destination.join("sorgente/due.txt")).unwrap(),
                "due"
            );
        }
    }
}

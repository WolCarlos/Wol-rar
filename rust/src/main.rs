use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use wol_rar::{CreateOptions, RustArchiveEngine};
use zeroize::Zeroizing;

const AGENT_PROMPT: &str = include_str!("../../agent/AGENTS.snippet.md");

#[derive(Debug, Parser)]
#[command(
    name = "Wol-rar",
    version,
    about = "Archiviatore privato agent-first scritto in Rust"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    archive: Option<PathBuf>,
    #[arg(long)]
    open: Option<PathBuf>,
    #[arg(long)]
    extract: Option<PathBuf>,
    #[arg(long)]
    test: Option<PathBuf>,
    #[arg(long)]
    create: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    destination: Option<PathBuf>,
    #[arg(long, default_value_t = 5)]
    level: u32,
    #[arg(long, global = true)]
    comment: Option<String>,
    /// Dimensione massima di ogni volume .wolrar v2 in byte (minimo 65536)
    #[arg(long, global = true)]
    volume_size: Option<u64>,
    /// Percentuale di parity Reed-Solomon per volumi .wolrar v2 (0-100)
    #[arg(long, global = true, default_value_t = 0)]
    recovery_percent: u8,
    /// Deduplica i contenuti identici durante la creazione .wolrar v2
    #[arg(long, global = true)]
    deduplicate: bool,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    password_stdin: bool,
    #[arg(long)]
    gui: bool,
    #[arg(long, hide = true)]
    smoke_test: bool,
    #[arg(long, hide = true)]
    gui_smoke_test: bool,
    #[arg(long, hide = true)]
    gui_smoke_test_light: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Elenca il contenuto di un archivio
    List { archive: PathBuf },
    /// Crea un archivio .wolrar, 7z, ZIP, TAR o TAR compresso
    Create {
        #[arg(required = true, num_args = 1..)]
        sources: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 5)]
        level: u32,
    },
    /// Mostra i metadati pubblici di un archivio .wolrar
    Info { archive: PathBuf },
    /// Estrai in una nuova cartella senza sovrascrivere
    Extract {
        archive: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
    /// Verifica integrità tramite decodifica completa
    Test { archive: PathBuf },
    /// Misura creazione ed estrazione .wolrar v2 con verifica digest
    Benchmark {
        #[arg(long, default_value_t = 32)]
        size_mib: u64,
        /// Misura anche cifratura, volumi, recovery, ZIP e 7z
        #[arg(long)]
        suite: bool,
        /// Salva il report JSON in un nuovo file
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Crea un eseguibile autoestraente da un .wolrar v2 a volume unico
    Sfx {
        archive: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Ricostruisce volumi .wolrar v2 in una nuova cartella
    Repair {
        archive: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
    /// Elimina una voce riscrivendo e verificando l'archivio
    Delete {
        archive: PathBuf,
        entry: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Rinomina una voce con percorso relativo sicuro
    Rename {
        archive: PathBuf,
        entry: String,
        to: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Aggiunge un file a un archivio esistente
    Add {
        archive: PathBuf,
        source: PathBuf,
        entry: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Aggiorna il contenuto di una voce esistente
    Update {
        archive: PathBuf,
        source: PathBuf,
        entry: String,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Converte in un formato supportato in scrittura
    Convert {
        archive: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Mostra formati e capacità del motore
    Formats,
    /// Controlla la release più recente su GitHub
    UpdateCheck,
    /// Scarica e verifica l'aggiornamento nella cache gestita
    UpdateDownload,
    /// Avvia l'installer Windows verificato e attende il suo esito
    UpdateInstall { asset: PathBuf },
    /// Mostra il prompt compatto per AGENTS.md
    Agent,
}

fn main() {
    let args = Args::parse();
    let json = args.json;
    match execute(args) {
        Ok(value) => match value {
            Value::String(text) => println!("{text}"),
            other => println!("{}", serde_json::to_string_pretty(&other).unwrap()),
        },
        Err(error) => {
            if json {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&json!({"ok": false, "error": error.to_string()}))
                        .unwrap()
                );
            } else {
                eprintln!("Errore: {error}");
            }
            std::process::exit(1);
        }
    }
}

fn execute(args: Args) -> Result<Value, wol_rar::ArchiveError> {
    if args.smoke_test {
        return Ok(json!({"ok": true, "engine": "rust", "version": env!("CARGO_PKG_VERSION")}));
    }
    if args.gui_smoke_test || args.gui_smoke_test_light {
        wol_rar::gui::run_with_theme(
            wol_rar::gui::GuiAction::None,
            Some(std::time::Instant::now() + std::time::Duration::from_secs(3)),
            args.gui_smoke_test_light,
        )
        .map_err(wol_rar::ArchiveError::Message)?;
        return Ok(json!({"ok": true, "gui": "eframe", "version": env!("CARGO_PKG_VERSION")}));
    }
    if args.command.is_none()
        && !args.gui
        && let Some(embedded) = wol_rar::sfx::materialize_current()?
    {
        wol_rar::gui::run(wol_rar::gui::GuiAction::Open(embedded.path.clone()), None)
            .map_err(wol_rar::ArchiveError::Message)?;
        return Ok(json!({"ok": true, "gui": "eframe", "sfx": true}));
    }
    if args.gui {
        let action = if let Some(path) = args.open.clone() {
            wol_rar::gui::GuiAction::Open(path)
        } else if let Some(path) = args.extract.clone() {
            wol_rar::gui::GuiAction::Extract(path)
        } else if let Some(path) = args.test.clone() {
            wol_rar::gui::GuiAction::Test(path)
        } else if let Some(path) = args.create.clone() {
            wol_rar::gui::GuiAction::Create(path)
        } else if let Some(path) = args.archive.clone() {
            wol_rar::gui::GuiAction::Open(path)
        } else {
            wol_rar::gui::GuiAction::None
        };
        wol_rar::gui::run(action, None).map_err(wol_rar::ArchiveError::Message)?;
        return Ok(json!({"ok": true, "gui": "eframe"}));
    }
    let password = read_password(args.password_stdin)?;
    let edit_level = args.level;
    let edit_comment = args.comment.clone();
    let edit_volume_size = args.volume_size;
    let edit_recovery_percent = args.recovery_percent;
    let edit_deduplicate = args.deduplicate;
    let make_edit_options = |password| CreateOptions {
        level: edit_level,
        password,
        solid: true,
        comment: edit_comment.clone(),
        volume_size: edit_volume_size,
        recovery_percent: edit_recovery_percent,
        deduplicate: edit_deduplicate,
    };
    match args.command {
        Some(Command::Agent) => Ok(Value::String(AGENT_PROMPT.into())),
        Some(Command::Formats) => Ok(json!({
            "ok": true,
            "create": ["wolrar", "7z", "zip", "tar", "tar.gz", "tar.bz2", "tar.xz"],
            "read": ["wolrar", "7z", "zip", "rar", "tar", "tar.gz", "tar.bz2", "tar.xz", "iso", "cab", "arj", "lzh", "lha", "uue", "z"],
            "edit": ["delete", "rename", "add", "update", "convert"],
            "legacy_limitations": {"z": "Nessun digest o lunghezza originale: alcuni troncamenti non sono rilevabili"},
            "wolrar": {
                "write_version": 2,
                "read_versions": [1, 2],
                "compression": "Zstandard",
                "integrity": "BLAKE3",
                "encryption": "XChaCha20-Poly1305",
                "kdf": "Argon2id",
                "streaming_encryption": true,
                "chunk_size_mib": 1,
                "volumes": true,
                "volume_extensions": ["wolrar", "w02", "w03"],
                "recovery": "Reed-Solomon",
                "repair": true,
                "sfx_single_volume": true,
                "deduplication": true,
                "benchmark": true,
                "v1_encrypted_file_limit_mib": 512
            },
            "rar_create": false,
            "json": true,
            "agent_prompt": "wol-rar agent"
        })),
        Some(Command::List { archive }) => {
            list_result(&archive, password.as_ref().map(|value| value.as_str()))
        }
        Some(Command::Info { archive }) => {
            if wol_rar::ArchiveFormat::detect(&archive)? != wol_rar::ArchiveFormat::WolRar {
                return Err(wol_rar::ArchiveError::UnsupportedFormat(
                    "info richiede un archivio .wolrar".into(),
                ));
            }
            match wol_rar::wolformat::version(&archive)? {
                1 => {
                    let info = wol_rar::wolformat::inspect(&archive)?;
                    Ok(
                        json!({"ok": true, "operation": "info", "archive": archive, "format_version": 1, "comment": info.comment, "entry_count": info.entry_count, "encrypted": info.encrypted, "body_len": info.body_len, "volume_count": 1}),
                    )
                }
                2 => {
                    let info = wol_rar::wolformat_v2::inspect(&archive)?;
                    Ok(
                        json!({"ok": true, "operation": "info", "archive": archive, "format_version": 2, "comment": info.comment, "entry_count": info.entry_count, "encrypted": info.encrypted, "body_len": info.body_len, "volume_count": info.volume_count, "volume_size": info.volume_size, "recovery_count": wol_rar::wolrecovery::recovery_count(&archive)}),
                    )
                }
                other => Err(wol_rar::ArchiveError::UnsupportedFormat(format!(
                    "Versione .wolrar {other}"
                ))),
            }
        }
        Some(Command::Create {
            sources,
            output,
            level,
        }) => {
            let options = CreateOptions {
                level,
                password,
                solid: true,
                comment: args.comment.clone(),
                volume_size: args.volume_size,
                recovery_percent: args.recovery_percent,
                deduplicate: args.deduplicate,
            };
            RustArchiveEngine::create(&output, &sources, &options)?;
            Ok(json!({"ok": true, "operation": "create", "archive": output, "sources": sources}))
        }
        Some(Command::Extract {
            archive,
            destination,
        }) => {
            RustArchiveEngine::extract(
                &archive,
                &destination,
                password.as_ref().map(|value| value.as_str()),
            )?;
            Ok(
                json!({"ok": true, "operation": "extract", "archive": archive, "destination": destination}),
            )
        }
        Some(Command::Test { archive }) => {
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "test", "archive": archive}))
        }
        Some(Command::Benchmark {
            size_mib,
            suite,
            output,
        }) => {
            if suite {
                let report = wol_rar::benchmark::run_suite(size_mib)?;
                if let Some(output) = output {
                    let file = std::fs::File::create_new(&output)?;
                    serde_json::to_writer_pretty(file, &report)
                        .map_err(|error| wol_rar::ArchiveError::Message(error.to_string()))?;
                }
                Ok(json!({"ok": true, "operation": "benchmark-suite", "report": report}))
            } else {
                let report = RustArchiveEngine::benchmark(size_mib)?;
                Ok(json!({"ok": true, "operation": "benchmark", "report": report}))
            }
        }
        Some(Command::Sfx { archive, output }) => {
            let report = RustArchiveEngine::create_sfx(&archive, &output)?;
            Ok(json!({"ok": true, "operation": "sfx", "report": report}))
        }
        Some(Command::Repair {
            archive,
            destination,
        }) => {
            let report = RustArchiveEngine::repair(&archive, &destination)?;
            Ok(json!({"ok": true, "operation": "repair", "report": report}))
        }
        Some(Command::Delete {
            archive,
            entry,
            output,
        }) => {
            let report = wol_rar::edit::rewrite(
                &archive,
                output.as_deref(),
                wol_rar::edit::EditOperation::Delete { entry },
                make_edit_options(password.clone()),
                password,
            )?;
            Ok(json!({"ok": true, "operation": "delete", "report": report}))
        }
        Some(Command::Rename {
            archive,
            entry,
            to,
            output,
        }) => {
            let report = wol_rar::edit::rewrite(
                &archive,
                output.as_deref(),
                wol_rar::edit::EditOperation::Rename {
                    entry,
                    new_path: to,
                },
                make_edit_options(password.clone()),
                password,
            )?;
            Ok(json!({"ok": true, "operation": "rename", "report": report}))
        }
        Some(Command::Add {
            archive,
            source,
            entry,
            output,
        }) => {
            let report = wol_rar::edit::rewrite(
                &archive,
                output.as_deref(),
                wol_rar::edit::EditOperation::Add { source, entry },
                make_edit_options(password.clone()),
                password,
            )?;
            Ok(json!({"ok": true, "operation": "add", "report": report}))
        }
        Some(Command::Update {
            archive,
            source,
            entry,
            output,
        }) => {
            let report = wol_rar::edit::rewrite(
                &archive,
                output.as_deref(),
                wol_rar::edit::EditOperation::Update { source, entry },
                make_edit_options(password.clone()),
                password,
            )?;
            Ok(json!({"ok": true, "operation": "update", "report": report}))
        }
        Some(Command::Convert { archive, output }) => {
            let report = wol_rar::edit::rewrite(
                &archive,
                Some(&output),
                wol_rar::edit::EditOperation::Convert,
                make_edit_options(password.clone()),
                password,
            )?;
            Ok(json!({"ok": true, "operation": "convert", "report": report}))
        }
        Some(Command::UpdateCheck) => {
            let release = wol_rar::update::check_latest()?;
            let newer = wol_rar::update::is_newer(&release.tag_name)?;
            Ok(
                json!({"ok": true, "operation": "update-check", "release": release, "newer": newer, "current": env!("CARGO_PKG_VERSION")}),
            )
        }
        Some(Command::UpdateDownload) => {
            let download = wol_rar::update::download_latest()?;
            Ok(json!({"ok": true, "operation": "update-download", "download": download}))
        }
        Some(Command::UpdateInstall { asset }) => {
            let install = wol_rar::update::install_asset(&asset)?;
            Ok(json!({"ok": true, "operation": "update-install", "install": install}))
        }
        None if !args.json
            && args.open.is_none()
            && args.archive.is_none()
            && args.create.is_none()
            && args.extract.is_none()
            && args.test.is_none() =>
        {
            wol_rar::gui::run(wol_rar::gui::GuiAction::None, None)
                .map_err(wol_rar::ArchiveError::Message)?;
            Ok(json!({"ok": true, "gui": "eframe"}))
        }
        None if args.create.is_some() => {
            let source = args.create.unwrap();
            let output = args.output.ok_or_else(|| {
                wol_rar::ArchiveError::Message("--output obbligatorio con --create".into())
            })?;
            let options = CreateOptions {
                level: args.level,
                password,
                solid: true,
                comment: args.comment.clone(),
                volume_size: args.volume_size,
                recovery_percent: args.recovery_percent,
                deduplicate: args.deduplicate,
            };
            RustArchiveEngine::create(&output, std::slice::from_ref(&source), &options)?;
            Ok(json!({"ok": true, "operation": "create", "archive": output, "source": source}))
        }
        None if args.extract.is_some() => {
            let archive = args.extract.unwrap();
            let destination = args.destination.ok_or_else(|| {
                wol_rar::ArchiveError::Message("--destination obbligatorio con --extract".into())
            })?;
            RustArchiveEngine::extract(
                &archive,
                &destination,
                password.as_ref().map(|value| value.as_str()),
            )?;
            Ok(
                json!({"ok": true, "operation": "extract", "archive": archive, "destination": destination}),
            )
        }
        None if args.test.is_some() => {
            let archive = args.test.unwrap();
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "test", "archive": archive}))
        }
        None => {
            let archive = args.open.or(args.archive).ok_or_else(|| {
                wol_rar::ArchiveError::Message(
                    "Specificare un archivio o un comando; vedere --help".into(),
                )
            })?;
            list_result(&archive, password.as_ref().map(|value| value.as_str()))
        }
    }
}

fn list_result(
    archive: &std::path::Path,
    password: Option<&str>,
) -> Result<Value, wol_rar::ArchiveError> {
    let entries = RustArchiveEngine::list(archive, password)?;
    Ok(
        json!({"ok": true, "operation": "list", "archive": archive, "count": entries.len(), "entries": entries}),
    )
}

fn read_password(enabled: bool) -> Result<Option<Zeroizing<String>>, wol_rar::ArchiveError> {
    if !enabled {
        return Ok(None);
    }
    let mut password = Zeroizing::new(String::new());
    std::io::stdin().read_to_string(&mut password)?;
    Ok(Some(Zeroizing::new(
        password.trim_end_matches(['\r', '\n']).to_owned(),
    )))
}

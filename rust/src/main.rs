use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};
use wol_rar::{CreateOptions, RustArchiveEngine};
use zeroize::Zeroizing;

const AGENT_PROMPT: &str = include_str!("../../agent/AGENTS.snippet.md");

#[derive(Debug, Parser)]
#[command(name = "Wol-rar", version, about = "Archiviatore privato agent-first scritto in Rust")]
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
    json: bool,
    #[arg(long, global = true)]
    password_stdin: bool,
    #[arg(long)]
    gui: bool,
    #[arg(long, hide = true)]
    smoke_test: bool,
    #[arg(long, hide = true)]
    gui_smoke_test: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Elenca il contenuto di un archivio
    List { archive: PathBuf },
    /// Crea un archivio 7z, ZIP, TAR, TAR.GZ, TAR.BZ2 o TAR.XZ
    Create { source: PathBuf, #[arg(long)] output: PathBuf, #[arg(long, default_value_t = 5)] level: u32 },
    /// Estrai in una nuova cartella senza sovrascrivere
    Extract { archive: PathBuf, #[arg(long)] destination: PathBuf },
    /// Verifica integrità tramite decodifica completa
    Test { archive: PathBuf },
    /// Mostra formati e capacità del motore
    Formats,
    /// Controlla la release più recente su GitHub
    UpdateCheck,
    /// Scarica e verifica l'aggiornamento per questo sistema
    UpdateDownload { destination: PathBuf },
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
                eprintln!("{}", serde_json::to_string_pretty(&json!({"ok": false, "error": error.to_string()})).unwrap());
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
    if args.gui_smoke_test {
        wol_rar::gui::run(wol_rar::gui::GuiAction::None, Some(std::time::Instant::now() + std::time::Duration::from_secs(2)))
            .map_err(wol_rar::ArchiveError::Message)?;
        return Ok(json!({"ok": true, "gui": "eframe", "version": env!("CARGO_PKG_VERSION")}));
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
    match args.command {
        Some(Command::Agent) => Ok(Value::String(AGENT_PROMPT.into())),
        Some(Command::Formats) => Ok(json!({
            "ok": true,
            "create": ["7z", "zip", "tar", "tar.gz", "tar.bz2", "tar.xz"],
            "read": ["7z", "zip", "rar", "tar", "tar.gz", "tar.bz2", "tar.xz"],
            "rar_create": false,
            "json": true,
            "agent_prompt": "wol-rar agent"
        })),
        Some(Command::List { archive }) => list_result(&archive, password.as_ref().map(|value| value.as_str())),
        Some(Command::Create { source, output, level }) => {
            let options = CreateOptions { level, password, solid: true };
            RustArchiveEngine::create(&output, &[source.clone()], &options)?;
            Ok(json!({"ok": true, "operation": "create", "archive": output, "source": source}))
        }
        Some(Command::Extract { archive, destination }) => {
            RustArchiveEngine::extract(&archive, &destination, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "extract", "archive": archive, "destination": destination}))
        }
        Some(Command::Test { archive }) => {
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "test", "archive": archive}))
        }
        Some(Command::UpdateCheck) => {
            let release = wol_rar::update::check_latest()?;
            let newer = wol_rar::update::is_newer(&release.tag_name)?;
            Ok(json!({"ok": true, "operation": "update-check", "release": release, "newer": newer, "current": env!("CARGO_PKG_VERSION")}))
        }
        Some(Command::UpdateDownload { destination }) => {
            let download = wol_rar::update::download_latest(&destination)?;
            Ok(json!({"ok": true, "operation": "update-download", "download": download}))
        }
        None if !args.json && args.open.is_none() && args.archive.is_none() && args.create.is_none() && args.extract.is_none() && args.test.is_none() => {
            wol_rar::gui::run(wol_rar::gui::GuiAction::None, None).map_err(wol_rar::ArchiveError::Message)?;
            Ok(json!({"ok": true, "gui": "eframe"}))
        }
        None if args.create.is_some() => {
            let source = args.create.unwrap();
            let output = args.output.ok_or_else(|| wol_rar::ArchiveError::Message("--output obbligatorio con --create".into()))?;
            let options = CreateOptions { level: args.level, password, solid: true };
            RustArchiveEngine::create(&output, &[source.clone()], &options)?;
            Ok(json!({"ok": true, "operation": "create", "archive": output, "source": source}))
        }
        None if args.extract.is_some() => {
            let archive = args.extract.unwrap();
            let destination = args.destination.ok_or_else(|| wol_rar::ArchiveError::Message("--destination obbligatorio con --extract".into()))?;
            RustArchiveEngine::extract(&archive, &destination, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "extract", "archive": archive, "destination": destination}))
        }
        None if args.test.is_some() => {
            let archive = args.test.unwrap();
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))?;
            Ok(json!({"ok": true, "operation": "test", "archive": archive}))
        }
        None => {
            let archive = args.open.or(args.archive).ok_or_else(|| wol_rar::ArchiveError::Message("Specificare un archivio o un comando; vedere --help".into()))?;
            list_result(&archive, password.as_ref().map(|value| value.as_str()))
        }
    }
}

fn list_result(archive: &std::path::Path, password: Option<&str>) -> Result<Value, wol_rar::ArchiveError> {
    let entries = RustArchiveEngine::list(archive, password)?;
    Ok(json!({"ok": true, "operation": "list", "archive": archive, "count": entries.len(), "entries": entries}))
}

fn read_password(enabled: bool) -> Result<Option<Zeroizing<String>>, wol_rar::ArchiveError> {
    if !enabled { return Ok(None); }
    let mut password = Zeroizing::new(String::new());
    std::io::stdin().read_to_string(&mut password)?;
    Ok(Some(Zeroizing::new(password.trim_end_matches(['\r', '\n']).to_owned())))
}

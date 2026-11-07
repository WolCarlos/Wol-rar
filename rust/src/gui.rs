use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, ScrollArea};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::update::{ReleaseInfo, UpdateDownload};
use crate::{ArchiveEntry, CreateOptions, RustArchiveEngine};

const AGENT_PROMPT: &str = include_str!("../../agent/AGENTS.snippet.md");

#[derive(Clone)]
pub enum GuiAction {
    None,
    Open(PathBuf),
    Extract(PathBuf),
    Test(PathBuf),
    Create(PathBuf),
}

#[derive(Clone, Copy)]
enum WorkerKind { List, Operation, UpdateCheck, UpdateDownload }

struct WorkerResult {
    kind: WorkerKind,
    result: Result<Value, String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab { Archive, News, Agent }

pub fn run(action: GuiAction, close_after: Option<Instant>) -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([860.0, 560.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Wol-rar",
        options,
        Box::new(move |_| Ok(Box::new(WolRarApp::new(action, close_after)))),
    ).map_err(|error| error.to_string())
}

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../../assets/wol-rar.png"))
        .expect("icona Wol-rar non valida")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData { rgba: image.into_raw(), width, height }
}

struct WolRarApp {
    tab: Tab,
    archive: Option<PathBuf>,
    entries: Vec<ArchiveEntry>,
    filter: String,
    status: String,
    busy: bool,
    password: Zeroizing<String>,
    tx: Sender<WorkerResult>,
    rx: Receiver<WorkerResult>,
    pending_action: GuiAction,
    close_after: Option<Instant>,
    release_notes: String,
    newer_release: Option<ReleaseInfo>,
    last_download: Option<UpdateDownload>,
}

impl WolRarApp {
    fn new(action: GuiAction, close_after: Option<Instant>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tab: Tab::Archive,
            archive: None,
            entries: Vec::new(),
            filter: String::new(),
            status: "Pronto · rete solo per aggiornamenti su richiesta".into(),
            busy: false,
            password: Zeroizing::new(String::new()),
            tx,
            rx,
            pending_action: action,
            close_after,
            release_notes: local_notes(),
            newer_release: None,
            last_download: None,
        }
    }

    fn spawn(&mut self, kind: WorkerKind, operation: impl FnOnce() -> Result<Value, String> + Send + 'static) {
        if self.busy { return; }
        self.busy = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = operation();
            let _ = tx.send(WorkerResult { kind, result });
        });
    }

    fn password_value(&self) -> Option<Zeroizing<String>> {
        if self.password.is_empty() { None } else { Some(self.password.clone()) }
    }

    fn open_archive(&mut self, path: PathBuf) {
        let password = self.password_value();
        self.spawn(WorkerKind::List, move || {
            let entries = RustArchiveEngine::list(&path, password.as_ref().map(|value| value.as_str()))
                .map_err(|error| error.to_string())?;
            Ok(json!({"archive": path, "entries": entries}))
        });
    }

    fn choose_archive(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_archive(path);
        }
    }

    fn create_archive(&mut self) {
        let mut sources = rfd::FileDialog::new().pick_files().unwrap_or_default();
        if sources.is_empty() {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() { sources.push(folder); }
        }
        if sources.is_empty() { return; }
        let Some(output) = rfd::FileDialog::new().set_file_name("Nuovo archivio.7z").save_file() else { return; };
        let password = self.password_value();
        self.spawn(WorkerKind::Operation, move || {
            let options = CreateOptions { level: 5, password, solid: true };
            RustArchiveEngine::create(&output, &sources, &options).map_err(|error| error.to_string())?;
            Ok(json!({"operation": "create", "archive": output}))
        });
    }

    fn extract_archive(&mut self) {
        let Some(archive) = self.archive.clone() else { self.status = "Apri prima un archivio".into(); return; };
        let Some(destination) = rfd::FileDialog::new().pick_folder() else { return; };
        let password = self.password_value();
        self.spawn(WorkerKind::Operation, move || {
            RustArchiveEngine::extract(&archive, &destination, password.as_ref().map(|value| value.as_str()))
                .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "extract", "destination": destination}))
        });
    }

    fn test_archive(&mut self) {
        let Some(archive) = self.archive.clone() else { self.status = "Apri prima un archivio".into(); return; };
        let password = self.password_value();
        self.spawn(WorkerKind::Operation, move || {
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))
                .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "test", "archive": archive}))
        });
    }

    fn check_updates(&mut self) {
        self.spawn(WorkerKind::UpdateCheck, || {
            let release = crate::update::check_latest().map_err(|error| error.to_string())?;
            let newer = crate::update::is_newer(&release.tag_name).map_err(|error| error.to_string())?;
            Ok(json!({"release": release, "newer": newer}))
        });
    }

    fn download_update(&mut self) {
        let Some(destination) = rfd::FileDialog::new().pick_folder() else { return; };
        self.spawn(WorkerKind::UpdateDownload, move || {
            let download = crate::update::download_latest(&destination).map_err(|error| error.to_string())?;
            Ok(json!(download))
        });
    }

    fn process_messages(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            self.busy = false;
            match message.result {
                Err(error) => self.status = format!("Operazione non riuscita: {error}"),
                Ok(value) => match message.kind {
                    WorkerKind::List => {
                        self.archive = value.get("archive").and_then(Value::as_str).map(PathBuf::from);
                        self.entries = serde_json::from_value(value.get("entries").cloned().unwrap_or_default()).unwrap_or_default();
                        self.status = format!("{} voci · archivio aperto in sola lettura", self.entries.len());
                    }
                    WorkerKind::Operation => {
                        self.status = value.get("operation").and_then(Value::as_str).unwrap_or("Operazione completata").to_owned();
                    }
                    WorkerKind::UpdateCheck => {
                        if let Ok(release) = serde_json::from_value::<ReleaseInfo>(value.get("release").cloned().unwrap_or_default()) {
                            self.release_notes = format!("ULTIMA RELEASE {}\n\n{}\n\n{}", release.tag_name, release.body, local_notes());
                            self.newer_release = Some(release);
                        }
                        self.status = if value.get("newer").and_then(Value::as_bool).unwrap_or(false) {
                            "Nuova versione disponibile".into()
                        } else {
                            "Wol-rar è aggiornato".into()
                        };
                    }
                    WorkerKind::UpdateDownload => {
                        if let Ok(download) = serde_json::from_value::<UpdateDownload>(value) {
                            self.status = format!("Aggiornamento {} scaricato e verificato", download.version);
                            self.last_download = Some(download);
                        }
                    }
                }
            }
        }
    }

    fn handle_pending_action(&mut self) {
        let action = self.pending_action.clone();
        self.pending_action = GuiAction::None;
        match action {
            GuiAction::None => {}
            GuiAction::Open(path) => self.open_archive(path),
            GuiAction::Test(path) => {
                self.open_archive(path);
                self.status = "Archivio aperto; usa Verifica per il test completo".into();
            }
            GuiAction::Extract(path) => {
                let destination = suggested_extract_directory(&path);
                self.spawn(WorkerKind::Operation, move || {
                    RustArchiveEngine::extract(&path, &destination, None).map_err(|error| error.to_string())?;
                    Ok(json!({"operation": "extract", "destination": destination}))
                });
            }
            GuiAction::Create(path) => {
                let output = suggested_archive_path(&path);
                self.spawn(WorkerKind::Operation, move || {
                    RustArchiveEngine::create(&output, &[path], &CreateOptions::default()).map_err(|error| error.to_string())?;
                    Ok(json!({"operation": "create", "archive": output}))
                });
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui.button("📂  Apri").clicked() { self.choose_archive(); }
            if ui.button("＋  Nuovo archivio").clicked() { self.create_archive(); }
            if ui.button("⇧  Estrai").clicked() { self.extract_archive(); }
            if ui.button("✓  Verifica").clicked() { self.test_archive(); }
            if ui.button("⟳  Aggiornamenti").clicked() { self.tab = Tab::News; }
            ui.separator();
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut *self.password).password(true).desired_width(130.0).hint_text("solo se richiesta"));
        });
    }

    fn archive_page(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.heading(self.archive.as_ref().and_then(|path| path.file_name()).and_then(|name| name.to_str()).unwrap_or("Apri un archivio o creane uno nuovo"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(format!("{} voci", self.entries.len())).color(Color32::from_rgb(82, 96, 113)));
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Filtra contenuto");
            ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(f32::INFINITY).hint_text("Nome o percorso…"));
        });
        ui.add_space(8.0);
        let query = self.filter.to_lowercase();
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            egui::Grid::new("archive_entries").num_columns(4).striped(true).min_col_width(90.0).show(ui, |ui| {
                ui.strong("Nome");
                ui.strong("Dimensione");
                ui.strong("Compressa");
                ui.strong("Modificato");
                ui.end_row();
                for entry in self.entries.iter().filter(|entry| query.is_empty() || entry.path.to_lowercase().contains(&query)) {
                    ui.label(&entry.path);
                    ui.label(format_size(entry.size));
                    ui.label(format_size(entry.packed_size));
                    ui.label(&entry.modified);
                    ui.end_row();
                }
            });
        });
    }

    fn news_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Novità e aggiornamenti");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Controlla aggiornamenti").clicked() { self.check_updates(); }
                if self.newer_release.is_some() && ui.button("Scarica e verifica").clicked() { self.download_update(); }
            });
        });
        ui.label("Nessuna connessione automatica. Il download viene salvato solo dopo verifica SHA-256.");
        ui.add_space(12.0);
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add(egui::Label::new(self.release_notes.clone()).wrap());
        });
    }

    fn agent_page(&self, ui: &mut egui::Ui) {
        ui.heading("Agenti AI e CLI");
        ui.label("Tutte le operazioni hanno una CLI equivalente. Copia questo prompt nel file AGENTS.md del tuo agente.");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Copia prompt").clicked() { ui.ctx().copy_text(AGENT_PROMPT.into()); }
            ui.code("wol-rar agent");
            ui.code("wol-rar formats --json");
        });
        ui.add_space(8.0);
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(AGENT_PROMPT).monospace()).wrap());
        });
    }
}

impl eframe::App for WolRarApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_messages();
        if matches!(self.pending_action, GuiAction::None) == false { self.handle_pending_action(); }
        if self.close_after.is_some_and(|when| Instant::now() >= when) {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Wol-rar").size(28.0).color(Color32::from_rgb(23, 63, 53)));
            ui.label(format!("v{} · archivi privati, semplici e locali", env!("CARGO_PKG_VERSION")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("● NESSUNA TELEMETRIA").color(Color32::from_rgb(25, 112, 82)));
            });
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Archive, "Archivio");
            ui.selectable_value(&mut self.tab, Tab::News, "Novità e aggiornamenti");
            ui.selectable_value(&mut self.tab, Tab::Agent, "Agenti AI");
        });
        ui.add_space(12.0);
        match self.tab {
            Tab::Archive => self.archive_page(ui),
            Tab::News => self.news_page(ui),
            Tab::Agent => self.agent_page(ui),
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.horizontal(|ui| {
                if self.busy { ui.spinner(); }
                ui.label(&self.status);
            });
        });
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

fn local_notes() -> String {
    include_str!("../../RELEASE_NOTES.md").into()
}

fn format_size(value: u64) -> String {
    let mut amount = value as f64;
    for unit in ["B", "KB", "MB", "GB", "TB"] {
        if amount < 1024.0 || unit == "TB" {
            return if unit == "B" { format!("{amount:.0} {unit}") } else { format!("{amount:.1} {unit}") };
        }
        amount /= 1024.0;
    }
    format!("{value} B")
}

fn suggested_archive_path(source: &Path) -> PathBuf {
    unique_path(source.with_extension("7z"))
}

fn suggested_extract_directory(archive: &Path) -> PathBuf {
    unique_path(archive.with_extension(""))
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() { return path; }
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("archivio");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..1000 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = path.with_file_name(name);
        if !candidate.exists() { return candidate; }
    }
    path
}

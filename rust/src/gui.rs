use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, TextureHandle, Vec2};
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
enum WorkerKind {
    List,
    Operation,
    UpdateCheck,
    UpdateDownload,
}

struct WorkerResult {
    kind: WorkerKind,
    result: Result<Value, String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Archive,
    News,
    Agent,
}

#[derive(Clone, Copy, PartialEq)]
enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy)]
struct Palette {
    background: Color32,
    surface: Color32,
    elevated: Color32,
    border: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    accent_soft: Color32,
    success: Color32,
}

impl Palette {
    fn dark() -> Self {
        Self {
            background: Color32::from_rgb(10, 15, 22),
            surface: Color32::from_rgb(18, 26, 37),
            elevated: Color32::from_rgb(25, 36, 50),
            border: Color32::from_rgb(43, 59, 78),
            text: Color32::from_rgb(232, 240, 248),
            muted: Color32::from_rgb(145, 163, 184),
            accent: Color32::from_rgb(74, 214, 165),
            accent_soft: Color32::from_rgb(25, 67, 59),
            success: Color32::from_rgb(83, 220, 157),
        }
    }

    fn light() -> Self {
        Self {
            background: Color32::from_rgb(242, 246, 250),
            surface: Color32::WHITE,
            elevated: Color32::from_rgb(232, 239, 246),
            border: Color32::from_rgb(207, 220, 232),
            text: Color32::from_rgb(24, 36, 51),
            muted: Color32::from_rgb(88, 105, 124),
            accent: Color32::from_rgb(0, 128, 91),
            accent_soft: Color32::from_rgb(211, 241, 229),
            success: Color32::from_rgb(0, 137, 86),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum SortColumn {
    Name,
    Size,
    Packed,
    Ratio,
    Modified,
}

struct Icons {
    logo: TextureHandle,
    open: TextureHandle,
    create: TextureHandle,
    extract: TextureHandle,
    test: TextureHandle,
    update: TextureHandle,
    info: TextureHandle,
}

impl Icons {
    fn new(context: &egui::Context) -> Self {
        Self {
            logo: texture(context, "logo", include_bytes!("../../assets/wol-rar.png")),
            open: texture(
                context,
                "open",
                include_bytes!("../../assets/menu-open.png"),
            ),
            create: texture(
                context,
                "create",
                include_bytes!("../../assets/menu-create.png"),
            ),
            extract: texture(
                context,
                "extract",
                include_bytes!("../../assets/menu-extract.png"),
            ),
            test: texture(
                context,
                "test",
                include_bytes!("../../assets/menu-test.png"),
            ),
            update: texture(
                context,
                "update",
                include_bytes!("../../assets/menu-update.png"),
            ),
            info: texture(
                context,
                "info",
                include_bytes!("../../assets/menu-info.png"),
            ),
        }
    }
}

pub fn run(action: GuiAction, close_after: Option<Instant>) -> Result<(), String> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 780.0])
            .with_min_inner_size([940.0, 600.0])
            .with_icon(load_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Wol-rar",
        options,
        Box::new(move |context| {
            Ok(Box::new(WolRarApp::new(
                action,
                close_after,
                &context.egui_ctx,
            )))
        }),
    )
    .map_err(|error| error.to_string())
}

fn texture(context: &egui::Context, name: &str, bytes: &[u8]) -> TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("icona Wol-rar non valida")
        .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    context.load_texture(name, image, egui::TextureOptions::LINEAR)
}

fn load_icon() -> egui::IconData {
    let image = image::load_from_memory(include_bytes!("../../assets/wol-rar.png"))
        .expect("icona Wol-rar non valida")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

struct WolRarApp {
    tab: Tab,
    theme: ThemeMode,
    icons: Icons,
    archive: Option<PathBuf>,
    entries: Vec<ArchiveEntry>,
    filter: String,
    comment: String,
    archive_comment: String,
    compression_level: u32,
    volume_size_mib: u64,
    recovery_percent: u8,
    deduplicate: bool,
    sort_column: SortColumn,
    sort_ascending: bool,
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
    fn new(action: GuiAction, close_after: Option<Instant>, context: &egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        let theme = ThemeMode::Dark;
        apply_theme(context, theme);
        Self {
            tab: Tab::Archive,
            theme,
            icons: Icons::new(context),
            archive: None,
            entries: Vec::new(),
            filter: String::new(),
            comment: String::new(),
            archive_comment: String::new(),
            compression_level: 5,
            volume_size_mib: 0,
            recovery_percent: 0,
            deduplicate: false,
            sort_column: SortColumn::Name,
            sort_ascending: true,
            status: "Pronto · operazioni locali, rete solo su richiesta".into(),
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

    fn palette(&self) -> Palette {
        match self.theme {
            ThemeMode::Dark => Palette::dark(),
            ThemeMode::Light => Palette::light(),
        }
    }

    fn spawn(
        &mut self,
        kind: WorkerKind,
        operation: impl FnOnce() -> Result<Value, String> + Send + 'static,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let result = operation();
            let _ = tx.send(WorkerResult { kind, result });
        });
    }

    fn password_value(&self) -> Option<Zeroizing<String>> {
        if self.password.is_empty() {
            None
        } else {
            Some(self.password.clone())
        }
    }

    fn open_archive(&mut self, path: PathBuf) {
        let password = self.password_value();
        self.spawn(WorkerKind::List, move || {
            let entries =
                RustArchiveEngine::list(&path, password.as_ref().map(|value| value.as_str()))
                    .map_err(|error| error.to_string())?;
            let comment =
                if crate::ArchiveFormat::detect(&path).ok() == Some(crate::ArchiveFormat::WolRar) {
                    crate::wolformat::comment(&path).map_err(|error| error.to_string())?
                } else {
                    String::new()
                };
            Ok(json!({"archive": path, "entries": entries, "comment": comment}))
        });
    }

    fn choose_archive(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Archivi supportati",
                &["wolrar", "7z", "zip", "rar", "tar", "gz", "bz2", "xz"],
            )
            .pick_file()
        {
            self.open_archive(path);
        }
    }

    fn create_archive(&mut self) {
        let mut sources = rfd::FileDialog::new().pick_files().unwrap_or_default();
        if sources.is_empty()
            && let Some(folder) = rfd::FileDialog::new().pick_folder()
        {
            sources.push(folder);
        }
        if sources.is_empty() {
            return;
        }
        let Some(output) = rfd::FileDialog::new()
            .add_filter("Archivio Wol-rar", &["wolrar"])
            .add_filter("Archivio 7z", &["7z"])
            .add_filter("Archivio ZIP", &["zip"])
            .set_file_name("Nuovo archivio.wolrar")
            .save_file()
        else {
            return;
        };
        let password = self.password_value();
        let comment = if self.comment.is_empty() {
            None
        } else {
            Some(self.comment.clone())
        };
        let level = self.compression_level;
        let volume_size = (self.volume_size_mib > 0).then_some(self.volume_size_mib * 1024 * 1024);
        let recovery_percent = self.recovery_percent;
        let deduplicate = self.deduplicate;
        self.spawn(WorkerKind::Operation, move || {
            let options = CreateOptions {
                level,
                password,
                solid: true,
                comment,
                volume_size,
                recovery_percent,
                deduplicate,
            };
            RustArchiveEngine::create(&output, &sources, &options)
                .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "create", "archive": output}))
        });
    }

    fn extract_archive(&mut self) {
        let Some(archive) = self.archive.clone() else {
            self.status = "Apri prima un archivio".into();
            return;
        };
        let Some(destination) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let password = self.password_value();
        self.spawn(WorkerKind::Operation, move || {
            RustArchiveEngine::extract(
                &archive,
                &destination,
                password.as_ref().map(|value| value.as_str()),
            )
            .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "extract", "destination": destination}))
        });
    }

    fn test_archive(&mut self) {
        let Some(archive) = self.archive.clone() else {
            self.status = "Apri prima un archivio".into();
            return;
        };
        let password = self.password_value();
        self.spawn(WorkerKind::Operation, move || {
            RustArchiveEngine::test(&archive, password.as_ref().map(|value| value.as_str()))
                .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "test", "archive": archive}))
        });
    }

    fn repair_archive(&mut self) {
        let Some(archive) = self.archive.clone() else {
            self.status = "Apri prima un archivio .wolrar v2".into();
            return;
        };
        let Some(destination) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        self.spawn(WorkerKind::Operation, move || {
            let report = RustArchiveEngine::repair(&archive, &destination)
                .map_err(|error| error.to_string())?;
            Ok(json!({"operation": "repair", "report": report}))
        });
    }

    fn run_benchmark(&mut self) {
        self.spawn(WorkerKind::Operation, || {
            let report = RustArchiveEngine::benchmark(32).map_err(|error| error.to_string())?;
            Ok(json!({"operation": "benchmark", "report": report}))
        });
    }

    fn check_updates(&mut self) {
        self.spawn(WorkerKind::UpdateCheck, || {
            let release = crate::update::check_latest().map_err(|error| error.to_string())?;
            let newer =
                crate::update::is_newer(&release.tag_name).map_err(|error| error.to_string())?;
            Ok(json!({"release": release, "newer": newer}))
        });
    }

    fn download_update(&mut self) {
        let Some(destination) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        self.spawn(WorkerKind::UpdateDownload, move || {
            let download =
                crate::update::download_latest(&destination).map_err(|error| error.to_string())?;
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
                        self.archive = value
                            .get("archive")
                            .and_then(Value::as_str)
                            .map(PathBuf::from);
                        self.entries = serde_json::from_value(
                            value.get("entries").cloned().unwrap_or_default(),
                        )
                        .unwrap_or_default();
                        self.archive_comment = value
                            .get("comment")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned();
                        self.status = format!(
                            "{} voci · archivio aperto in sola lettura",
                            self.entries.len()
                        );
                    }
                    WorkerKind::Operation => {
                        self.status = match value
                            .get("operation")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                        {
                            "create" => "Archivio creato e pronto".into(),
                            "extract" => "Estrazione completata".into(),
                            "test" => "Integrità verificata: nessun errore".into(),
                            "repair" => "Volumi ricostruiti e manifest verificato".into(),
                            "benchmark" => {
                                let report = value.get("report").cloned().unwrap_or_default();
                                format!(
                                    "Benchmark: {:.1} MiB/s creazione · {:.1} MiB/s estrazione",
                                    report
                                        .get("create_mib_per_second")
                                        .and_then(Value::as_f64)
                                        .unwrap_or(0.0),
                                    report
                                        .get("extract_mib_per_second")
                                        .and_then(Value::as_f64)
                                        .unwrap_or(0.0),
                                )
                            }
                            _ => "Operazione completata".into(),
                        };
                    }
                    WorkerKind::UpdateCheck => {
                        if let Ok(release) = serde_json::from_value::<ReleaseInfo>(
                            value.get("release").cloned().unwrap_or_default(),
                        ) {
                            self.release_notes = format!(
                                "ULTIMA RELEASE {}\n\n{}\n\n{}",
                                release.tag_name,
                                release.body,
                                local_notes()
                            );
                            self.newer_release = Some(release);
                        }
                        self.status =
                            if value.get("newer").and_then(Value::as_bool).unwrap_or(false) {
                                "Nuova versione disponibile".into()
                            } else {
                                "Wol-rar è aggiornato".into()
                            };
                    }
                    WorkerKind::UpdateDownload => {
                        if let Ok(download) = serde_json::from_value::<UpdateDownload>(value) {
                            self.status = format!(
                                "Aggiornamento {} scaricato e verificato",
                                download.version
                            );
                            self.last_download = Some(download);
                        }
                    }
                },
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
                self.status = "Archivio aperto; usa Verifica o premi F5".into();
            }
            GuiAction::Extract(path) => {
                let destination = suggested_extract_directory(&path);
                self.spawn(WorkerKind::Operation, move || {
                    RustArchiveEngine::extract(&path, &destination, None)
                        .map_err(|error| error.to_string())?;
                    Ok(json!({"operation": "extract", "destination": destination}))
                });
            }
            GuiAction::Create(path) => {
                let output = suggested_archive_path(&path);
                self.spawn(WorkerKind::Operation, move || {
                    RustArchiveEngine::create(&output, &[path], &CreateOptions::default())
                        .map_err(|error| error.to_string())?;
                    Ok(json!({"operation": "create", "archive": output}))
                });
            }
        }
    }

    fn handle_shortcuts_and_drop(&mut self, context: &egui::Context) {
        if self.busy {
            return;
        }
        if context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::O)) {
            self.choose_archive();
        }
        if context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::N)) {
            self.create_archive();
        }
        if context.input(|input| input.modifiers.command && input.key_pressed(egui::Key::E)) {
            self.extract_archive();
        }
        if context.input(|input| input.key_pressed(egui::Key::F5)) {
            self.test_archive();
        }
        let dropped = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter(|file| !file.path().as_os_str().is_empty())
                .map(|file| file.path().to_path_buf())
                .collect::<Vec<_>>()
        });
        if dropped.len() == 1 {
            self.open_archive(dropped[0].clone());
        } else if dropped.len() > 1 {
            self.status = "Trascina un solo archivio alla volta".into();
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.icons.logo).fit_to_exact_size(Vec2::new(44.0, 44.0)));
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Wol-rar")
                            .size(26.0)
                            .strong()
                            .color(palette.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Archivi privati, locali e agent-first · v{}",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .color(palette.muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = match self.theme {
                        ThemeMode::Dark => "Tema chiaro",
                        ThemeMode::Light => "Tema scuro",
                    };
                    if ui
                        .add(egui::Button::new(label).min_size(Vec2::new(108.0, 32.0)))
                        .clicked()
                    {
                        self.theme = match self.theme {
                            ThemeMode::Dark => ThemeMode::Light,
                            ThemeMode::Light => ThemeMode::Dark,
                        };
                        apply_theme(ui.ctx(), self.theme);
                    }
                    ui.add_space(12.0);
                    badge(
                        ui,
                        "●  NESSUNA TELEMETRIA",
                        palette.accent_soft,
                        palette.success,
                    );
                });
            });
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Archive, "  Archivio  ");
                ui.selectable_value(&mut self.tab, Tab::News, "  Novità e aggiornamenti  ");
                ui.selectable_value(&mut self.tab, Tab::Agent, "  Agenti AI  ");
            });
        });
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, &self.icons.open, "Apri", "Ctrl+O", !self.busy).clicked() {
                    self.choose_archive();
                }
                if action_button(
                    ui,
                    &self.icons.create,
                    "Nuovo archivio",
                    "Ctrl+N",
                    !self.busy,
                )
                .clicked()
                {
                    self.create_archive();
                }
                let has_archive = self.archive.is_some() && !self.busy;
                if action_button(ui, &self.icons.extract, "Estrai", "Ctrl+E", has_archive).clicked()
                {
                    self.extract_archive();
                }
                if action_button(ui, &self.icons.test, "Verifica", "F5", has_archive).clicked() {
                    self.test_archive();
                }
                if action_button(ui, &self.icons.test, "Ripara", "", has_archive).clicked() {
                    self.repair_archive();
                }
                if action_button(ui, &self.icons.update, "Aggiornamenti", "", !self.busy).clicked()
                {
                    self.tab = Tab::News;
                }
            });
        });
    }

    fn options(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            egui::CollapsingHeader::new(
                RichText::new("Opzioni di creazione e sicurezza").color(palette.text),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label("Compressione");
                    ui.add(egui::Slider::new(&mut self.compression_level, 0..=9).text("livello"));
                    ui.separator();
                    ui.label("Volume");
                    ui.add(
                        egui::Slider::new(&mut self.volume_size_mib, 0..=4096)
                            .text("MiB (0 = unico)"),
                    );
                    ui.separator();
                    ui.label("Recovery");
                    ui.add(egui::Slider::new(&mut self.recovery_percent, 0..=100).text("% parity"));
                    ui.separator();
                    ui.label("Password");
                    ui.add(
                        egui::TextEdit::singleline(&mut *self.password)
                            .password(true)
                            .desired_width(170.0)
                            .hint_text("mai salvata"),
                    );
                    if !self.password.is_empty() && ui.small_button("Cancella").clicked() {
                        self.password = Zeroizing::new(String::new());
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Commento .wolrar");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.comment)
                            .desired_width(f32::INFINITY)
                            .hint_text("Facoltativo; nomi e commenti non sono segreti"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.deduplicate, "Deduplica contenuti identici (v2)");
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Esegui benchmark 32 MiB"))
                        .clicked()
                    {
                        self.run_benchmark();
                    }
                });
            });
        });
    }

    fn archive_page(&mut self, ui: &mut egui::Ui) {
        self.toolbar(ui);
        ui.add_space(10.0);
        self.options(ui);
        ui.add_space(10.0);
        if self.entries.is_empty() {
            self.empty_archive(ui);
            return;
        }
        self.summary(ui);
        ui.add_space(10.0);
        self.entries_table(ui);
    }

    fn empty_archive(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(34.0);
                ui.add(egui::Image::new(&self.icons.logo).fit_to_exact_size(Vec2::new(76.0, 76.0)));
                ui.add_space(12.0);
                ui.label(RichText::new("Il tuo spazio di archiviazione privato").size(22.0).strong().color(palette.text));
                ui.add_space(6.0);
                ui.label(RichText::new("Apri un archivio oppure trascinalo qui. Nessun contenuto lascia il dispositivo.").color(palette.muted));
                ui.add_space(16.0);
                if action_button(ui, &self.icons.open, "Scegli un archivio", "Ctrl+O", !self.busy).clicked() {
                    self.choose_archive();
                }
                ui.add_space(30.0);
            });
        });
    }

    fn summary(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        let total = self
            .entries
            .iter()
            .filter(|entry| !entry.directory)
            .map(|entry| entry.size)
            .sum::<u64>();
        let packed = self
            .entries
            .iter()
            .filter(|entry| !entry.directory)
            .map(|entry| entry.packed_size)
            .sum::<u64>();
        let ratio = if total == 0 {
            0.0
        } else {
            100.0 - packed as f64 * 100.0 / total as f64
        };
        card(ui, palette, |ui| {
            ui.horizontal_wrapped(|ui| {
                metric(ui, "VOCI", self.entries.len().to_string(), palette);
                metric(ui, "DIMENSIONE", format_size(total), palette);
                metric(ui, "COMPRESSA", format_size(packed), palette);
                metric(ui, "RISPARMIO", format!("{ratio:.1}%"), palette);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Sola lettura").color(palette.accent));
                });
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(
                        self.archive
                            .as_ref()
                            .and_then(|path| path.file_name())
                            .and_then(|name| name.to_str())
                            .unwrap_or("Archivio"),
                    )
                    .strong()
                    .color(palette.text),
                );
                if !self.archive_comment.is_empty() {
                    ui.separator();
                    ui.add(
                        egui::Label::new(
                            RichText::new(format!("Commento: {}", self.archive_comment))
                                .color(palette.muted),
                        )
                        .truncate(),
                    );
                }
            });
        });
    }

    fn sort_button(&mut self, ui: &mut egui::Ui, column: SortColumn, label: &str) {
        let marker = if self.sort_column == column {
            if self.sort_ascending { " ↑" } else { " ↓" }
        } else {
            ""
        };
        if ui.button(format!("{label}{marker}")).clicked() {
            if self.sort_column == column {
                self.sort_ascending = !self.sort_ascending;
            } else {
                self.sort_column = column;
                self.sort_ascending = true;
            }
        }
    }

    fn entries_table(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.icons.info).fit_to_exact_size(Vec2::new(20.0, 20.0)));
                ui.add(
                    egui::TextEdit::singleline(&mut self.filter)
                        .desired_width(f32::INFINITY)
                        .hint_text("Cerca per nome o percorso…"),
                );
            });
            ui.add_space(10.0);
            let query = self.filter.to_lowercase();
            let mut entries = self
                .entries
                .iter()
                .filter(|entry| query.is_empty() || entry.path.to_lowercase().contains(&query))
                .cloned()
                .collect::<Vec<_>>();
            let column = self.sort_column;
            let ascending = self.sort_ascending;
            entries.sort_by(|left, right| {
                let ordering = match column {
                    SortColumn::Name => left.path.to_lowercase().cmp(&right.path.to_lowercase()),
                    SortColumn::Size => left.size.cmp(&right.size),
                    SortColumn::Packed => left.packed_size.cmp(&right.packed_size),
                    SortColumn::Ratio => {
                        compression_ratio(left).total_cmp(&compression_ratio(right))
                    }
                    SortColumn::Modified => left.modified.cmp(&right.modified),
                };
                if ascending {
                    ordering
                } else {
                    ordering.reverse()
                }
            });
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    egui::Grid::new("archive_entries")
                        .num_columns(5)
                        .striped(true)
                        .min_col_width(86.0)
                        .show(ui, |ui| {
                            self.sort_button(ui, SortColumn::Name, "Nome");
                            self.sort_button(ui, SortColumn::Size, "Dimensione");
                            self.sort_button(ui, SortColumn::Packed, "Compressa");
                            self.sort_button(ui, SortColumn::Ratio, "Rapporto");
                            self.sort_button(ui, SortColumn::Modified, "Modificato");
                            ui.end_row();
                            for entry in entries {
                                let name = if entry.directory {
                                    format!("{}/", entry.path)
                                } else {
                                    entry.path.clone()
                                };
                                let response =
                                    ui.add(egui::Label::new(name).sense(egui::Sense::click()));
                                if response.clicked() {
                                    self.status = format!("Selezionato: {}", entry.path);
                                }
                                ui.label(format_size(entry.size));
                                ui.label(format_size(entry.packed_size));
                                ui.label(if entry.directory {
                                    "—".into()
                                } else {
                                    format!("{:.0}%", compression_ratio(&entry))
                                });
                                ui.label(if entry.modified == "0" {
                                    "—"
                                } else {
                                    entry.modified.as_str()
                                });
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn news_page(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::Image::new(&self.icons.update).fit_to_exact_size(Vec2::new(30.0, 30.0)),
                );
                ui.label(
                    RichText::new("Novità e aggiornamenti")
                        .size(22.0)
                        .strong()
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(!self.busy, egui::Button::new("Controlla aggiornamenti"))
                        .clicked()
                    {
                        self.check_updates();
                    }
                    if self.newer_release.is_some()
                        && ui
                            .add_enabled(!self.busy, egui::Button::new("Scarica e verifica"))
                            .clicked()
                    {
                        self.download_update();
                    }
                });
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Nessuna connessione automatica. Download HTTPS solo su richiesta, con SHA-256 obbligatorio.").color(palette.muted));
        });
        ui.add_space(10.0);
        card(ui, palette, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(egui::Label::new(self.release_notes.clone()).wrap());
                });
        });
    }

    fn agent_page(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.label(
                RichText::new("Controllo tramite agenti AI e CLI")
                    .size(22.0)
                    .strong()
                    .color(palette.text),
            );
            ui.add_space(6.0);
            ui.label(RichText::new("Ogni operazione ha una CLI equivalente con output JSON. Copia il prompt nel file AGENTS.md del tuo agente.").color(palette.muted));
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copia prompt").clicked() {
                    ui.ctx().copy_text(AGENT_PROMPT.into());
                    self.status = "Prompt agente copiato".into();
                }
                ui.code("wol-rar agent");
                ui.code("wol-rar formats --json");
            });
        });
        ui.add_space(10.0);
        card(ui, palette, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add(egui::Label::new(RichText::new(AGENT_PROMPT).monospace()).wrap());
                });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette();
        card(ui, palette, |ui| {
            ui.horizontal(|ui| {
                if self.busy {
                    ui.spinner();
                } else {
                    ui.label(RichText::new("●").color(palette.success));
                }
                ui.label(RichText::new(&self.status).color(palette.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Offline per default").color(palette.accent));
                });
            });
        });
    }
}

impl eframe::App for WolRarApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.process_messages();
        if !matches!(self.pending_action, GuiAction::None) {
            self.handle_pending_action();
        }
        self.handle_shortcuts_and_drop(ui.ctx());
        if self.close_after.is_some_and(|when| Instant::now() >= when) {
            ui.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(self.palette().background)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| self.header(ui));
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(self.palette().background)
                    .inner_margin(egui::Margin::symmetric(14, 6)),
            )
            .show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(self.palette().background)
                    .inner_margin(egui::Margin::symmetric(14, 4)),
            )
            .show(ui, |ui| match self.tab {
                Tab::Archive => self.archive_page(ui),
                Tab::News => self.news_page(ui),
                Tab::Agent => self.agent_page(ui),
            });
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}

fn apply_theme(context: &egui::Context, mode: ThemeMode) {
    let palette = match mode {
        ThemeMode::Dark => Palette::dark(),
        ThemeMode::Light => Palette::light(),
    };
    let theme = match mode {
        ThemeMode::Dark => egui::Theme::Dark,
        ThemeMode::Light => egui::Theme::Light,
    };
    let mut style = (*context.style_of(theme)).clone();
    style.visuals = match mode {
        ThemeMode::Dark => egui::Visuals::dark(),
        ThemeMode::Light => egui::Visuals::light(),
    };
    style.visuals.dark_mode = mode == ThemeMode::Dark;
    style.visuals.panel_fill = palette.background;
    style.visuals.window_fill = palette.surface;
    style.visuals.extreme_bg_color = palette.elevated;
    style.visuals.faint_bg_color = palette.elevated;
    style.visuals.text_edit_bg_color = Some(palette.elevated);
    style.visuals.code_bg_color = palette.elevated;
    style.visuals.hyperlink_color = palette.accent;
    style.visuals.striped = true;
    style.visuals.window_corner_radius = egui::CornerRadius::same(14);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
    style.visuals.widgets.noninteractive.bg_fill = palette.surface;
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    style.visuals.widgets.inactive.bg_fill = palette.elevated;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    style.visuals.widgets.hovered.bg_fill = palette.accent_soft;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.accent);
    style.visuals.widgets.active.bg_fill = palette.accent_soft;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent);
    style.visuals.selection.bg_fill = palette.accent_soft;
    style.visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.interact_size = Vec2::new(42.0, 34.0);
    context.set_style_of(theme, style);
    context.set_theme(theme);
}

fn card(ui: &mut egui::Ui, palette: Palette, content: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.style_mut().visuals.widgets.noninteractive.fg_stroke =
                Stroke::new(1.0, palette.text);
            content(ui);
        });
}

fn badge(ui: &mut egui::Ui, text: &str, fill: Color32, color: Color32) {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(20))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(color));
        });
}

fn action_button(
    ui: &mut egui::Ui,
    icon: &TextureHandle,
    text: &str,
    shortcut: &str,
    enabled: bool,
) -> egui::Response {
    let image = egui::Image::new(icon).fit_to_exact_size(Vec2::new(18.0, 18.0));
    let button = egui::Button::image_and_text(image, text)
        .min_size(Vec2::new(116.0, 36.0))
        .corner_radius(egui::CornerRadius::same(8))
        .shortcut_text(shortcut);
    ui.add_enabled(enabled, button)
}

fn metric(ui: &mut egui::Ui, label: &str, value: String, palette: Palette) {
    ui.vertical(|ui| {
        ui.label(RichText::new(label).small().color(palette.muted));
        ui.label(RichText::new(value).size(18.0).strong().color(palette.text));
    });
    ui.add_space(22.0);
}

fn compression_ratio(entry: &ArchiveEntry) -> f64 {
    if entry.size == 0 {
        0.0
    } else {
        entry.packed_size as f64 * 100.0 / entry.size as f64
    }
}

fn local_notes() -> String {
    include_str!("../../RELEASE_NOTES.md").into()
}

fn format_size(value: u64) -> String {
    let mut amount = value as f64;
    for unit in ["B", "KB", "MB", "GB", "TB"] {
        if amount < 1024.0 || unit == "TB" {
            return if unit == "B" {
                format!("{amount:.0} {unit}")
            } else {
                format!("{amount:.1} {unit}")
            };
        }
        amount /= 1024.0;
    }
    format!("{value} B")
}

fn suggested_archive_path(source: &Path) -> PathBuf {
    unique_path(source.with_extension("wolrar"))
}

fn suggested_extract_directory(archive: &Path) -> PathBuf {
    unique_path(archive.with_extension(""))
}

fn unique_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("archivio");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 1..1000 {
        let name = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let candidate = path.with_file_name(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_and_light_palettes_are_distinct() {
        assert_ne!(Palette::dark().background, Palette::light().background);
        assert!(
            Palette::dark().background.to_array()[0] < Palette::light().background.to_array()[0]
        );
    }

    #[test]
    fn ratios_and_sizes_are_consistent() {
        let entry = ArchiveEntry {
            path: "file".into(),
            size: 200,
            packed_size: 50,
            modified: "0".into(),
            directory: false,
            encrypted: false,
        };
        assert_eq!(compression_ratio(&entry), 25.0);
        assert_eq!(format_size(1024), "1.0 KB");
    }
}

from __future__ import annotations

import platform
import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk
from typing import Callable, TypeVar

from wolrar import __version__
from wolrar.app_context import resource_path
from wolrar.backend import ArchiveEntry, ArchiveError, SevenZip
from wolrar.news import NewsPanel
from wolrar.shell import suggested_archive_path
from wolrar.update import UpdateError


T = TypeVar("T")
FONT = "Segoe UI" if platform.system() == "Windows" else "Helvetica"
ARCHIVE_FILETYPES = [
    ("Archivi", "*.7z *.zip *.rar *.tar *.gz *.bz2 *.xz *.iso *.cab"),
    ("7-Zip", "*.7z"),
    ("ZIP", "*.zip"),
    ("RAR (sola lettura)", "*.rar"),
    ("Tutti i file", "*.*"),
]
COLORS = {
    "primary": "#173f35",
    "accent": "#27866d",
    "muted": "#526071",
    "surface": "#f3f6f5",
    "stripe": "#edf3f0",
}


def format_size(value: int | None) -> str:
    if value is None:
        return "—"
    amount = float(value)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if amount < 1024 or unit == "TB":
            return f"{amount:.0f} {unit}" if unit == "B" else f"{amount:.1f} {unit}"
        amount /= 1024
    return str(value)


class CreateArchiveDialog(tk.Toplevel):
    def __init__(
        self,
        parent: tk.Misc,
        initial_sources: list[Path] | None = None,
        initial_output: Path | None = None,
    ) -> None:
        super().__init__(parent)
        self.title("Nuovo archivio · Wol-rar")
        self.geometry("720x520")
        self.minsize(620, 460)
        self.transient(parent)
        self.grab_set()
        self.result: tuple[Path, list[Path], int] | None = None
        self.sources: list[Path] = []
        suggested = initial_output or (suggested_archive_path(initial_sources[0]) if initial_sources else Path.home() / "Nuovo archivio.7z")
        self.output_var = tk.StringVar(value=str(suggested))
        self.level_var = tk.StringVar(value="Normale · 5")

        container = ttk.Frame(self, padding=22)
        container.pack(fill="both", expand=True)
        container.columnconfigure(1, weight=1)
        container.rowconfigure(2, weight=1)
        ttk.Label(container, text="Crea un nuovo archivio", font=(FONT, 17, "bold"), foreground=COLORS["primary"]).grid(
            row=0, column=0, columnspan=3, sticky="w", pady=(0, 5)
        )
        ttk.Label(container, text="Aggiungi contenuti e scegli il formato 7z o ZIP.", foreground=COLORS["muted"]).grid(
            row=1, column=0, columnspan=3, sticky="w", pady=(0, 14)
        )

        list_frame = ttk.Frame(container)
        list_frame.grid(row=2, column=0, columnspan=3, sticky="nsew", pady=(0, 8))
        list_frame.columnconfigure(0, weight=1)
        list_frame.rowconfigure(0, weight=1)
        self.source_list = tk.Listbox(list_frame, activestyle="none", font=(FONT, 10), relief="flat", background="#f5f7f6")
        self.source_list.grid(row=0, column=0, sticky="nsew")
        scrollbar = ttk.Scrollbar(list_frame, orient="vertical", command=self.source_list.yview)
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.source_list.configure(yscrollcommand=scrollbar.set)

        source_buttons = ttk.Frame(container)
        source_buttons.grid(row=3, column=0, columnspan=3, sticky="w", pady=(0, 18))
        ttk.Button(source_buttons, text="Aggiungi file", command=self.add_files).pack(side="left")
        ttk.Button(source_buttons, text="Aggiungi cartella", command=self.add_folder).pack(side="left", padx=8)
        ttk.Button(source_buttons, text="Rimuovi", command=self.remove_selected).pack(side="left")

        ttk.Label(container, text="Destinazione").grid(row=4, column=0, sticky="w", padx=(0, 10), pady=6)
        ttk.Entry(container, textvariable=self.output_var).grid(row=4, column=1, sticky="ew", pady=6)
        ttk.Button(container, text="Sfoglia…", command=self.browse_output).grid(row=4, column=2, padx=(8, 0), pady=6)
        ttk.Label(container, text="Compressione").grid(row=5, column=0, sticky="w", padx=(0, 10), pady=6)
        ttk.Combobox(
            container,
            textvariable=self.level_var,
            values=("Archivia · 0", "Veloce · 3", "Normale · 5", "Massima · 7"),
            state="readonly",
            width=20,
        ).grid(row=5, column=1, sticky="w", pady=6)
        ttk.Label(
            container,
            text="I file esistenti non vengono sovrascritti. RAR resta disponibile solo in lettura.",
            foreground=COLORS["muted"],
        ).grid(row=6, column=0, columnspan=3, sticky="w", pady=(12, 18))

        actions = ttk.Frame(container)
        actions.grid(row=7, column=0, columnspan=3, sticky="e")
        ttk.Button(actions, text="Annulla", command=self.destroy).pack(side="left", padx=(0, 8))
        ttk.Button(actions, text="Crea archivio", command=self.confirm).pack(side="left")
        self.protocol("WM_DELETE_WINDOW", self.destroy)
        for source in initial_sources or []:
            self._add_source(Path(source))

    def add_files(self) -> None:
        for name in filedialog.askopenfilenames(parent=self, title="Aggiungi file"):
            self._add_source(Path(name))

    def add_folder(self) -> None:
        name = filedialog.askdirectory(parent=self, title="Aggiungi cartella")
        if name:
            self._add_source(Path(name))

    def _add_source(self, source: Path) -> None:
        if source not in self.sources:
            self.sources.append(source)
            self.source_list.insert("end", str(source))

    def remove_selected(self) -> None:
        for index in reversed(self.source_list.curselection()):
            self.source_list.delete(index)
            del self.sources[index]

    def browse_output(self) -> None:
        name = filedialog.asksaveasfilename(
            parent=self,
            title="Salva archivio",
            defaultextension=".7z",
            filetypes=[("7-Zip", "*.7z"), ("ZIP", "*.zip")],
            confirmoverwrite=False,
        )
        if name:
            self.output_var.set(name)

    def confirm(self) -> None:
        if not self.sources:
            messagebox.showwarning("Contenuti mancanti", "Aggiungi almeno un file o una cartella.", parent=self)
            return
        output = Path(self.output_var.get().strip())
        if output.suffix.lower() not in {".7z", ".zip"}:
            messagebox.showwarning("Formato non valido", "Usa un nome file con estensione .7z o .zip.", parent=self)
            return
        level = int(self.level_var.get().rsplit(" · ", 1)[1])
        self.result = (output, list(self.sources), level)
        self.destroy()


class ArchiveApp(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title(f"Wol-rar {__version__}")
        self.geometry("1080x700")
        self.minsize(880, 560)
        self.configure(background=COLORS["surface"])
        self.engine = SevenZip()
        self.current_archive: Path | None = None
        self.current_entries: list[ArchiveEntry] = []
        self.busy = False
        self.icons: dict[str, tk.PhotoImage] = {}
        self.status_var = tk.StringVar(value="Pronto · rete solo per aggiornamenti su richiesta")
        self.archive_var = tk.StringVar(value="Apri un archivio o creane uno nuovo")
        self.summary_var = tk.StringVar(value="Nessun contenuto da mostrare")
        self.search_var = tk.StringVar()
        self.search_var.trace_add("write", lambda *_: self.populate_entries())
        self._load_icons()
        self._configure_style()
        self._build_ui()
        self._bind_shortcuts()

    def _load_icons(self) -> None:
        for name in ("open", "create", "extract", "test", "update", "info"):
            path = resource_path("assets", f"menu-{name}.png")
            if path.is_file():
                self.icons[name] = tk.PhotoImage(file=str(path)).subsample(3, 3)
        app_icon = resource_path("assets", "wol-rar.png")
        if app_icon.is_file():
            self.app_icon = tk.PhotoImage(file=str(app_icon))
            self.iconphoto(True, self.app_icon)
            self.header_icon = self.app_icon.subsample(4, 4)

    def _configure_style(self) -> None:
        style = ttk.Style(self)
        if "clam" in style.theme_names():
            style.theme_use("clam")
        style.configure("TFrame", background=COLORS["surface"])
        style.configure("TLabel", background=COLORS["surface"], font=(FONT, 10))
        style.configure("TButton", padding=(12, 8), font=(FONT, 10))
        style.map("TButton", foreground=[("active", COLORS["primary"])])
        style.configure("Treeview", rowheight=30, font=(FONT, 10), fieldbackground="#ffffff")
        style.configure("Treeview.Heading", font=(FONT, 10, "bold"), padding=(6, 7))
        style.configure("TNotebook", background=COLORS["surface"], borderwidth=0)
        style.configure("TNotebook.Tab", padding=(16, 9), font=(FONT, 10, "bold"))

    def _button(self, parent: tk.Misc, text: str, icon: str, command: Callable) -> ttk.Button:
        return ttk.Button(parent, text=text, image=self.icons.get(icon, ""), compound="left", command=command)

    def _build_ui(self) -> None:
        header = ttk.Frame(self, padding=(22, 16))
        header.pack(fill="x")
        if hasattr(self, "header_icon"):
            ttk.Label(header, image=self.header_icon).pack(side="left", padx=(0, 14))
        title_group = ttk.Frame(header)
        title_group.pack(side="left")
        ttk.Label(title_group, text="Wol-rar", font=(FONT, 23, "bold"), foreground=COLORS["primary"]).pack(anchor="w")
        ttk.Label(title_group, text=f"Archivi privati, semplici e locali · v{__version__}", foreground=COLORS["muted"]).pack(anchor="w")
        ttk.Label(header, text="● NESSUNA TELEMETRIA", foreground="#197052", font=(FONT, 10, "bold")).pack(side="right", pady=(10, 0))

        self.notebook = ttk.Notebook(self)
        self.notebook.pack(fill="both", expand=True, padx=14)
        archive_page = ttk.Frame(self.notebook, padding=8)
        self.notebook.add(archive_page, text="  Archivio  ")
        news_page = NewsPanel(self.notebook, submit=self._submit, set_status=self.status_var.set)
        self.notebook.add(news_page, text="  Novità e aggiornamenti  ")

        toolbar = ttk.Frame(archive_page, padding=(4, 2, 4, 12))
        toolbar.pack(fill="x")
        self._button(toolbar, "Apri", "open", self.choose_archive).pack(side="left", padx=(0, 6))
        self._button(toolbar, "Nuovo archivio", "create", self.create_archive).pack(side="left", padx=6)
        self._button(toolbar, "Estrai", "extract", self.extract_archive).pack(side="left", padx=6)
        self._button(toolbar, "Verifica", "test", self.test_archive).pack(side="left", padx=6)
        self._button(toolbar, "Aggiornamenti", "update", self.show_updates).pack(side="right")

        archive_header = ttk.Frame(archive_page, padding=(4, 2, 4, 8))
        archive_header.pack(fill="x")
        ttk.Label(archive_header, textvariable=self.archive_var, font=(FONT, 11, "bold"), foreground=COLORS["primary"]).pack(side="left")
        ttk.Label(archive_header, textvariable=self.summary_var, foreground=COLORS["muted"]).pack(side="right")

        filter_frame = ttk.Frame(archive_page, padding=(4, 0, 4, 8))
        filter_frame.pack(fill="x")
        ttk.Label(filter_frame, text="Filtra contenuto").pack(side="left", padx=(0, 8))
        self.search_entry = ttk.Entry(filter_frame, textvariable=self.search_var)
        self.search_entry.pack(side="left", fill="x", expand=True)
        ttk.Label(filter_frame, text="  Ctrl+F", foreground=COLORS["muted"]).pack(side="right")

        table = ttk.Frame(archive_page, padding=4)
        table.pack(fill="both", expand=True)
        table.columnconfigure(0, weight=1)
        table.rowconfigure(0, weight=1)
        self.tree = ttk.Treeview(
            table,
            columns=("name", "size", "packed", "modified"),
            show="headings",
            selectmode="extended",
        )
        for column, label in (("name", "Nome"), ("size", "Dimensione"), ("packed", "Compressa"), ("modified", "Modificato")):
            self.tree.heading(column, text=label, command=lambda c=column: self.sort_entries(c))
        self.tree.column("name", width=500, anchor="w")
        self.tree.column("size", width=115, anchor="e")
        self.tree.column("packed", width=115, anchor="e")
        self.tree.column("modified", width=185, anchor="w")
        self.tree.grid(row=0, column=0, sticky="nsew")
        self.tree.tag_configure("odd", background=COLORS["stripe"])
        scrollbar = ttk.Scrollbar(table, orient="vertical", command=self.tree.yview)
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.tree.configure(yscrollcommand=scrollbar.set)

        status = ttk.Frame(self, padding=(12, 7))
        status.pack(fill="x", side="bottom")
        ttk.Label(status, textvariable=self.status_var, foreground=COLORS["muted"]).pack(side="left", fill="x", expand=True)
        self.progress = ttk.Progressbar(status, mode="indeterminate", length=110)
        self.progress.pack(side="right")

    def _bind_shortcuts(self) -> None:
        self.bind("<Control-o>", lambda _: self.choose_archive())
        self.bind("<Control-n>", lambda _: self.create_archive())
        self.bind("<Control-e>", lambda _: self.extract_archive())
        self.bind("<Control-f>", lambda _: self.focus_search())
        self.bind("<F5>", lambda _: self.test_archive())

    def focus_search(self) -> None:
        self.search_entry.focus_set()
        self.search_entry.selection_range(0, "end")

    def choose_archive(self) -> None:
        name = filedialog.askopenfilename(parent=self, title="Apri archivio", filetypes=ARCHIVE_FILETYPES)
        if name:
            self.open_archive(Path(name))

    def open_archive(self, archive: Path, on_complete: Callable[[], None] | None = None) -> None:
        def completed(entries: list[ArchiveEntry]) -> None:
            self.current_archive = archive
            self.current_entries = entries
            self.archive_var.set(archive.name)
            self.populate_entries()
            total = sum(entry.size or 0 for entry in entries)
            packed = sum(entry.packed_size or 0 for entry in entries)
            saving = max(0, 100 - packed * 100 // total) if total and packed else 0
            self.summary_var.set(f"{len(entries)} voci · {format_size(total)} · risparmio {saving}%")
            self.status_var.set(f"{archive.parent} · archivio aperto in sola lettura")
            if on_complete:
                on_complete()

        self._submit("Lettura archivio…", lambda: self.engine.list(archive), completed)

    def populate_entries(self) -> None:
        query = self.search_var.get().casefold().strip()
        self.tree.delete(*self.tree.get_children())
        visible = [entry for entry in self.current_entries if query in entry.path.casefold()]
        for index, entry in enumerate(visible):
            self.tree.insert(
                "",
                "end",
                values=(entry.path, format_size(entry.size), format_size(entry.packed_size), entry.modified),
                tags=("odd",) if index % 2 else (),
            )
        if query:
            self.status_var.set(f"{len(visible)} voci su {len(self.current_entries)} · filtro “{self.search_var.get()}”")

    def sort_entries(self, column: str) -> None:
        descending = getattr(self, f"_sort_{column}", False)
        key = {
            "name": lambda entry: entry.path.casefold(),
            "size": lambda entry: entry.size or 0,
            "packed": lambda entry: entry.packed_size or 0,
            "modified": lambda entry: entry.modified,
        }[column]
        self.current_entries.sort(key=key, reverse=descending)
        setattr(self, f"_sort_{column}", not descending)
        self.populate_entries()

    def create_archive(self, initial_sources: list[Path] | None = None, initial_output: Path | None = None) -> None:
        dialog = CreateArchiveDialog(self, initial_sources, initial_output)
        self.wait_window(dialog)
        if not dialog.result:
            return
        archive, sources, level = dialog.result

        def completed(_: None) -> None:
            self.status_var.set("Archivio creato correttamente")
            self.open_archive(archive)

        self._submit("Creazione archivio…", lambda: self.engine.create(archive, sources, level), completed)

    def extract_archive(self, destination: Path | None = None) -> None:
        if not self._require_archive():
            return
        if destination is None:
            selected = filedialog.askdirectory(parent=self, title="Cartella di destinazione")
            if not selected:
                return
            destination = Path(selected)
        archive = self.current_archive
        assert archive is not None

        def completed(_: None) -> None:
            self.status_var.set(f"Estrazione completata in {destination} · file esistenti mantenuti")
            messagebox.showinfo("Estrazione completata", f"File estratti in:\n{destination}\n\nNessun file esistente è stato sovrascritto.", parent=self)

        self._submit("Estrazione sicura…", lambda: self.engine.extract(archive, destination), completed)

    def test_archive(self) -> None:
        if not self._require_archive():
            return
        archive = self.current_archive
        assert archive is not None

        def completed(_: None) -> None:
            self.status_var.set("Test integrità superato")
            messagebox.showinfo("Integrità archivio", "Nessun errore rilevato da 7-Zip.", parent=self)

        self._submit("Verifica integrità…", lambda: self.engine.test(archive), completed)

    def show_updates(self) -> None:
        self.notebook.select(1)

    def _require_archive(self) -> bool:
        if self.current_archive is None:
            messagebox.showinfo("Nessun archivio", "Apri prima un archivio.", parent=self)
            return False
        return True

    def _submit(self, label: str, operation: Callable[[], T], completed: Callable[[T], None]) -> None:
        if self.busy:
            messagebox.showinfo("Operazione in corso", "Attendi il completamento dell'operazione corrente.", parent=self)
            return
        self.busy = True
        self.status_var.set(label)
        self.progress.start(12)

        def work() -> None:
            try:
                result = operation()
            except (ArchiveError, UpdateError, OSError) as error:
                self.after(0, lambda: failed(error))
            else:
                self.after(0, lambda: succeeded(result))

        def succeeded(result: T) -> None:
            self.busy = False
            self.progress.stop()
            completed(result)

        def failed(error: Exception) -> None:
            self.busy = False
            self.progress.stop()
            self.status_var.set("Operazione non riuscita")
            messagebox.showerror("Wol-rar", str(error), parent=self)

        threading.Thread(target=work, daemon=True, name="wol-rar-operation").start()


def main() -> None:
    app = ArchiveApp()
    app.mainloop()

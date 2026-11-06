from __future__ import annotations

import threading
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk
from typing import Callable, TypeVar

from wolrar import __version__
from wolrar.app_context import resource_path
from wolrar.backend import ArchiveEntry, ArchiveError, SevenZip
from wolrar.news import NewsPanel
from wolrar.update import UpdateError


T = TypeVar("T")
ARCHIVE_FILETYPES = [
    ("Archivi", "*.7z *.zip *.rar *.tar *.gz *.bz2 *.xz *.iso *.cab"),
    ("7-Zip", "*.7z"),
    ("ZIP", "*.zip"),
    ("RAR (sola lettura)", "*.rar"),
    ("Tutti i file", "*.*"),
]


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
    def __init__(self, parent: tk.Misc) -> None:
        super().__init__(parent)
        self.title("Nuovo archivio")
        self.geometry("680x460")
        self.minsize(580, 400)
        self.transient(parent)
        self.grab_set()
        self.result: tuple[Path, list[Path]] | None = None
        self.sources: list[Path] = []
        self.output_var = tk.StringVar(value=str(Path.home() / "Nuovo archivio.7z"))

        container = ttk.Frame(self, padding=20)
        container.pack(fill="both", expand=True)
        container.columnconfigure(1, weight=1)
        container.rowconfigure(2, weight=1)
        ttk.Label(container, text="Crea archivio 7z o ZIP", font=("Segoe UI", 15, "bold")).grid(
            row=0, column=0, columnspan=3, sticky="w", pady=(0, 16)
        )
        ttk.Label(container, text="Contenuti").grid(row=1, column=0, columnspan=3, sticky="w")

        list_frame = ttk.Frame(container)
        list_frame.grid(row=2, column=0, columnspan=3, sticky="nsew", pady=8)
        list_frame.columnconfigure(0, weight=1)
        list_frame.rowconfigure(0, weight=1)
        self.source_list = tk.Listbox(list_frame, activestyle="none", font=("Segoe UI", 10))
        self.source_list.grid(row=0, column=0, sticky="nsew")
        scrollbar = ttk.Scrollbar(list_frame, orient="vertical", command=self.source_list.yview)
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.source_list.configure(yscrollcommand=scrollbar.set)

        source_buttons = ttk.Frame(container)
        source_buttons.grid(row=3, column=0, columnspan=3, sticky="w", pady=(0, 18))
        ttk.Button(source_buttons, text="Aggiungi file", command=self.add_files).pack(side="left")
        ttk.Button(source_buttons, text="Aggiungi cartella", command=self.add_folder).pack(side="left", padx=8)
        ttk.Button(source_buttons, text="Rimuovi selezionato", command=self.remove_selected).pack(side="left")

        ttk.Label(container, text="Destinazione").grid(row=4, column=0, sticky="w", padx=(0, 10))
        ttk.Entry(container, textvariable=self.output_var).grid(row=4, column=1, sticky="ew")
        ttk.Button(container, text="Sfoglia…", command=self.browse_output).grid(row=4, column=2, padx=(8, 0))
        ttk.Label(
            container,
            text="Nessuna sovrascrittura automatica. La creazione RAR non è disponibile.",
            foreground="#526071",
        ).grid(row=5, column=0, columnspan=3, sticky="w", pady=(10, 18))

        actions = ttk.Frame(container)
        actions.grid(row=6, column=0, columnspan=3, sticky="e")
        ttk.Button(actions, text="Annulla", command=self.destroy).pack(side="left", padx=(0, 8))
        ttk.Button(actions, text="Crea archivio", command=self.confirm).pack(side="left")
        self.protocol("WM_DELETE_WINDOW", self.destroy)

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
        self.result = (output, list(self.sources))
        self.destroy()


class ArchiveApp(tk.Tk):
    def __init__(self) -> None:
        super().__init__()
        self.title(f"Wol-rar {__version__}")
        self.geometry("1000x640")
        self.minsize(780, 480)
        self.engine = SevenZip()
        self.current_archive: Path | None = None
        self.busy = False
        self.status_var = tk.StringVar(value="Pronto · rete solo per aggiornamenti su richiesta")
        self.archive_var = tk.StringVar(value="Nessun archivio aperto")
        icon = resource_path("assets", "wol-rar.png")
        if icon.is_file():
            self.iconphoto(True, tk.PhotoImage(file=str(icon)))
        self._configure_style()
        self._build_ui()

    def _configure_style(self) -> None:
        style = ttk.Style(self)
        if "clam" in style.theme_names():
            style.theme_use("clam")
        style.configure("Treeview", rowheight=28, font=("Segoe UI", 10))
        style.configure("Treeview.Heading", font=("Segoe UI", 10, "bold"))
        style.configure("TButton", padding=(12, 7), font=("Segoe UI", 10))
        style.configure("TLabel", font=("Segoe UI", 10))

    def _build_ui(self) -> None:
        header = ttk.Frame(self, padding=(22, 18))
        header.pack(fill="x")
        ttk.Label(header, text="Wol-rar", font=("Segoe UI", 22, "bold"), foreground="#173f35").pack(side="left")
        ttk.Label(header, text="  Archivi privati, solo sul tuo PC", foreground="#526071").pack(side="left", pady=(8, 0))
        ttk.Label(header, text="● PRIVACY", foreground="#197052", font=("Segoe UI", 10, "bold")).pack(side="right", pady=(8, 0))

        notebook = ttk.Notebook(self)
        notebook.pack(fill="both", expand=True)
        archive_page = ttk.Frame(notebook)
        notebook.add(archive_page, text="Archivio")
        news_page = NewsPanel(notebook, submit=self._submit, set_status=self.status_var.set)
        notebook.add(news_page, text="Novità e aggiornamenti")

        toolbar = ttk.Frame(archive_page, padding=(22, 0, 22, 14))
        toolbar.pack(fill="x")
        ttk.Button(toolbar, text="Apri archivio", command=self.choose_archive).pack(side="left")
        ttk.Button(toolbar, text="Nuovo archivio", command=self.create_archive).pack(side="left", padx=8)
        ttk.Button(toolbar, text="Estrai", command=self.extract_archive).pack(side="left")
        ttk.Button(toolbar, text="Test integrità", command=self.test_archive).pack(side="left", padx=8)

        ttk.Label(archive_page, textvariable=self.archive_var, padding=(22, 4), font=("Segoe UI", 11, "bold")).pack(fill="x")
        table = ttk.Frame(archive_page, padding=(22, 4, 22, 12))
        table.pack(fill="both", expand=True)
        table.columnconfigure(0, weight=1)
        table.rowconfigure(0, weight=1)
        self.tree = ttk.Treeview(
            table,
            columns=("name", "size", "packed", "modified"),
            show="headings",
            selectmode="browse",
        )
        self.tree.heading("name", text="Nome")
        self.tree.heading("size", text="Dimensione")
        self.tree.heading("packed", text="Compressa")
        self.tree.heading("modified", text="Modificato")
        self.tree.column("name", width=480, anchor="w")
        self.tree.column("size", width=110, anchor="e")
        self.tree.column("packed", width=110, anchor="e")
        self.tree.column("modified", width=180, anchor="w")
        self.tree.grid(row=0, column=0, sticky="nsew")
        scrollbar = ttk.Scrollbar(table, orient="vertical", command=self.tree.yview)
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.tree.configure(yscrollcommand=scrollbar.set)
        ttk.Label(
            self,
            textvariable=self.status_var,
            relief="sunken",
            anchor="w",
            padding=(12, 7),
        ).pack(fill="x", side="bottom")

    def choose_archive(self) -> None:
        name = filedialog.askopenfilename(parent=self, title="Apri archivio", filetypes=ARCHIVE_FILETYPES)
        if name:
            self.open_archive(Path(name))

    def open_archive(self, archive: Path) -> None:
        def completed(entries: list[ArchiveEntry]) -> None:
            self.current_archive = archive
            self.archive_var.set(str(archive))
            self.tree.delete(*self.tree.get_children())
            for entry in entries:
                self.tree.insert(
                    "",
                    "end",
                    values=(entry.path, format_size(entry.size), format_size(entry.packed_size), entry.modified),
                )
            self.status_var.set(f"{len(entries)} voci · archivio aperto in sola lettura")

        self._submit("Lettura archivio…", lambda: self.engine.list(archive), completed)

    def create_archive(self) -> None:
        dialog = CreateArchiveDialog(self)
        self.wait_window(dialog)
        if not dialog.result:
            return
        archive, sources = dialog.result

        def completed(_: None) -> None:
            self.status_var.set("Archivio creato correttamente")
            self.open_archive(archive)

        self._submit("Creazione archivio…", lambda: self.engine.create(archive, sources), completed)

    def extract_archive(self) -> None:
        if not self._require_archive():
            return
        destination = filedialog.askdirectory(parent=self, title="Cartella di destinazione")
        if not destination:
            return
        archive = self.current_archive
        assert archive is not None

        def completed(_: None) -> None:
            self.status_var.set("Estrazione completata · file esistenti mantenuti")
            messagebox.showinfo("Estrazione completata", "File estratti senza sovrascrivere quelli esistenti.", parent=self)

        self._submit("Estrazione sicura…", lambda: self.engine.extract(archive, Path(destination)), completed)

    def test_archive(self) -> None:
        if not self._require_archive():
            return
        archive = self.current_archive
        assert archive is not None

        def completed(_: None) -> None:
            self.status_var.set("Test integrità superato")
            messagebox.showinfo("Integrità archivio", "Nessun errore rilevato da 7-Zip.", parent=self)

        self._submit("Verifica integrità…", lambda: self.engine.test(archive), completed)

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

        def work() -> None:
            try:
                result = operation()
            except (ArchiveError, UpdateError, OSError) as error:
                self.after(0, lambda: failed(error))
            else:
                self.after(0, lambda: succeeded(result))

        def succeeded(result: T) -> None:
            self.busy = False
            completed(result)

        def failed(error: Exception) -> None:
            self.busy = False
            self.status_var.set("Operazione non riuscita")
            messagebox.showerror("Wol-rar", str(error), parent=self)

        threading.Thread(target=work, daemon=True, name="wol-rar-operation").start()


def main() -> None:
    app = ArchiveApp()
    app.mainloop()

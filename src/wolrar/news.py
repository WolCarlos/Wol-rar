from __future__ import annotations

import json
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk
from typing import Callable, TypeVar

from wolrar import __version__
from wolrar.app_context import release_asset_name, resource_path
from wolrar.update import ReleaseInfo, UpdateClient, is_newer


T = TypeVar("T")


def local_release_notes() -> str:
    path = resource_path("resources", "changelog.json")
    entries = json.loads(path.read_text(encoding="utf-8"))
    sections: list[str] = []
    for entry in entries:
        sections.append(f"VERSIONE {entry['version']} · {entry['date']}\n")
        sections.extend(f"• {note}" for note in entry["notes"])
        sections.append("")
    return "\n".join(sections)


class NewsPanel(ttk.Frame):
    def __init__(
        self,
        parent: tk.Misc,
        submit: Callable[[str, Callable[[], T], Callable[[T], None]], None],
        set_status: Callable[[str], None],
    ) -> None:
        super().__init__(parent, padding=22)
        self._submit = submit
        self._set_status = set_status
        self.client = UpdateClient()
        self.latest: ReleaseInfo | None = None
        self.columnconfigure(0, weight=1)
        self.rowconfigure(2, weight=1)

        heading = ttk.Frame(self)
        heading.grid(row=0, column=0, sticky="ew", pady=(0, 14))
        ttk.Label(heading, text="Novità e aggiornamenti", font=("Segoe UI", 16, "bold")).pack(side="left")
        ttk.Label(heading, text=f"Versione installata: {__version__}", foreground="#526071").pack(side="right", pady=(4, 0))
        ttk.Label(
            self,
            text="La rete viene usata soltanto quando premi “Controlla aggiornamenti”. Nessun dato personale viene inviato.",
            foreground="#526071",
        ).grid(row=1, column=0, sticky="w", pady=(0, 12))

        text_frame = ttk.Frame(self)
        text_frame.grid(row=2, column=0, sticky="nsew")
        text_frame.columnconfigure(0, weight=1)
        text_frame.rowconfigure(0, weight=1)
        self.notes = tk.Text(
            text_frame,
            wrap="word",
            state="disabled",
            font=("Segoe UI", 11),
            padx=16,
            pady=14,
            relief="flat",
            background="#f5f7f8",
            foreground="#17242c",
        )
        self.notes.grid(row=0, column=0, sticky="nsew")
        scrollbar = ttk.Scrollbar(text_frame, orient="vertical", command=self.notes.yview)
        scrollbar.grid(row=0, column=1, sticky="ns")
        self.notes.configure(yscrollcommand=scrollbar.set)
        self._set_notes(local_release_notes())

        actions = ttk.Frame(self)
        actions.grid(row=3, column=0, sticky="ew", pady=(16, 0))
        ttk.Button(actions, text="Controlla aggiornamenti", command=self.check_updates).pack(side="left")
        self.download_button = ttk.Button(actions, text="Scarica aggiornamento", command=self.download_update, state="disabled")
        self.download_button.pack(side="left", padx=8)
        ttk.Button(actions, text="Licenze", command=self.show_licenses).pack(side="left")
        ttk.Label(actions, text="SHA-256 verificato prima del salvataggio", foreground="#526071").pack(side="right")

    def show_licenses(self) -> None:
        window = tk.Toplevel(self)
        window.title("Licenze · Wol-rar")
        window.geometry("760x560")
        license_text = resource_path("7ZIP-LICENSE.txt").read_text(encoding="utf-8")
        content = (
            f"Wol-rar {__version__}\n"
            "Applicazione source-available; nessuna licenza open-source sul codice applicativo.\n\n"
            "Il motore 7-Zip conserva le licenze riportate di seguito.\n"
            "RAR è supportato soltanto in lettura; il codice unRAR non può essere usato\n"
            "per ricreare l'algoritmo proprietario di compressione RAR.\n\n"
            + license_text
        )
        text = tk.Text(window, wrap="word", padx=16, pady=14, font=("Segoe UI", 10))
        text.pack(fill="both", expand=True)
        text.insert("1.0", content)
        text.configure(state="disabled")

    def _set_notes(self, value: str) -> None:
        self.notes.configure(state="normal")
        self.notes.delete("1.0", "end")
        self.notes.insert("1.0", value)
        self.notes.configure(state="disabled")

    def check_updates(self) -> None:
        def completed(release: ReleaseInfo) -> None:
            self.latest = release
            title = f"ULTIMA RELEASE {release.version} · {release.published_at[:10]}\n\n"
            self._set_notes(title + release.notes + "\n\n" + local_release_notes())
            if is_newer(release.version):
                self.download_button.configure(state="normal")
                self._set_status(f"Disponibile Wol-rar {release.version}")
            else:
                self.download_button.configure(state="disabled")
                self._set_status("Wol-rar è già aggiornato")

        self._submit("Controllo aggiornamenti su GitHub…", self.client.check_latest, completed)

    def download_update(self) -> None:
        if self.latest is None or not is_newer(self.latest.version):
            return
        asset_name = release_asset_name()
        destination = filedialog.asksaveasfilename(
            parent=self,
            title="Scarica aggiornamento",
            initialfile=asset_name,
            defaultextension=Path(asset_name).suffix,
            confirmoverwrite=False,
        )
        if not destination:
            return

        def completed(digest: str) -> None:
            self.download_button.configure(state="disabled")
            self._set_status("Aggiornamento scaricato e verificato")
            messagebox.showinfo(
                "Download completato",
                f"Aggiornamento salvato e verificato.\n\nSHA-256: {digest}\n\nChiudi Wol-rar prima di installare la nuova versione.",
                parent=self,
            )

        self._submit(
            "Download e verifica aggiornamento…",
            lambda: self.client.download(self.latest, Path(destination)),
            completed,
        )

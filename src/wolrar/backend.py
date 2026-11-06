from __future__ import annotations

import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path, PurePosixPath
from typing import Callable, Sequence

from wolrar.app_context import engine_relative_path, resource_path


class ArchiveError(RuntimeError):
    """Errore leggibile durante un'operazione su un archivio."""


@dataclass(frozen=True)
class ArchiveEntry:
    path: str
    size: int | None
    packed_size: int | None
    modified: str
    attributes: str


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    stdout: str
    stderr: str


Runner = Callable[[Sequence[str], Path | None], CommandResult]


def _decode_output(value: bytes) -> str:
    return value.decode("utf-8", errors="replace")


def _default_runner(arguments: Sequence[str], cwd: Path | None) -> CommandResult:
    completed = subprocess.run(
        list(arguments),
        cwd=str(cwd) if cwd else None,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        shell=False,
        check=False,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    return CommandResult(
        completed.returncode,
        _decode_output(completed.stdout),
        _decode_output(completed.stderr),
    )


def parse_slt_listing(output: str) -> list[ArchiveEntry]:
    entries: list[ArchiveEntry] = []
    for block in re.split(r"(?:\r?\n){2,}", output.strip()):
        fields: dict[str, str] = {}
        for line in block.splitlines():
            key, separator, value = line.partition(" = ")
            if separator:
                fields[key.strip()] = value.strip()
        path = fields.get("Path")
        if not path:
            continue
        entries.append(
            ArchiveEntry(
                path=path,
                size=_optional_int(fields.get("Size")),
                packed_size=_optional_int(fields.get("Packed Size")),
                modified=fields.get("Modified", ""),
                attributes=fields.get("Attributes", ""),
            )
        )
    return entries


def _optional_int(value: str | None) -> int | None:
    if value is None or not value.isdigit():
        return None
    return int(value)


def validate_member_path(name: str) -> None:
    """Rifiuta percorsi assoluti, traversal, unità e flussi alternativi NTFS."""
    normalized = name.replace("\\", "/")
    candidate = PurePosixPath(normalized)
    if (
        not normalized
        or "\x00" in normalized
        or normalized.startswith("/")
        or candidate.is_absolute()
        or ":" in normalized
        or ".." in candidate.parts
    ):
        raise ArchiveError(f"Voce non sicura nell'archivio: {name!r}")


class SevenZip:
    def __init__(
        self,
        executable: Path | None = None,
        runner: Runner | None = None,
    ) -> None:
        self.executable = executable or resource_path(*engine_relative_path().parts)
        self._runner = runner or _default_runner

    def _run(self, arguments: Sequence[str], cwd: Path | None = None) -> CommandResult:
        if not self.executable.is_file():
            raise ArchiveError(
                "Motore 7-Zip non trovato. Esegui scripts/fetch_7zip.py dal progetto."
            )
        result = self._runner([str(self.executable), *map(str, arguments)], cwd)
        if result.returncode != 0:
            detail = result.stderr.strip() or result.stdout.strip() or "Errore sconosciuto"
            raise ArchiveError(f"7-Zip ha restituito codice {result.returncode}:\n{detail}")
        return result

    def list(self, archive: Path) -> list[ArchiveEntry]:
        archive = Path(archive).resolve()
        if not archive.is_file():
            raise ArchiveError("Seleziona un file di archivio esistente.")
        result = self._run(["l", "-slt", "-ba", "-sccUTF-8", "--", str(archive)])
        return parse_slt_listing(result.stdout)

    def create(self, archive: Path, sources: Sequence[Path], level: int = 5) -> None:
        archive = Path(archive).resolve()
        suffix = archive.suffix.lower()
        if suffix == ".rar":
            raise ArchiveError("La creazione RAR non è supportata: usa 7z o ZIP.")
        if suffix not in {".7z", ".zip"}:
            raise ArchiveError("Formato di creazione non supportato: usa .7z o .zip.")
        if archive.exists():
            raise ArchiveError("L'archivio di destinazione esiste già; scegli un altro nome.")
        if not sources:
            raise ArchiveError("Aggiungi almeno un file o una cartella.")
        if not 0 <= level <= 9:
            raise ArchiveError("Il livello di compressione deve essere tra 0 e 9.")

        resolved_sources = [Path(source).resolve() for source in sources]
        for source in resolved_sources:
            if not source.exists():
                raise ArchiveError(f"Origine non trovata: {source}")
            if source == archive:
                raise ArchiveError("L'archivio non può includere sé stesso.")
            if source.is_dir() and archive.is_relative_to(source):
                raise ArchiveError("Salva l'archivio fuori dalla cartella da comprimere.")

        base = Path(os.path.commonpath([str(source.parent) for source in resolved_sources]))
        archive.parent.mkdir(parents=True, exist_ok=True)
        relative_sources = [str(source.relative_to(base)) for source in resolved_sources]
        self._run(
            [
                "a",
                f"-t{suffix[1:]}",
                f"-mx={level}",
                "-sccUTF-8",
                "-y",
                "--",
                str(archive),
                *relative_sources,
            ],
            cwd=base,
        )

    def extract(self, archive: Path, destination: Path) -> None:
        archive = Path(archive).resolve()
        destination = Path(destination).resolve()
        entries = self.list(archive)
        if not entries:
            raise ArchiveError("Nessuna voce leggibile trovata nell'archivio.")
        for entry in entries:
            validate_member_path(entry.path)
        if destination.exists() and not destination.is_dir():
            raise ArchiveError("La destinazione esiste e non è una cartella.")
        destination.mkdir(parents=True, exist_ok=True)
        self._run(
            [
                "x",
                str(archive),
                f"-o{destination}",
                "-aos",
                "-snl-",
                "-sccUTF-8",
                "--",
            ]
        )

    def test(self, archive: Path) -> None:
        archive = Path(archive).resolve()
        if not archive.is_file():
            raise ArchiveError("Seleziona un file di archivio esistente.")
        self._run(["t", "-sccUTF-8", "--", str(archive)])

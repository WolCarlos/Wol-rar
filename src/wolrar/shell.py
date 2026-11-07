from __future__ import annotations

from pathlib import Path


def unique_path(path: Path) -> Path:
    if not path.exists():
        return path
    for index in range(1, 1000):
        candidate = path.with_name(f"{path.stem} ({index}){path.suffix}")
        if not candidate.exists():
            return candidate
    raise RuntimeError("Impossibile trovare un nome disponibile.")


def suggested_archive_path(source: Path) -> Path:
    source = Path(source).resolve()
    return unique_path(source.with_suffix(".7z"))


def suggested_extract_directory(archive: Path) -> Path:
    archive = Path(archive).resolve()
    return unique_path(archive.parent / archive.stem)

from __future__ import annotations

import argparse
from pathlib import Path

from wolrar import __version__
from wolrar.shell import suggested_archive_path, suggested_extract_directory
from wolrar.ui import ArchiveApp


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="Wol-rar", description="Archiviatore privato per Windows, Ubuntu e macOS")
    parser.add_argument("archive", nargs="?", type=Path, help="Archivio da aprire")
    actions = parser.add_mutually_exclusive_group()
    actions.add_argument("--open", dest="open_path", type=Path, help="Apri un archivio")
    actions.add_argument("--extract", dest="extract_path", type=Path, help="Estrai in una nuova cartella")
    actions.add_argument("--test", dest="test_path", type=Path, help="Verifica l'integrità di un archivio")
    actions.add_argument("--create", dest="create_path", type=Path, help="Comprimi un file o una cartella")
    actions.add_argument("--smoke-test", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--version", action="version", version=__version__)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    app = ArchiveApp()
    if args.smoke_test:
        if not app.engine.executable.is_file():
            raise RuntimeError(f"Motore 7-Zip non incluso: {app.engine.executable}")
        app.after(150, app.destroy)
    elif args.extract_path:
        archive = args.extract_path.resolve()
        destination = suggested_extract_directory(archive)
        app.after(100, lambda: app.open_archive(archive, lambda: app.extract_archive(destination)))
    elif args.test_path:
        app.after(100, lambda: app.open_archive(args.test_path.resolve(), app.test_archive))
    elif args.create_path:
        source = args.create_path.resolve()
        app.after(100, lambda: app.create_archive([source], suggested_archive_path(source)))
    elif args.open_path or args.archive:
        archive = (args.open_path or args.archive).resolve()
        app.after(100, lambda: app.open_archive(archive))
    app.mainloop()


if __name__ == "__main__":
    main()

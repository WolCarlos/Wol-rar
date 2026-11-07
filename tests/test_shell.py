from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from wolrar.app_context import update_asset_name
from wolrar.shell import suggested_archive_path, suggested_extract_directory, unique_path


class ShellTests(unittest.TestCase):
    def test_unique_paths_and_suggestions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "documento.txt"
            source.write_text("test", encoding="utf-8")
            archive = root / "documento.7z"
            self.assertEqual(suggested_archive_path(source), archive)
            archive.write_bytes(b"occupato")
            self.assertEqual(suggested_archive_path(source), root / "documento (1).7z")
            self.assertEqual(suggested_extract_directory(archive), root / "documento")
            (root / "documento").mkdir()
            self.assertEqual(unique_path(root / "documento"), root / "documento (1)")

    def test_windows_update_prefers_installer(self) -> None:
        self.assertEqual(update_asset_name("windows"), "Wol-rar-Setup-x64.exe")
        self.assertEqual(update_asset_name("linux"), "Wol-rar-Linux-x64.tar.gz")


if __name__ == "__main__":
    unittest.main()

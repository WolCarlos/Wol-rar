from __future__ import annotations

import tempfile
import unittest
import zipfile
from pathlib import Path

from wolrar.backend import (
    ArchiveError,
    SevenZip,
    parse_slt_listing,
    validate_member_path,
)


class ListingTests(unittest.TestCase):
    def test_parse_slt_listing(self) -> None:
        output = (
            "Path = cartella/documento.txt\n"
            "Size = 123\n"
            "Packed Size = 45\n"
            "Modified = 2026-11-06 12:00:00\n"
            "Attributes = A\n\n"
            "Path = cartella/\n"
            "Size = 0\n"
            "Attributes = D\n"
        )
        entries = parse_slt_listing(output)
        self.assertEqual(len(entries), 2)
        self.assertEqual(entries[0].path, "cartella/documento.txt")
        self.assertEqual(entries[0].size, 123)
        self.assertEqual(entries[0].packed_size, 45)
        self.assertEqual(entries[1].attributes, "D")

    def test_validate_member_path(self) -> None:
        validate_member_path("cartella/documento.txt")
        validate_member_path("cartella\\documento.txt")
        for unsafe in ("../fuori.txt", "a/../../fuori", "/assoluto", "C:/Windows/x", "file.txt:ads", ""):
            with self.subTest(unsafe=unsafe), self.assertRaises(ArchiveError):
                validate_member_path(unsafe)


class IntegrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.engine = SevenZip()
        if not self.engine.executable.is_file():
            self.skipTest("Motore 7-Zip non installato")

    def test_roundtrip_7z_and_zip(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "dati"
            source.mkdir()
            (source / "documento àèé.txt").write_text("contenuto riservato", encoding="utf-8")
            for suffix in (".7z", ".zip"):
                with self.subTest(suffix=suffix):
                    archive = root / f"prova{suffix}"
                    destination = root / f"estratto{suffix}"
                    self.engine.create(archive, [source])
                    self.engine.test(archive)
                    entries = self.engine.list(archive)
                    self.assertTrue(any(entry.path.endswith("documento àèé.txt") for entry in entries))
                    self.engine.extract(archive, destination)
                    extracted = destination / "dati" / "documento àèé.txt"
                    self.assertEqual(extracted.read_text(encoding="utf-8"), "contenuto riservato")

    def test_rar_creation_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(ArchiveError):
                self.engine.create(Path(temporary) / "vietato.rar", [])

    def test_path_traversal_is_refused_before_extraction(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "malevolo.zip"
            with zipfile.ZipFile(archive, "w") as malicious:
                malicious.writestr("../fuori.txt", "non deve essere scritto")
            with self.assertRaises(ArchiveError):
                self.engine.extract(archive, root / "destinazione")
            self.assertFalse((root / "fuori.txt").exists())

    def test_existing_archive_is_not_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "origine.txt"
            source.write_text("origine", encoding="utf-8")
            archive = root / "esistente.zip"
            archive.write_bytes(b"non toccare")
            with self.assertRaises(ArchiveError):
                self.engine.create(archive, [source])
            self.assertEqual(archive.read_bytes(), b"non toccare")

    def test_extraction_does_not_overwrite_existing_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "dati"
            source.mkdir()
            original = source / "documento.txt"
            original.write_text("nuovo contenuto", encoding="utf-8")
            archive = root / "prova.zip"
            self.engine.create(archive, [source])
            destination = root / "estratto"
            existing = destination / "dati" / "documento.txt"
            existing.parent.mkdir(parents=True)
            existing.write_text("contenuto precedente", encoding="utf-8")
            self.engine.extract(archive, destination)
            self.assertEqual(existing.read_text(encoding="utf-8"), "contenuto precedente")


if __name__ == "__main__":
    unittest.main()

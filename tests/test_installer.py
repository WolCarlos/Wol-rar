from pathlib import Path
import unittest


class InstallerTests(unittest.TestCase):
    def test_explorer_rules_use_dotted_extensions_and_cleanup(self) -> None:
        script = Path(__file__).resolve().parents[1].joinpath("installer", "wol-rar.iss").read_text(encoding="utf-8")
        self.assertIn('[Registry]', script)
        self.assertIn('#define ArchiveExtension ".zip"', script)
        self.assertIn('#define ArchiveExtension ".7z"', script)
        self.assertIn('#define ArchiveExtension ".rar"', script)
        self.assertIn('SystemFileAssociations\\{#ArchiveExtension}\\shell\\Wol-rar', script)
        self.assertIn('Flags: uninsdeletekey', script)
        self.assertIn('PrivilegesRequired=lowest', script)


if __name__ == "__main__":
    unittest.main()

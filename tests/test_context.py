import unittest

from wolrar.app_context import current_platform, engine_relative_path, release_asset_name


class PlatformTests(unittest.TestCase):
    def test_platform_aliases_and_assets(self) -> None:
        self.assertEqual(current_platform("Windows"), "windows")
        self.assertEqual(current_platform("Linux"), "linux")
        self.assertEqual(current_platform("Darwin"), "macos")
        self.assertEqual(current_platform("macos"), "macos")
        self.assertEqual(engine_relative_path("macos").as_posix(), "vendor/7zip/macos/7zz")
        self.assertEqual(release_asset_name("macos"), "Wol-rar-macOS.tar.gz")


if __name__ == "__main__":
    unittest.main()

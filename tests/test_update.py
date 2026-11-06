from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from wolrar.app_context import release_asset_name
from wolrar.update import ReleaseInfo, UpdateClient, UpdateError, _checksum_for, is_newer


class UpdateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.asset_name = release_asset_name()
        self.payload = b"nuovo eseguibile"
        self.digest = hashlib.sha256(self.payload).hexdigest()
        release = {
            "tag_name": "v9.9.9",
            "body": "Novità di test",
            "published_at": "2026-11-06T12:00:00Z",
            "assets": [
                {"name": self.asset_name, "browser_download_url": "https://github.com/download/app", "size": len(self.payload)},
                {"name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/download/sums", "size": 100},
            ],
        }

        def fetch(url: str, maximum: int) -> bytes:
            if url.endswith("/releases/latest"):
                return json.dumps(release).encode()
            if url.endswith("/sums"):
                return f"{self.digest}  {self.asset_name}\n".encode()
            if url.endswith("/app"):
                return self.payload
            raise AssertionError(url)

        self.client = UpdateClient(fetcher=fetch)

    def test_latest_release_and_verified_download(self) -> None:
        release = self.client.check_latest()
        self.assertEqual(release.version, "v9.9.9")
        self.assertTrue(is_newer(release.version))
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / self.asset_name
            digest = self.client.download(release, destination)
            self.assertEqual(digest, self.digest)
            self.assertEqual(destination.read_bytes(), self.payload)
            self.assertFalse(destination.with_name(destination.name + ".part").exists())

    def test_invalid_checksum_is_refused(self) -> None:
        release = self.client.check_latest()
        client = UpdateClient(
            fetcher=lambda url, maximum: (
                f"{'0' * 64}  {self.asset_name}\n".encode()
                if url.endswith("/sums")
                else self.payload
            )
        )
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaises(UpdateError):
                client.download(release, Path(temporary) / self.asset_name)

    def test_checksum_accepts_text_and_binary_markers(self) -> None:
        digest = "a" * 64
        self.assertEqual(_checksum_for(f"{digest}  app.exe\n", "app.exe"), digest)
        self.assertEqual(_checksum_for(f"{digest} *app.exe\n", "app.exe"), digest)

    def test_version_comparison(self) -> None:
        self.assertTrue(is_newer("v0.10.0", "0.9.9"))
        self.assertFalse(is_newer("0.2.0", "0.2.0"))


if __name__ == "__main__":
    unittest.main()

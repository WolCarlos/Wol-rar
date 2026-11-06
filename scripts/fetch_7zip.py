from __future__ import annotations

import argparse
import hashlib
import platform
import subprocess
import tarfile
import tempfile
import urllib.request
from pathlib import Path


VERSION = "26.05"
ASSETS = {
    "7zr.exe": "9bf582b894dfa665f852d7e9112b02290a1c303c2d95a21fcdc41ffcb03143b7",
    "7z2605-extra.7z": "f27a8f103c84785026edcba1f67faf1357225df9d19ddb0b7760d55947141bc7",
    "7z2605-linux-x64.tar.xz": "22cf82fcc6eb7474f1d5bb4f02c0f8ebd573a718089d6833dc30f8350a3b232e",
    "7z2605-mac.tar.xz": "12ef6830317743cfd3f0ab1724bde43fcc270d8b6d8564f8c12c2a2b7f993570",
}
BASE_URL = f"https://github.com/ip7z/7zip/releases/download/{VERSION}"


def download_verified(name: str, destination: Path) -> None:
    expected = ASSETS[name]
    with urllib.request.urlopen(f"{BASE_URL}/{name}", timeout=60) as response:
        data = response.read()
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected:
        raise RuntimeError(f"Hash non valido per {name}: {actual}")
    destination.write_bytes(data)


def fetch_windows(root: Path, temp: Path) -> None:
    destination = root / "vendor" / "7zip"
    if not (destination / "x64" / "7za.exe").is_file():
        bootstrap = temp / "7zr.exe"
        bundle = temp / "extra.7z"
        download_verified("7zr.exe", bootstrap)
        download_verified("7z2605-extra.7z", bundle)
        destination.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            [str(bootstrap), "x", str(bundle), f"-o{destination}", "-y"],
            check=True,
            shell=False,
        )


def fetch_posix(root: Path, temp: Path, selected: str) -> None:
    asset = "7z2605-linux-x64.tar.xz" if selected == "linux" else "7z2605-mac.tar.xz"
    folder = "linux-x64" if selected == "linux" else "macos"
    destination = root / "vendor" / "7zip" / folder
    if not (destination / "7zz").is_file():
        archive = temp / asset
        download_verified(asset, archive)
        destination.mkdir(parents=True, exist_ok=True)
        with tarfile.open(archive, "r:xz") as bundle:
            for member in bundle.getmembers():
                target = (destination / member.name).resolve()
                if not target.is_relative_to(destination.resolve()):
                    raise RuntimeError(f"Percorso non sicuro nel pacchetto 7-Zip: {member.name}")
            bundle.extractall(destination, filter="tar")
        (destination / "7zz").chmod((destination / "7zz").stat().st_mode | 0o111)


def download_license(root: Path) -> None:
    destination = root / "vendor" / "7zip" / "License.txt"
    if not destination.is_file():
        destination.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen("https://www.7-zip.org/license.txt", timeout=60) as response:
            destination.write_bytes(response.read())


def main() -> None:
    parser = argparse.ArgumentParser(description="Scarica i motori 7-Zip ufficiali con hash verificato")
    parser.add_argument("--platform", choices=("windows", "linux", "macos", "all"), default="all")
    args = parser.parse_args()
    selected = args.platform
    if selected == "all":
        current = {"Windows": "windows", "Linux": "linux", "Darwin": "macos"}.get(platform.system())
        if current is None:
            raise RuntimeError("Piattaforma non supportata")
        selected = current
    root = Path(__file__).resolve().parents[1]
    (root / "vendor").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(dir=root / "vendor") as temporary:
        temp = Path(temporary)
        if selected == "windows":
            fetch_windows(root, temp)
        else:
            fetch_posix(root, temp, selected)
    download_license(root)
    print(f"7-Zip {VERSION} pronto per {selected}.")


if __name__ == "__main__":
    main()

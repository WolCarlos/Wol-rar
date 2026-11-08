from __future__ import annotations

import hashlib
import platform
import shutil
import tarfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.7.0"


def selected_platform() -> str:
    system = platform.system().lower()
    if system == "windows":
        return "windows"
    if system == "linux":
        return "linux"
    if system == "darwin":
        return "macos"
    raise RuntimeError(f"Sistema non supportato: {system}")


def asset_name(selected: str) -> str:
    if selected == "windows":
        return "Wol-rar-Windows-x64.exe"
    if selected == "linux":
        return "Wol-rar-Linux-x64.tar.gz"
    return "Wol-rar-macOS.tar.gz"


def main() -> None:
    selected = selected_platform()
    release = ROOT / "release"
    release.mkdir(exist_ok=True)
    asset = release / asset_name(selected)
    if selected == "windows":
        shutil.copy2(ROOT / "dist" / "Wol-rar.exe", asset)
    elif selected == "linux":
        executable = ROOT / "dist" / "Wol-rar"
        executable.chmod(executable.stat().st_mode | 0o111)
        with tarfile.open(asset, "w:gz") as bundle:
            bundle.add(executable, arcname="Wol-rar")
            bundle.add(ROOT / "THIRD_PARTY_NOTICES.md", arcname="THIRD_PARTY_NOTICES.md")
            bundle.add(ROOT / "WOLRAR_FORMAT.md", arcname="WOLRAR_FORMAT.md")
            bundle.add(ROOT / "WOLRAR_FORMAT_V2.md", arcname="WOLRAR_FORMAT_V2.md")
            bundle.add(ROOT / "resources" / "changelog.json", arcname="changelog.json")
    else:
        with tarfile.open(asset, "w:gz") as bundle:
            bundle.add(ROOT / "dist" / "Wol-rar.app", arcname="Wol-rar.app")
            bundle.add(ROOT / "THIRD_PARTY_NOTICES.md", arcname="THIRD_PARTY_NOTICES.md")
            bundle.add(ROOT / "WOLRAR_FORMAT.md", arcname="WOLRAR_FORMAT.md")
            bundle.add(ROOT / "WOLRAR_FORMAT_V2.md", arcname="WOLRAR_FORMAT_V2.md")
            bundle.add(ROOT / "resources" / "changelog.json", arcname="changelog.json")
    digest = hashlib.sha256(asset.read_bytes()).hexdigest()
    with (release / f"{asset.name}.sha256").open("w", encoding="utf-8") as output:
        output.write(f"{digest}  {asset.name}\n")
    print(f"{asset.name}: {digest}")
    print(f"Versione: {VERSION}")


if __name__ == "__main__":
    main()

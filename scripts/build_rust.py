from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.7.0"


def main() -> None:
    cargo = shutil.which("cargo")
    if cargo is None and os.name == "nt":
        cargo = str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    if cargo is None:
        raise RuntimeError("Cargo non trovato nel PATH")
    subprocess.run([cargo, "build", "--release", "--manifest-path", str(ROOT / "rust" / "Cargo.toml")], cwd=ROOT, check=True)
    executable = ROOT / "rust" / "target" / "release" / ("wol-rar.exe" if os.name == "nt" else "wol-rar")
    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    if os.name == "nt":
        shutil.copy2(executable, dist / "Wol-rar.exe")
        return
    if os.uname().sysname == "Darwin":
        app = dist / "Wol-rar.app" / "Contents"
        (app / "MacOS").mkdir(parents=True, exist_ok=True)
        (app / "Resources").mkdir(parents=True, exist_ok=True)
        shutil.copy2(executable, app / "MacOS" / "Wol-rar")
        shutil.copy2(ROOT / "assets" / "wol-rar.png", app / "Resources" / "wol-rar.png")
        (app / "Info.plist").write_text(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n"
            "<plist version=\"1.0\"><dict>"
            "<key>CFBundleName</key><string>Wol-rar</string>"
            "<key>CFBundleDisplayName</key><string>Wol-rar</string>"
            "<key>CFBundleIdentifier</key><string>com.wolcarlos.wolrar</string>"
            "<key>CFBundleVersion</key><string>0.7.0</string>"
            "<key>CFBundleShortVersionString</key><string>0.7.0</string>"
            "<key>CFBundleExecutable</key><string>Wol-rar</string>"
            "<key>CFBundleIconFile</key><string>wol-rar.png</string>"
            "<key>CFBundlePackageType</key><string>APPL</string>"
            "<key>LSMinimumSystemVersion</key><string>10.13</string>"
            "</dict></plist>\n",
            encoding="utf-8",
        )
    else:
        shutil.copy2(executable, dist / "Wol-rar")


if __name__ == "__main__":
    main()

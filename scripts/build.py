from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from wolrar.app_context import current_platform, engine_relative_path


ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    selected = current_platform()
    engine = ROOT / engine_relative_path(selected)
    if not engine.is_file():
        raise RuntimeError(f"Motore mancante: {engine}; esegui scripts/fetch_7zip.py")
    separator = os.pathsep
    command = [
        sys.executable,
        "-m",
        "PyInstaller",
        "--noconfirm",
        "--clean",
        "--noupx",
        "--name",
        "Wol-rar",
        "--paths",
        str(ROOT / "src"),
        "--distpath",
        str(ROOT / "dist"),
        "--workpath",
        str(ROOT / "build"),
        "--specpath",
        str(ROOT / "build"),
        "--add-data",
        f"{engine.parent}{separator}vendor/7zip/{engine.parent.name}",
        "--add-data",
        f"{ROOT / 'vendor' / '7zip' / 'License.txt'}{separator}vendor/7zip",
        "--add-data",
        f"{ROOT / 'resources' / 'changelog.json'}{separator}resources",
        "--add-data",
        f"{ROOT / 'assets'}{separator}assets",
        "--add-data",
        f"{ROOT / '7ZIP-LICENSE.txt'}{separator}.",
        "--windowed",
    ]
    if selected == "macos":
        command.extend([
            "--onedir",
            "--icon",
            str(ROOT / "assets" / "wol-rar.png"),
            "--osx-bundle-identifier",
            "com.wolcarlos.wolrar",
            "--target-architecture",
            "universal2",
        ])
    else:
        command.append("--onefile")
    if selected == "windows":
        command.extend([
            "--icon",
            str(ROOT / "assets" / "wol-rar.ico"),
            "--version-file",
            str(ROOT / "scripts" / "windows-version.txt"),
        ])
    command.append(str(ROOT / "src" / "wolrar" / "__main__.py"))
    subprocess.run(command, cwd=ROOT, check=True)


if __name__ == "__main__":
    main()

from __future__ import annotations

import os
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VERSION = "0.5.0"


def find_compiler() -> Path:
    candidates = [
        Path(os.environ.get("ISCC", "")),
        ROOT / "build" / "inno" / "ISCC.exe",
        ROOT.parent / ".tools" / "inno" / "ISCC.exe",
        Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Inno Setup 7" / "ISCC.exe",
        Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Inno Setup 7" / "ISCC.exe",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    raise RuntimeError("ISCC.exe non trovato; esegui scripts/install_inno.py")


def main() -> None:
    if os.name != "nt":
        raise RuntimeError("L'installer Windows può essere compilato solo su Windows.")
    if not (ROOT / "dist" / "Wol-rar.exe").is_file():
        raise RuntimeError("Build Windows mancante; esegui prima scripts/build.py")
    compiler = find_compiler()
    subprocess.run(
        [
            str(compiler),
            "--no-ide-signtools",
            f"--define=AppVersion={VERSION}",
            str(ROOT / "installer" / "wol-rar.iss"),
        ],
        cwd=ROOT / "installer",
        check=True,
    )
    installer = ROOT / "release" / "Wol-rar-Setup-x64.exe"
    if not installer.is_file():
        raise RuntimeError("Installer non generato.")
    print(installer)


if __name__ == "__main__":
    main()

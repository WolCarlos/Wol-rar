#!/usr/bin/env python3
"""Verifica statica e riproducibile delle icone e dell'integrazione Inno Setup."""

from __future__ import annotations

import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
REQUIRED_SIZES = {16, 24, 32, 48, 64, 128, 256}
ICON_NAMES = {
    "wol-rar",
    "wol-rar-installer",
    "wol-rar-uninstall",
    "wol-rar-shortcut",
    "wol-rar-archive",
    "wol-rar-sfx",
    "menu-open",
    "menu-create",
    "menu-extract",
    "menu-test",
    "menu-update",
    "menu-info",
    "menu-installer",
}


def ico_sizes(path: Path) -> set[int]:
    data = path.read_bytes()
    reserved, kind, count = struct.unpack_from("<HHH", data)
    assert (reserved, kind) == (0, 1), f"Header ICO non valido: {path.name}"
    assert count > 0 and len(data) >= 6 + count * 16, f"ICO troncata: {path.name}"
    sizes: set[int] = set()
    for index in range(count):
        width, height = data[6 + index * 16 : 8 + index * 16]
        width = 256 if width == 0 else width
        height = 256 if height == 0 else height
        assert width == height, f"Icona non quadrata: {path.name}"
        sizes.add(width)
    return sizes


def verify_png(path: Path) -> None:
    data = path.read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", f"PNG non valido: {path.name}"
    width, height = struct.unpack_from(">II", data, 16)
    assert width == 256 and height == 256, f"PNG 256 px mancante: {path.name}"


def main() -> None:
    for name in sorted(ICON_NAMES):
        ico = ASSETS / f"{name}.ico"
        png = ASSETS / f"{name}.png"
        assert ico.is_file() and png.is_file(), f"Asset mancante: {name}"
        missing = REQUIRED_SIZES - ico_sizes(ico)
        assert not missing, f"Risoluzioni mancanti in {ico.name}: {sorted(missing)}"
        verify_png(png)

    installer = (ROOT / "installer" / "wol-rar.iss").read_text(encoding="utf-8")
    for reference in (
        "SetupIconFile=..\\assets\\wol-rar-installer.ico",
        "UninstallDisplayIcon={app}\\assets\\wol-rar-uninstall.ico",
        "wol-rar-shortcut.ico",
        "wol-rar-archive.ico,0",
        'Software\\Classes\\.wolrar',
        'Software\\Classes\\Wol-rar.wolrar\\DefaultIcon',
        "uninsdeletevalue uninsdeletekeyifempty",
    ):
        assert reference in installer, f"Integrazione installer mancante: {reference}"
    for extension in (".7z", ".zip", ".rar", ".tar", ".gz", ".bz2", ".xz"):
        association = f'Software\\Classes\\{extension}"; ValueType: string; ValueName: ""'
        assert association not in installer, f"Associazione predefinita modificata: {extension}"
    assert "iconcache" not in installer.lower(), "La cache icone globale non deve essere modificata"

    build_resource = (ROOT / "rust" / "build.rs").read_text(encoding="utf-8")
    assert 'set_icon("../assets/wol-rar.ico")' in build_resource
    print(f"OK: {len(ICON_NAMES)} icone multi-dimensione e integrazione .wolrar verificate")


if __name__ == "__main__":
    main()

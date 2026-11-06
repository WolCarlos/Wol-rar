from __future__ import annotations

import platform
import sys
from pathlib import Path


def resource_root() -> Path:
    frozen_root = getattr(sys, "_MEIPASS", None)
    if frozen_root:
        return Path(frozen_root)
    return Path(__file__).resolve().parents[2]


def resource_path(*parts: str) -> Path:
    return resource_root().joinpath(*parts)


def current_platform(system: str | None = None) -> str:
    normalized = (system or platform.system()).lower()
    if normalized == "windows":
        return "windows"
    if normalized == "linux":
        return "linux"
    if normalized in {"darwin", "macos"}:
        return "macos"
    raise RuntimeError(f"Sistema operativo non supportato: {normalized}")


def engine_relative_path(system: str | None = None) -> Path:
    selected = current_platform(system)
    if selected == "windows":
        return Path("vendor/7zip/x64/7za.exe")
    if selected == "linux":
        return Path("vendor/7zip/linux-x64/7zz")
    return Path("vendor/7zip/macos/7zz")


def release_asset_name(system: str | None = None) -> str:
    selected = current_platform(system)
    if selected == "windows":
        return "Wol-rar-Windows-x64.exe"
    if selected == "linux":
        return "Wol-rar-Linux-x64.tar.gz"
    return "Wol-rar-macOS.tar.gz"

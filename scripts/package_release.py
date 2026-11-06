from __future__ import annotations

import hashlib
import shutil
import tarfile
from pathlib import Path

from wolrar import __version__
from wolrar.app_context import current_platform, release_asset_name


ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    selected = current_platform()
    release = ROOT / "release"
    release.mkdir(exist_ok=True)
    asset = release / release_asset_name(selected)
    if selected == "windows":
        shutil.copy2(ROOT / "dist" / "Wol-rar.exe", asset)
    elif selected == "linux":
        executable = ROOT / "dist" / "Wol-rar"
        executable.chmod(executable.stat().st_mode | 0o111)
        with tarfile.open(asset, "w:gz") as bundle:
            bundle.add(executable, arcname="Wol-rar")
            bundle.add(ROOT / "7ZIP-LICENSE.txt", arcname="7ZIP-LICENSE.txt")
            bundle.add(ROOT / "resources" / "changelog.json", arcname="changelog.json")
    else:
        with tarfile.open(asset, "w:gz") as bundle:
            bundle.add(ROOT / "dist" / "Wol-rar.app", arcname="Wol-rar.app")
            bundle.add(ROOT / "7ZIP-LICENSE.txt", arcname="7ZIP-LICENSE.txt")
            bundle.add(ROOT / "resources" / "changelog.json", arcname="changelog.json")
    digest = hashlib.sha256(asset.read_bytes()).hexdigest()
    with (release / f"{asset.name}.sha256").open("w", encoding="utf-8") as output:
        output.write(f"{digest}  {asset.name}\n")
    print(f"{asset.name}: {digest}")
    print(f"Versione: {__version__}")


if __name__ == "__main__":
    main()

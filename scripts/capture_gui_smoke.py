#!/usr/bin/env python3
"""Cattura gli smoke test GUI reali in Windows, senza lasciare processi attivi."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import time

from PIL import ImageGrab

ROOT = Path(__file__).resolve().parents[1]
RESULTS = ROOT / "analysis" / "results"
RESULTS.mkdir(parents=True, exist_ok=True)


def main() -> None:
    executable = ROOT / "dist" / "Wol-rar.exe"
    assert executable.is_file(), f"Build mancante: {executable}"
    for theme, flag in (("dark", "--gui-smoke-test"), ("light", "--gui-smoke-test-light")):
        process = subprocess.Popen(
            [str(executable), flag],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
        )
        try:
            time.sleep(1.2)
            image = ImageGrab.grab()
            destination = RESULTS / f"ui-{theme}-next.png"
            image.save(destination)
            output, _ = process.communicate(timeout=15)
        except BaseException:
            process.kill()
            process.wait(timeout=5)
            raise
        assert process.returncode == 0, output
        print(json.dumps({"theme": theme, "screenshot": str(destination), "output": output.strip()}, ensure_ascii=False))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Smoke test locale del formato .wolrar attraverso la CLI pubblica."""

import json
import os
from pathlib import Path
import secrets
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "rust" / "target" / "debug" / ("wol-rar.exe" if os.name == "nt" else "wol-rar")


def run(*args: object, password: str | None = None, ok: bool = True) -> dict:
    completed = subprocess.run(
        [str(BINARY), *(str(arg) for arg in args), "--json"],
        input=password,
        text=True,
        encoding="utf-8",
        capture_output=True,
        cwd=ROOT,
        check=False,
    )
    output = completed.stdout if completed.stdout.strip() else completed.stderr
    try:
        result = json.loads(output)
    except json.JSONDecodeError as error:
        raise AssertionError(f"Output CLI non JSON: {output!r}") from error
    assert result.get("ok") is ok, result
    assert (completed.returncode == 0) is ok, result
    return result


def main() -> None:
    assert BINARY.is_file(), f"Build debug mancante: {BINARY}"
    (ROOT / "analysis").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="wol-cli-", dir=ROOT / "analysis") as temporary:
        root = Path(temporary)
        first = root / "primo àèé.txt"
        second = root / "secondo.txt"
        first.write_text("contenuto Unicode 日本語", encoding="utf-8")
        second.write_bytes(os.urandom(2 * 1024 * 1024))

        formats = run("formats")
        assert "wolrar" in formats["create"] and "wolrar" in formats["read"]
        assert formats["wolrar"]["write_version"] == 2 and formats["wolrar"]["volumes"] is True

        archive = root / "normale.wolrar"
        created = run("create", first, second, "--output", archive, "--comment", "Commento 日本語")
        assert len(created["sources"]) == 2
        info = run("info", archive)
        assert info["comment"] == "Commento 日本語" and info["entry_count"] == 2, info
        assert run("list", archive)["count"] == 2
        run("test", archive)
        plain_destination = root / "estratto"
        run("extract", archive, "--destination", plain_destination)
        assert (plain_destination / first.name).read_bytes() == first.read_bytes()
        assert (plain_destination / second.name).read_bytes() == second.read_bytes()

        multi = root / "volumi.wolrar"
        run("create", second, "--output", multi, "--volume-size", 65536, "--recovery-percent", 10)
        multi_info = run("info", multi)
        assert multi_info["format_version"] == 2 and multi_info["volume_count"] > 1, multi_info
        assert (root / "volumi.w02").is_file()
        run("test", multi)
        multi_destination = root / "volumi-estratti"
        run("extract", multi, "--destination", multi_destination)
        assert (multi_destination / second.name).read_bytes() == second.read_bytes()
        assert multi_info["recovery_count"] > 0
        (root / "volumi.w02").unlink()
        repaired = root / "riparato"
        report = run("repair", multi, "--destination", repaired)
        assert report["report"]["verified"] is True
        repaired_archive = repaired / "volumi.wolrar"
        run("test", repaired_archive)
        repaired_destination = root / "riparato-estratto"
        run("extract", repaired_archive, "--destination", repaired_destination)
        assert (repaired_destination / second.name).read_bytes() == second.read_bytes()

        password = secrets.token_urlsafe(32)
        encrypted = root / "cifrato.wolrar"
        run("create", first, "--output", encrypted, "--password-stdin", password=password)
        assert run("info", encrypted)["encrypted"] is True
        run("test", encrypted, "--password-stdin", password=password)
        wrong_destination = root / "errato"
        run("extract", encrypted, "--destination", wrong_destination, "--password-stdin", password=password + "x", ok=False)
        assert not any(path.is_file() for path in wrong_destination.rglob("*"))
        encrypted_destination = root / "decifrato"
        run("extract", encrypted, "--destination", encrypted_destination, "--password-stdin", password=password)
        assert (encrypted_destination / first.name).read_bytes() == first.read_bytes()

        sfx = root / "autoestraente.exe"
        report = run("sfx", archive, "--output", sfx)
        assert report["report"]["archive_bytes"] > 0 and sfx.is_file()
        benchmark = run("benchmark", "--size-mib", 1)
        assert benchmark["report"]["verified"] is True

    print(json.dumps({"ok": True, "operation": "wolrar-cli-smoke"}, ensure_ascii=False))


if __name__ == "__main__":
    main()

from __future__ import annotations

import hashlib
import os
import subprocess
import urllib.request
from pathlib import Path


VERSION = "7.1.1"
URL = f"https://github.com/jrsoftware/issrc/releases/download/is-7_1_1/innosetup-{VERSION}-x64.exe"
SHA256 = "8a5a6711c3766a6e5fb74f280ba0e3551e719ce94538d901fdbd1f8f9920b3f0"


def main() -> None:
    if os.name != "nt":
        raise RuntimeError("Inno Setup è disponibile solo su Windows.")
    root = Path(__file__).resolve().parents[1]
    destination = root / "build" / "inno"
    compiler = destination / "ISCC.exe"
    if compiler.is_file():
        print(compiler)
        return
    destination.parent.mkdir(exist_ok=True)
    installer = destination.parent / f"innosetup-{VERSION}-x64.exe"
    urllib.request.urlretrieve(URL, installer)
    if hashlib.sha256(installer.read_bytes()).hexdigest() != SHA256:
        raise RuntimeError("Hash Inno Setup non valido.")
    signature_command = (
        f"$s=Get-AuthenticodeSignature -FilePath '{installer}'; "
        "if ($null -eq $s -or $null -eq $s.SignerCertificate) { throw 'Certificato Inno Setup assente' }; "
        "if ($s.Status -ne 'Valid') { throw ('Firma Inno Setup non valida: ' + $s.Status) }"
    )
    subprocess.run(
        ["powershell.exe", "-NoProfile", "-Command", signature_command],
        check=True,
        capture_output=True,
        text=True,
    )
    subprocess.run(
        [str(installer), "/VERYSILENT", "/NORESTART", "/CURRENTUSER", f"/DIR={destination}"],
        check=True,
    )
    if not compiler.is_file():
        raise RuntimeError("ISCC.exe non trovato dopo l'installazione.")
    print(compiler)


if __name__ == "__main__":
    main()

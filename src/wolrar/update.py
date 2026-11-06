from __future__ import annotations

import hashlib
import json
import re
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from wolrar import __version__
from wolrar.app_context import release_asset_name


REPOSITORY = "WolCarlos/Wol-rar"
LATEST_RELEASE_URL = f"https://api.github.com/repos/{REPOSITORY}/releases/latest"
USER_AGENT = f"Wol-rar/{__version__}"


class UpdateError(RuntimeError):
    """Errore durante controllo o download di un aggiornamento."""


@dataclass(frozen=True)
class ReleaseAsset:
    name: str
    url: str
    size: int


@dataclass(frozen=True)
class ReleaseInfo:
    version: str
    notes: str
    published_at: str
    assets: tuple[ReleaseAsset, ...]

    def asset(self, name: str) -> ReleaseAsset:
        for asset in self.assets:
            if asset.name == name:
                return asset
        raise UpdateError(f"La release non contiene il pacchetto {name}.")


Fetcher = Callable[[str, int], bytes]


def version_key(version: str) -> tuple[int, int, int]:
    match = re.search(r"(\d+)\.(\d+)\.(\d+)", version)
    if not match:
        raise UpdateError(f"Versione non valida: {version}")
    return tuple(int(part) for part in match.groups())


def is_newer(candidate: str, current: str = __version__) -> bool:
    return version_key(candidate) > version_key(current)


def _fetch_bytes(url: str, maximum_bytes: int) -> bytes:
    if urllib.parse.urlparse(url).scheme != "https":
        raise UpdateError("Sono consentiti soltanto download HTTPS.")
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github+json, application/octet-stream",
            "User-Agent": USER_AGENT,
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=45) as response:
            if urllib.parse.urlparse(response.geturl()).scheme != "https":
                raise UpdateError("Reindirizzamento HTTPS non valido.")
            data = response.read(maximum_bytes + 1)
    except (urllib.error.URLError, TimeoutError) as error:
        raise UpdateError(f"Connessione a GitHub non riuscita: {error.reason if hasattr(error, 'reason') else error}") from error
    if len(data) > maximum_bytes:
        raise UpdateError("Il download supera la dimensione massima consentita.")
    return data


class UpdateClient:
    def __init__(self, fetcher: Fetcher | None = None) -> None:
        self._fetch = fetcher or _fetch_bytes

    def check_latest(self) -> ReleaseInfo:
        try:
            payload = json.loads(self._fetch(LATEST_RELEASE_URL, 2 * 1024 * 1024))
            assets = tuple(
                ReleaseAsset(
                    name=str(item["name"]),
                    url=str(item["browser_download_url"]),
                    size=int(item.get("size", 0)),
                )
                for item in payload["assets"]
            )
            release = ReleaseInfo(
                version=str(payload["tag_name"]),
                notes=str(payload.get("body") or "Nessuna nota di rilascio."),
                published_at=str(payload.get("published_at") or ""),
                assets=assets,
            )
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            raise UpdateError("Risposta GitHub non valida.") from error
        version_key(release.version)
        release.asset(release_asset_name())
        release.asset("SHA256SUMS.txt")
        return release

    def download(self, release: ReleaseInfo, destination: Path) -> str:
        asset = release.asset(release_asset_name())
        checksums = self._fetch(release.asset("SHA256SUMS.txt").url, 128 * 1024).decode("utf-8")
        expected = _checksum_for(checksums, asset.name)
        data = self._fetch(asset.url, 512 * 1024 * 1024)
        actual = hashlib.sha256(data).hexdigest()
        if actual != expected:
            raise UpdateError("Checksum dell'aggiornamento non valido; file scartato.")
        destination = Path(destination).resolve()
        if destination.exists():
            raise UpdateError("Il file di destinazione esiste già; scegli un altro nome.")
        destination.parent.mkdir(parents=True, exist_ok=True)
        partial = destination.with_name(destination.name + ".part")
        try:
            partial.write_bytes(data)
            partial.replace(destination)
        finally:
            if partial.exists():
                partial.unlink()
        return actual


def _checksum_for(contents: str, asset_name: str) -> str:
    for line in contents.splitlines():
        match = re.fullmatch(r"([0-9a-fA-F]{64})\s+\*?(.+)", line.strip())
        if match and match.group(2) == asset_name:
            return match.group(1).lower()
    raise UpdateError(f"Checksum mancante per {asset_name}.")

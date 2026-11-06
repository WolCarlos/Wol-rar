# Wol-rar

Archiviatore privato per **Windows x64, Ubuntu x64 e macOS**, con interfaccia Tkinter e motore 7-Zip 26.05. Nessun account, pubblicità, tracking o telemetria.

## Funzionalità

- Apertura ed elenco di archivi 7z, ZIP, RAR e altri formati supportati da 7-Zip.
- Creazione di archivi **7z e ZIP** senza sovrascrittura automatica.
- Estrazione con controlli contro path traversal, percorsi assoluti e flussi NTFS; i file esistenti vengono mantenuti.
- Test di integrità degli archivi.
- Scheda **Novità e aggiornamenti** con changelog locale e note della release più recente.
- Download diretto dell'aggiornamento per il sistema corrente, con verifica SHA-256 obbligatoria.
- RAR soltanto in lettura: il progetto non ricrea l'algoritmo proprietario di compressione RAR.

Le operazioni sugli archivi sono locali. La rete viene utilizzata soltanto quando l'utente preme **Controlla aggiornamenti** o avvia il download; la richiesta contiene unicamente lo User-Agent `Wol-rar/<versione>` necessario all'API GitHub.

## Download e avvio

Le build pubblicate sono disponibili in [GitHub Releases](https://github.com/WolCarlos/Wol-rar/releases).

- **Windows:** avviare `Wol-rar-Windows-x64.exe`.
- **Ubuntu:** estrarre `Wol-rar-Linux-x64.tar.gz` e avviare `./Wol-rar`. Servono le librerie grafiche Tk/X11 già presenti nei normali desktop Ubuntu.
- **macOS:** estrarre `Wol-rar-macOS.tar.gz` e aprire `Wol-rar.app`.

Prima di installare un aggiornamento chiudere l'applicazione. Il download non sostituisce automaticamente il programma in esecuzione.

## Firme e attendibilità

- L'asset Windows della release 0.2.1 include una firma Authenticode **autofirmata**, utile per verificarne l'integrità ma non pubblicamente attendibile.
- La pipeline supporta certificati Windows tramite `WINDOWS_CERTIFICATE_BASE64` e `WINDOWS_CERTIFICATE_PASSWORD`.
- Su macOS la build usa firma ad-hoc; con certificato Developer ID configura `MACOS_CERTIFICATE_BASE64`, `MACOS_CERTIFICATE_PASSWORD` e `MACOS_CERTIFICATE_NAME`.
- La notarization Apple richiede anche `APPLE_ID`, `APPLE_PASSWORD` e `APPLE_TEAM_ID`.
- `SHA256SUMS.txt` copre tutti i pacchetti. Se configurata, la chiave `RELEASE_GPG_PRIVATE_KEY` con `RELEASE_GPG_PASSPHRASE` produce anche la firma OpenPGP dei checksum.

Per eliminare gli avvisi SmartScreen e Gatekeeper servono certificati pubblici e, per macOS, la notarization: non usare certificati autofirmati come se fossero pubblicamente attendibili.

## Sviluppo

Richiede Python 3.12 o successivo.

```bash
python scripts/fetch_7zip.py
PYTHONPATH=src python -m unittest discover -s tests -v
PYTHONPATH=src python -m wolrar
```

Build nativa sul sistema corrente:

```bash
python -m pip install pyinstaller==6.19.0 pillow
python scripts/make_icon.py
PYTHONPATH=src python scripts/build.py
PYTHONPATH=src python scripts/package_release.py
```

PyInstaller non effettua cross-compile: Windows, Ubuntu e macOS vengono compilati sui rispettivi runner tramite `.github/workflows/release.yml`.

## Rilascio

1. Aggiornare `__version__` in `src/wolrar/__init__.py`, `pyproject.toml`, `scripts/windows-version.txt`, `resources/changelog.json` e `RELEASE_NOTES.md`.
2. Eseguire tutti i test.
3. Creare e pubblicare un tag `vX.Y.Z`.
4. La workflow compila i tre sistemi, firma quando i segreti sono configurati e pubblica gli asset con `SHA256SUMS.txt`.

## Licenze

Il codice applicativo è source-available e non è concesso con una licenza open-source. I componenti 7-Zip mantengono le rispettive licenze LGPL/BSD/unRAR, riportate in `7ZIP-LICENSE.txt` e accessibili dalla scheda **Novità → Licenze** dell'applicazione.

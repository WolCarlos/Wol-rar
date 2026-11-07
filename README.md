# Wol-rar

Archiviatore privato per **Windows x64, Ubuntu x64 e macOS**, con interfaccia Tkinter e motore 7-Zip 26.05. Nessun account, pubblicità, tracking o telemetria.

## Funzionalità

- Apertura ed elenco di archivi 7z, ZIP, RAR e altri formati supportati da 7-Zip.
- Creazione di archivi **7z e ZIP** con livello di compressione selezionabile e senza sovrascrittura automatica.
- Estrazione con controlli contro path traversal, percorsi assoluti e flussi NTFS; i file esistenti vengono mantenuti.
- Test di integrità degli archivi.
- UX con toolbar iconata, filtro istantaneo, riepilogo compressione, ordinamento colonne e scorciatoie.
- Scheda **Novità e aggiornamenti** con changelog locale e note della release più recente.
- Download diretto dell'aggiornamento per il sistema corrente, con verifica SHA-256 obbligatoria.
- RAR soltanto in lettura: il progetto non ricrea l'algoritmo proprietario di compressione RAR.

Le operazioni sugli archivi sono locali. La rete viene utilizzata soltanto quando l'utente preme **Controlla aggiornamenti** o avvia il download; la richiesta contiene unicamente lo User-Agent `Wol-rar/<versione>` necessario all'API GitHub.

## Installer e integrazione Windows

`Wol-rar-Setup-x64.exe` installa l'app nel profilo utente, senza richiedere amministratore. Include:

- collegamento nel menu Start e collegamento desktop opzionale;
- disinstallatore con rimozione delle integrazioni create;
- menu **Wol-rar** raggruppato nel tasto destro di Explorer;
- **Apri, Estrai in una nuova cartella e Verifica integrità** per gli archivi;
- **Comprimi in un archivio 7z o ZIP** per file e cartelle;
- icone coordinate per app e comandi, originali e non copiate da WinRAR.

Su Windows 11 il menu classico può comparire sotto **Mostra altre opzioni**, in base alle impostazioni di Explorer. Il programma non cambia l'applicazione predefinita associata agli archivi.

È disponibile anche il binario portabile `Wol-rar-Windows-x64.exe`, che non modifica il registro. Ubuntu e macOS usano rispettivamente `Wol-rar-Linux-x64.tar.gz` e `Wol-rar-macOS.tar.gz`.

Prima di installare un aggiornamento chiudere l'applicazione. Il download dalla scheda Novità non avvia automaticamente l'installer.

## Firme e attendibilità

- Gli asset Windows della release 0.3.0 includono una firma Authenticode **autofirmata**, utile per verificarne l'integrità ma non pubblicamente attendibile.
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

Installer Windows con Inno Setup 7.1.1:

```bash
python scripts/install_inno.py
PYTHONPATH=src python scripts/build_installer.py
```

Comandi usati anche da Explorer:

```text
Wol-rar.exe --open ARCHIVIO
Wol-rar.exe --extract ARCHIVIO
Wol-rar.exe --test ARCHIVIO
Wol-rar.exe --create FILE_O_CARTELLA
```

PyInstaller non effettua cross-compile: Windows, Ubuntu e macOS vengono compilati sui rispettivi runner tramite `.github/workflows/release.yml`.

## Rilascio

1. Aggiornare `__version__` in `src/wolrar/__init__.py`, `pyproject.toml`, `scripts/windows-version.txt`, `resources/changelog.json` e `RELEASE_NOTES.md`.
2. Eseguire tutti i test.
3. Creare e pubblicare un tag `vX.Y.Z`.
4. La workflow compila i tre sistemi e l'installer, firma quando i segreti sono configurati e pubblica gli asset con `SHA256SUMS.txt`.

## Licenze

Il codice applicativo è source-available e non è concesso con una licenza open-source. I componenti 7-Zip mantengono le rispettive licenze LGPL/BSD/unRAR, riportate in `7ZIP-LICENSE.txt` e accessibili dalla scheda **Novità → Licenze** dell'applicazione.

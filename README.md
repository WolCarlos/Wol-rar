# Wol-rar

Archiviatore privato scritto in Rust per Windows x64, Ubuntu x64 e macOS, con GUI `eframe`, CLI agent-first e nessuna telemetria. Versione: **0.7.0**.

## Funzionalità principali

- **`.wolrar` v2 aperto:** Zstandard, BLAKE3, streaming e lettura v1.
- Cifratura Argon2id + XChaCha20-Poly1305 per chunk da 1 MiB.
- Volumi multipli, parity Reed–Solomon e riparazione senza sovrascrittura.
- Deduplicazione opzionale e SFX a volume unico.
- Creazione 7z, ZIP, TAR e varianti compresse; RAR in sola lettura.
- Lettura ISO/Joliet, CAB, ARJ, LHA/LZH, UUE e Unix `.Z`, con estrazione sicura e fixture controllate.
- Eliminazione, ridenominazione, aggiunta, aggiornamento e conversione con riscrittura verificata e rollback.
- Metadati estesi JSON/GUI, wizard guidati e benchmark comparativo con risultati JSON.
- Estrazione transazionale con protezione da traversal, ADS, link e file esistenti.
- Tema scuro predefinito e chiaro, icone originali, tabella ordinabile, ricerca, scorciatoie e drag-and-drop.
- Installer Windows con menu Explorer raggruppato e aggiornamenti HTTPS con SHA-256, installazione guidata e cache gestita.

Le specifiche aperte sono in `WOLRAR_FORMAT.md` (v1) e `WOLRAR_FORMAT_V2.md` (v2), entrambe CC0. Lo stato e i limiti sono in `FEATURE_PARITY.md`.

## Agenti AI e CLI

Tutte le operazioni hanno una CLI equivalente con `--json`; `wol-rar agent` stampa il prompt compatto da inserire in `AGENTS.md`.

```text
wol-rar formats --json
wol-rar list ARCHIVIO --json
wol-rar info ARCHIVIO.wolrar --json
wol-rar create ORIGINE [ALTRE...] --output ARCHIVIO.wolrar --level 5 --json
wol-rar create ORIGINE --output VOLUMI.wolrar --volume-size 1048576 --recovery-percent 10 --deduplicate --json
wol-rar extract ARCHIVIO --destination CARTELLA --json
wol-rar test ARCHIVIO --json
wol-rar repair VOLUMI.wolrar --destination RIPARATO --json
wol-rar delete ARCHIVIO VOCE --json
wol-rar rename ARCHIVIO VOCE NUOVO-PERCORSO --json
wol-rar add ARCHIVIO FILE VOCE --json
wol-rar update ARCHIVIO FILE VOCE --json
wol-rar convert ARCHIVIO --output NUOVO.wolrar --json
wol-rar sfx ARCHIVIO.wolrar --output AUTO.exe --json
wol-rar benchmark --size-mib 32 --json
wol-rar benchmark --size-mib 32 --suite --output benchmark.json --json
wol-rar update-check --json
wol-rar update-download --json
wol-rar update-install CACHE/Wol-rar-Setup-x64.exe --json
wol-rar agent
```

Password soltanto con `--password-stdin` o campo volatile GUI, mai in argomenti o log.

## Download e installazione

Gli asset sono in [GitHub Releases](https://github.com/WolCarlos/Wol-rar/releases).

- **Windows:** `Wol-rar-Setup-x64.exe` oppure il portabile `Wol-rar-Windows-x64.exe`.
- **Ubuntu:** estrarre `Wol-rar-Linux-x64.tar.gz` e avviare `./Wol-rar`.
- **macOS:** estrarre `Wol-rar-macOS.tar.gz` e aprire `Wol-rar.app`.

Dalla scheda **Novità e aggiornamenti** il download avviene solo dopo un clic, nella cache dedicata del profilo utente. Dopo SHA-256 compare **Installa**; su Windows l’installer resta visibile e Wol-rar ne attende l’exit status. La cache viene ripulita dopo successo, mentre un artefatto utile resta disponibile per il retry dopo errore. Ubuntu e macOS mostrano istruzioni specifiche.

Il tasto destro contiene un solo menu **Wol-rar**, senza cambiare le associazioni predefinite. La disinstallazione rimuove le integrazioni create.

## Limiti espliciti

- Unix `.Z` non contiene digest né lunghezza originale: alcuni troncamenti possono non essere rilevabili.
- Modifica e conversione rifiutano volumi/recovery invece di prometterne una migrazione non verificata.
- RAR resta in sola lettura; non viene ricostruito l’algoritmo proprietario.
- La parità completa con WinRAR non viene dichiarata.

## Sviluppo e test

Richiede Rust stable 1.95+.

```bash
cargo test --manifest-path rust/Cargo.toml
cargo clippy --manifest-path rust/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path rust/Cargo.toml
python scripts/test_wolrar_cli.py
python scripts/test_windows_assets.py
python scripts/build_rust.py
python scripts/package_release.py
```

Build installer Windows con Inno Setup 7.1.1:

```bash
python scripts/install_inno.py
python scripts/build_installer.py
```

Smoke GUI e screenshot Windows:

```bash
./dist/Wol-rar.exe --gui-smoke-test
./dist/Wol-rar.exe --gui-smoke-test-light
python scripts/capture_gui_smoke.py
```

La workflow `.github/workflows/release.yml` esegue lint, test e smoke CLI/GUI su Windows, Ubuntu 22.04 e macOS 15 Intel, quindi pubblica gli asset.

## Firme e licenze

Windows usa Authenticode autofirmata se non è configurato un certificato reale; non è pubblicamente attendibile. macOS usa firma ad-hoc. La pipeline supporta certificati Windows, Apple Developer ID/notarization e firma OpenPGP dei checksum.

Il codice applicativo è source-available. Le specifiche `.wolrar` sono CC0 e consentono implementazioni indipendenti. Dipendenze e restrizione unRAR sono in `THIRD_PARTY_NOTICES.md`.

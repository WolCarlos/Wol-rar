# Wol-rar

Archiviatore privato **scritto in Rust** per Windows x64, Ubuntu x64 e macOS. Include GUI `eframe`, CLI agent-first e nessun account, pubblicità, tracking o telemetria.

## Funzionalità

- Creazione **7z, ZIP, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ** senza sovrascrittura automatica.
- Apertura ed elenco di 7z, ZIP, RAR e TAR compressi; RAR soltanto in lettura tramite unRAR.
- Estrazione con rifiuto di path traversal, percorsi assoluti, ADS e file esistenti.
- Verifica di integrità tramite decodifica completa in una directory temporanea.
- Opzioni AES-256 per ZIP e 7z, con password solo su stdin o campo volatile.
- GUI con filtro, operazioni in thread separato, Novità e sezione Agenti AI.
- Aggiornamenti espliciti da GitHub Releases con HTTPS, limite di download e verifica SHA-256.
- Installer Windows, binario portabile e menu Explorer raggruppato con comandi pertinenti.

La copertura aggiornata e le funzionalità ancora pianificate sono in [FEATURE_PARITY.md](FEATURE_PARITY.md).

## Agenti AI e CLI

Tutte le funzioni hanno una CLI equivalente con `--json`. Il comando `wol-rar agent` stampa il prompt compatto da copiare in `AGENTS.md`, disponibile anche nella scheda **Agenti AI**.

```text
wol-rar formats --json
wol-rar list ARCHIVIO --json
wol-rar create ORIGINE --output ARCHIVIO.7z --level 5 --json
wol-rar extract ARCHIVIO --destination CARTELLA --json
wol-rar test ARCHIVIO --json
wol-rar update-check --json
wol-rar update-download CARTELLA --json
wol-rar agent
```

Le password non vanno mai negli argomenti o nei log. Aggiungere `--password-stdin` e fornire il segreto soltanto su stdin.

## Download e installazione

Gli asset sono in [GitHub Releases](https://github.com/WolCarlos/Wol-rar/releases).

- **Windows:** `Wol-rar-Setup-x64.exe`, installer per utente senza amministratore; `Wol-rar-Windows-x64.exe` è il portabile.
- **Ubuntu:** estrarre `Wol-rar-Linux-x64.tar.gz` e avviare `./Wol-rar`.
- **macOS:** estrarre `Wol-rar-macOS.tar.gz` e aprire `Wol-rar.app`.

L'installer aggiunge un unico menu **Wol-rar** nel tasto destro: Apri/Estrai/Verifica per archivi e Comprimi per file e cartelle. Su Windows 11 può comparire sotto **Mostra altre opzioni**. Le associazioni predefinite non vengono cambiate. La disinstallazione rimuove le integrazioni create.

Le operazioni sugli archivi sono locali. La rete viene usata solo quando l'utente richiede controllo o download degli aggiornamenti, con User-Agent `Wol-rar/<versione>`.

## Firme

- App e installer Windows includono una firma Authenticode **autofirmata**, non pubblicamente attendibile.
- La pipeline supporta `WINDOWS_CERTIFICATE_BASE64` e `WINDOWS_CERTIFICATE_PASSWORD`.
- macOS usa firma ad-hoc; per Developer ID e notarization configurare `MACOS_CERTIFICATE_BASE64`, `MACOS_CERTIFICATE_PASSWORD`, `MACOS_CERTIFICATE_NAME`, `APPLE_ID`, `APPLE_PASSWORD` e `APPLE_TEAM_ID`.
- `SHA256SUMS.txt` copre tutti gli asset; `RELEASE_GPG_PRIVATE_KEY` e `RELEASE_GPG_PASSPHRASE` abilitano anche la firma OpenPGP.

## Sviluppo Rust

Richiede Rust stable 1.95 o successivo e, su Windows, il linker MSVC.

```bash
cargo test --manifest-path rust/Cargo.toml
cargo run --manifest-path rust/Cargo.toml
cargo build --release --manifest-path rust/Cargo.toml
```

Build e pacchetto sul sistema corrente:

```bash
python scripts/build_rust.py
python scripts/package_release.py
```

Installer Windows con Inno Setup 7.1.1:

```bash
python scripts/install_inno.py
python scripts/build_installer.py
```

Smoke test CLI e GUI:

```bash
cargo run --manifest-path rust/Cargo.toml -- --smoke-test
cargo run --manifest-path rust/Cargo.toml -- --gui-smoke-test
```

La workflow `.github/workflows/release.yml` installa Rust stable, esegue test e build native su Windows, Ubuntu 22.04 e macOS 15 Intel, firma quando configurato e pubblica gli asset.

## Rilascio

1. Aggiornare versione in `rust/Cargo.toml`, `rust/build.rs`, `scripts/build_rust.py`, `scripts/package_release.py`, `resources/changelog.json` e `RELEASE_NOTES.md`.
2. Aggiornare `FEATURE_PARITY.md` senza dichiarare funzionalità non testate.
3. Eseguire test, build e smoke test.
4. Creare una tag `vX.Y.Z`.

## Licenze

Il codice applicativo è source-available e non è concesso con licenza open-source. Dipendenze e restrizione unRAR sono documentate in `THIRD_PARTY_NOTICES.md`; le versioni esatte sono in `rust/Cargo.lock`.

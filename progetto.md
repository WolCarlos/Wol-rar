# Wol-rar

- **Stato Attuale:**
  - Versione 0.4.0 pubblicata: https://github.com/WolCarlos/Wol-rar/releases/tag/v0.4.0; branch `main` allineato a `origin/main`.
  - Applicazione interamente Rust 1.95+, senza runtime Python; GUI `eframe` con renderer `wgpu`.
  - Motore: `sevenz-rust2`, `zip`, `unrar`, `tar`; 7z/ZIP/TAR e varianti compresse, AES-256 testato, RAR in sola lettura.
  - Agent-first: CLI `list`, `create`, `extract`, `test`, `formats`, `update-check`, `update-download`, `agent`; JSON opzionale e password solo su stdin.
  - Prompt agenti in `agent/AGENTS.snippet.md`, disponibile via `wol-rar agent` e scheda GUI Agenti AI.
  - Windows: installer per utente con menu Explorer raggruppato, comandi `--gui`, disinstallazione pulita e binario portabile con icone/metadati.
  - Aggiornamenti GitHub Releases solo su richiesta, HTTPS, limiti download e SHA-256 obbligatorio.
  - Firme: Authenticode autofirmata su app/installer, non pubblicamente attendibile; CI supporta certificati reali; macOS ad-hoc.
  - CI release 42403796293 completata: test e smoke test verdi su Windows, Ubuntu 22.04 e macOS 15 Intel.
  - Test automatici: 9 unit test; AES, TAR, traversal, checksum, versioni e round-trip verificati.
  - Test pubblico: download installer, installazione, GUI, controllo comando ZIP, disinstallazione e pulizia registro completati; RAR multi-volume letto/testato/estratto con contenuto confrontato.
  - Parità avanzata ancora in corso secondo `FEATURE_PARITY.md`: benchmark, temi, wizard, altri formati, volumi, SFX e recovery.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt agent-first statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

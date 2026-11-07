# Wol-rar

- **Stato Attuale:**
  - Versione 0.5.0 pubblicata: https://github.com/WolCarlos/Wol-rar/releases/tag/v0.5.0; CI 42415564355 completata con successo.
  - Applicazione interamente Rust 1.95+, senza runtime Python; GUI `eframe` con renderer `wgpu`.
  - Formato aperto `.wolrar` v1 in `rust/src/wolformat.rs`, specifica CC0 in `WOLRAR_FORMAT.md`: Zstandard, BLAKE3, Argon2id, XChaCha20-Poly1305 con AAD.
  - Streaming non cifrato, limite 512 MiB per file cifrato, commenti Unicode, timestamp/permessi, estrazione transazionale senza file parziali.
  - Motore: `sevenz-rust2`, `zip`, `unrar`, `tar`; 7z/ZIP/TAR e varianti compresse, AES-256 testato, RAR in sola lettura.
  - Agent-first: CLI `list`, `info`, `create` multi-origine con commento, `extract`, `test`, `formats`, `update-check`, `update-download`, `agent`; JSON opzionale e password solo su stdin.
  - Prompt agenti in `agent/AGENTS.snippet.md`, disponibile via `wol-rar agent` e scheda GUI Agenti AI.
  - Windows: installer per utente con menu Explorer raggruppato anche per `.wolrar`, disinstallazione pulita e binario portabile con icone/metadati.
  - Aggiornamenti GitHub Releases solo su richiesta, HTTPS, limiti download e SHA-256 obbligatorio.
  - Firme: Authenticode autofirmata su app/installer, non pubblicamente attendibile; CI supporta certificati reali; macOS ad-hoc.
  - Test: 17 unit test, Clippy senza warning, smoke CLI e GUI verdi su Windows, Ubuntu 22.04 e macOS 15 Intel; download pubblico installer e checksum verificati.
  - Parità avanzata ancora in corso secondo `FEATURE_PARITY.md`: streaming cifrato oltre 512 MiB, benchmark, volumi, SFX, recovery e altri formati.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt agent-first statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

# Wol-rar

- **Stato Attuale:**
  - Versione 0.6.0 pubblicata: https://github.com/WolCarlos/Wol-rar/releases/tag/v0.6.0; CI 42424362668 completata con successo.
  - Applicazione Rust 1.95+, GUI `eframe`/`wgpu`, nessuna telemetria.
  - `.wolrar` v2 in `rust/src/wolformat_v2.rs`, specifica CC0 `WOLRAR_FORMAT_V2.md`; lettura v1 mantenuta.
  - Streaming Zstandard, BLAKE3, Argon2id e XChaCha20-Poly1305 per chunk; nessun limite applicativo v1 di 512 MiB.
  - Volumi con manifest/digest, parity Reed–Solomon e riparazione in `rust/src/wolrecovery.rs`.
  - Deduplicazione opzionale, SFX volume unico in `rust/src/sfx.rs`, benchmark offline in `rust/src/benchmark.rs`.
  - Motore 7z/ZIP/TAR/AES-256 e RAR sola lettura; estrazione transazionale e nessuna sovrascrittura.
  - GUI rinnovata: dark default, light, icone originali, riepilogo, ordinamento, ricerca, scorciatoie, drag-and-drop, opzioni v2 e riparazione.
  - CLI JSON `list`, `info`, `create`, `extract`, `test`, `repair`, `sfx`, `benchmark`, `formats`, aggiornamenti e `agent`; password solo stdin.
  - Test: 29 unit test, Clippy senza warning, smoke CLI/GUI su Windows, Ubuntu 22.04 e macOS 15 Intel; screenshot dark/light verificati.
  - Download pubblico installer 0.6.0 e SHA-256 verificati. Formati legacy e modifica archivio restano pianificati in `FEATURE_PARITY.md`.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

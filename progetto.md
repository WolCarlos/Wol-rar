# Wol-rar

- **Stato Attuale:**
  - Versione 0.6.0 in preparazione; release pubblica precedente 0.5.0. Applicazione Rust 1.95+, GUI `eframe`/`wgpu`, nessuna telemetria.
  - `.wolrar` v2 in `rust/src/wolformat_v2.rs`, specifica CC0 `WOLRAR_FORMAT_V2.md`; lettura v1 mantenuta.
  - Streaming Zstandard, BLAKE3, Argon2id e XChaCha20-Poly1305 per chunk; nessun limite applicativo v1 di 512 MiB.
  - Volumi con manifest/digest, parity Reed–Solomon e riparazione in `rust/src/wolrecovery.rs`.
  - Deduplicazione opzionale, SFX volume unico in `rust/src/sfx.rs`, benchmark offline in `rust/src/benchmark.rs`.
  - Motore 7z/ZIP/TAR/AES-256 e RAR sola lettura; estrazione transazionale e nessuna sovrascrittura.
  - GUI rinnovata: dark default, light, icone originali, riepilogo, ordinamento, ricerca, scorciatoie, drag-and-drop, opzioni v2 e riparazione.
  - CLI JSON `list`, `info`, `create`, `extract`, `test`, `repair`, `sfx`, `benchmark`, `formats`, aggiornamenti e `agent`; password solo stdin.
  - Test locali: 29 unit test, Clippy senza warning, smoke CLI end-to-end e screenshot dark/light; CI 0.6.0 ancora da eseguire.
  - Formati legacy e modifica archivio restano pianificati in `FEATURE_PARITY.md`.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

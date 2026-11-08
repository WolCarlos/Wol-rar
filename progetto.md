# Wol-rar

- **Stato Attuale:**
  - Versione 0.7.0 pubblicata: commit `c77a8b0`, CI `42506875894` verde su Windows, Ubuntu 22.04 e macOS 15 Intel; parità completa non dichiarata.
  - Applicazione Rust 1.95+, GUI `eframe`/`wgpu`, nessuna telemetria.
  - `.wolrar` v2 streaming, volumi, recovery, deduplicazione e SFX; lettura v1 invariata.
  - Motore 7z/ZIP/TAR/AES-256 e RAR sola lettura; lettura ISO/CAB/ARJ/LZH/UUE/Z in `rust/src/legacy.rs`.
  - Modifica e conversione transazionale in `rust/src/edit.rs`; cifratura/commento/deduplicazione preservati quando supportati, volumi/recovery rifiutati.
  - Updater completo in `rust/src/update.rs`: cache dedicata, HTTPS/SHA-256, Installa, exit status e retry.
  - GUI dark/light con wizard, metadati estesi, modifica entry e benchmark comparativo JSON in `rust/src/benchmark.rs`.
  - Tredici icone originali multi-dimensione e integrazione `.wolrar`; test statici in `scripts/test_windows_assets.py`.
  - Test: 46 unit test, Clippy, smoke CLI/GUI su tre sistemi, screenshot dark/light e installer pubblico verificato SHA-256 `59ad5f0fbb5d91d28a51184010633fc392bb36918329e5505be51f25a95c72ca`.
  - Limite `.Z`: nessun digest o lunghezza originale, alcuni troncamenti non rilevabili. Installazione/disinstallazione reale non eseguita per vincolo di consegna.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

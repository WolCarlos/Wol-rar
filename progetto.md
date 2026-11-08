# Wol-rar

- **Stato Attuale:**
  - Versione pubblicata: 0.6.0. Versione 0.7.0 in preparazione; parità completa non dichiarata.
  - Applicazione Rust 1.95+, GUI `eframe`/`wgpu`, nessuna telemetria.
  - `.wolrar` v2 streaming, volumi, recovery, deduplicazione e SFX; lettura v1 invariata.
  - Motore 7z/ZIP/TAR/AES-256 e RAR sola lettura; lettura ISO/CAB/ARJ/LZH/UUE/Z in `rust/src/legacy.rs`.
  - Modifica e conversione transazionale in `rust/src/edit.rs`; cifratura/commento/deduplicazione preservati quando supportati, volumi/recovery rifiutati.
  - Updater completo in `rust/src/update.rs`: cache dedicata, HTTPS/SHA-256, Installa, exit status e retry.
  - GUI dark/light con wizard, metadati estesi, modifica entry e benchmark comparativo JSON in `rust/src/benchmark.rs`.
  - Tredici icone originali multi-dimensione e integrazione `.wolrar`; test statici in `scripts/test_windows_assets.py`.
  - Test locali: 46 unit test; smoke CLI ampliato in `scripts/test_wolrar_cli.py`. CI multipiattaforma e screenshot nuova build ancora da completare.
  - Limite `.Z`: nessun digest o lunghezza originale, alcuni troncamenti non rilevabili. Installazione/disinstallazione reale non eseguita per vincolo di consegna.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `rust/src/update.rs`.
- **AI Tasks:** prompt statico in `agent/AGENTS.snippet.md`; nessun modello AI o chiamata modello nell'app.

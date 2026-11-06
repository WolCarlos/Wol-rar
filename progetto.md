# Wol-rar

- **Stato Attuale:**
  - Versione 0.2.0; repository Git su `main`, pubblicazione GitHub `WolCarlos/Wol-rar` in preparazione.
  - Piattaforme: Windows x64, Ubuntu x64 e macOS; build native PyInstaller 6.19.0 orchestrate da `.github/workflows/release.yml`.
  - Motori 7-Zip 26.05 selezionati da `app_context.py`; downloader ufficiale con hash in `scripts/fetch_7zip.py`.
  - Backend: elenco archivi, creazione 7z/ZIP senza sovrascrittura, estrazione sicura senza overwrite e test integrità.
  - UI: scheda Archivio e scheda Novità/Licenze; smoke test locale superato.
  - Aggiornamenti: `update.py` usa `https://api.github.com/repos/WolCarlos/Wol-rar/releases/latest`, solo su richiesta, con asset per OS e verifica SHA-256.
  - Windows: `dist/Wol-rar.exe` e `release/Wol-rar-Windows-x64.exe` compilati, smoke test superato e firma Authenticode autofirmata presente; attendibilità pubblica non disponibile.
  - Test: 10 test backend/update attivi in `tests/`; build CI Ubuntu/macOS da verificare su GitHub Actions.
  - Creazione RAR esclusa; RAR supportato soltanto in lettura tramite 7-Zip.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `src/wolrar/update.py`.
- **AI Tasks:** nessun modello o prompt nell'applicazione; analisi REA documentata in `analysis/report.md`.

# Wol-rar

- **Stato Attuale:**
  - Versione 0.2.1 pubblicata: https://github.com/WolCarlos/Wol-rar/releases/tag/v0.2.1; branch `main` allineato a `origin/main`.
  - Piattaforme: Windows x64, Ubuntu x64 e macOS universal2; build native PyInstaller 6.19.0 in `.github/workflows/release.yml`.
  - Motori 7-Zip 26.05 selezionati da `app_context.py`; downloader ufficiale con hash in `scripts/fetch_7zip.py`.
  - Backend: elenco archivi, creazione 7z/ZIP senza sovrascrittura, estrazione sicura senza overwrite e test integrità.
  - UI: scheda Archivio e scheda Novità/Licenze con changelog locale e note release.
  - Aggiornamenti: `update.py` usa GitHub Releases solo su richiesta; download HTTPS per OS con verifica SHA-256, formati checksum testo/binario supportati.
  - Windows: `dist/Wol-rar.exe` compilato, smoke test superato e firma Authenticode autofirmata presente; attendibilità pubblica non disponibile.
  - CI release 42301496490 completata: test e smoke test verdi su Windows, Ubuntu 22.04 e macOS 15 Intel.
  - Test pubblico: `UpdateClient` ha scaricato e verificato l'asset Windows v0.2.1 dalla release reale.
  - Test automatici: 12 test backend/piattaforma/update in `tests/`; checksum di tutti gli asset verificati.
  - Creazione RAR esclusa; RAR supportato soltanto in lettura tramite 7-Zip.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `src/wolrar/update.py`.
- **AI Tasks:** nessun modello o prompt nell'applicazione; analisi REA documentata in `analysis/report.md`.

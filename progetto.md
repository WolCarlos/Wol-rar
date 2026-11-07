# Wol-rar

- **Stato Attuale:**
  - Versione 0.3.0 pubblicata: https://github.com/WolCarlos/Wol-rar/releases/tag/v0.3.0; branch `main` allineato a `origin/main`.
  - Piattaforme: Windows x64, Ubuntu x64 e macOS universal2; build native PyInstaller 6.19.0 in `.github/workflows/release.yml`.
  - Windows: installer per utente `Wol-rar-Setup-x64.exe` con Inno Setup 7.1.1, collegamenti e disinstallazione; binario portabile `Wol-rar-Windows-x64.exe`.
  - Explorer: contenitore unico `Wol-rar`; azioni Apri/Estrai/Verifica per estensioni archivio puntate e Comprimi per file/cartelle; CLI in `__main__.py`; regole in `installer/wol-rar.iss`.
  - UX: icone originali coordinate in `assets/`, toolbar, filtro, riepilogo compressione, ordinamento colonne e scorciatoie.
  - Backend: creazione 7z/ZIP con livello, estrazione sicura senza overwrite e test integrità tramite 7-Zip 26.05.
  - Aggiornamenti: `update.py` usa GitHub Releases solo su richiesta; Windows scarica l'installer, altri OS i rispettivi pacchetti; SHA-256 obbligatorio.
  - Firme: Authenticode autofirmata su app e installer, non pubblicamente attendibile; CI supporta certificati reali; macOS ad-hoc.
  - CI release 42383976726 completata: test e smoke test verdi su Windows, Ubuntu 22.04 e macOS 15 Intel.
  - Test pubblico: download installer verificato, installazione/avvio/controllo chiave ZIP/disinstallazione e pulizia registro completati.
  - Test automatici: 15 test backend/piattaforma/installer/shell/update; checksum di tutti gli asset verificati.
  - Creazione RAR esclusa; RAR supportato soltanto in lettura.
- **Database Schema:** nessuno; nessuna cronologia, credenziale o certificato persistente nel repository.
- **API Endpoints:** nessun endpoint applicativo; client GitHub Releases in `src/wolrar/update.py`.
- **AI Tasks:** nessun modello o prompt nell'applicazione; analisi REA documentata in `analysis/report.md`.

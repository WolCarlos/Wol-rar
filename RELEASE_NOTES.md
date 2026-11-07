# Wol-rar 0.4.0

- Migrazione dell'applicazione a **Rust**, senza runtime Python, con GUI `eframe` e build native Cargo.
- Nuova architettura **agent-first**: tutte le operazioni hanno CLI equivalente e output JSON opzionale.
- Nuovo comando `wol-rar agent` e scheda Agenti AI con prompt compatto per `AGENTS.md`.
- Motore per 7z, ZIP, RAR in sola lettura, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ, con estrazione sicura e test di integrità.
- Opzioni AES-256 per ZIP e 7z, password solo su stdin o campo volatile.
- Aggiornamenti GitHub con HTTPS e verifica SHA-256; su Windows viene scaricato l'installer.
- Installer aggiornato ai comandi Rust `--gui`, con menu Explorer raggruppato e disinstallazione pulita.
- Icona e metadati di versione incorporati nell'eseguibile Rust Windows.
- Test unitari e smoke test CLI/GUI su Windows, Ubuntu e macOS.

Le funzionalità avanzate ancora pianificate sono elencate in `FEATURE_PARITY.md`, senza dichiarare parità non verificata. RAR resta disponibile soltanto in lettura.

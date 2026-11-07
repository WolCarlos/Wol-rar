# Wol-rar 0.5.0

- Nuovo formato aperto **`.wolrar` v1**, indipendente da RAR, con specifica pubblica CC0 e implementazione Rust.
- Compressione Zstandard, streaming per file non cifrati, digest BLAKE3 globali e per file, commenti Unicode, timestamp e permessi.
- Cifratura opzionale con **Argon2id + XChaCha20-Poly1305**, nonce per file e autenticazione AAD dei metadati.
- Estrazione transazionale: archivio verificato prima della scrittura e nessun file parziale dopo errori, password errate o dati corrotti.
- Nuovo comando `info`, commenti da CLI/GUI e creazione CLI da una o più origini; `.wolrar` è il formato predefinito per i nuovi archivi.
- Menu Explorer e installer aggiornati per `.wolrar`, con specifica inclusa.
- Diciassette test unitari, lint senza warning e smoke test CLI normale/cifrato su Windows.

Limite corrente: i file `.wolrar` cifrati hanno un massimo di 512 MiB per file. Volumi, recovery, deduplicazione e SFX restano pianificati; RAR resta disponibile soltanto in lettura. Le altre lacune sono elencate in `FEATURE_PARITY.md`.

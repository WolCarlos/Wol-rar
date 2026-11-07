# Parità funzionale Wol-rar / WinRAR

**Implementato** significa coperto da test eseguiti. La parità completa non viene ancora dichiarata.

## Implementato nella base Rust 0.6.0

- `.wolrar` v2 con Zstandard e BLAKE3; lettura compatibile v1.
- Cifratura streaming Argon2id + XChaCha20-Poly1305 per chunk da 1 MiB, senza limite applicativo di 512 MiB della v1.
- Volumi `.wolrar`, `.w02`, `.w03`… con manifest, identità, offset e digest indipendenti.
- Parity Reed–Solomon `.r01`… e riparazione in nuova cartella con test di volumi mancanti/corrotti e limite parity.
- Deduplicazione opzionale dei contenuti identici con riferimenti verificati.
- SFX `.wolrar` v2 a volume unico con apertura dell'archivio incorporato.
- Benchmark offline con velocità, rapporto e verifica digest.
- Creazione ZIP, 7z, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ; RAR in sola lettura tramite unRAR.
- Estrazione transazionale, protezione da traversal/ADS/link e nessuna sovrascrittura.
- GUI rinnovata con tema scuro predefinito e chiaro, icone originali, riepilogo, tabella ordinabile, ricerca, scorciatoie, drag-and-drop e opzioni v2.
- CLI `list`, `info`, `create`, `extract`, `test`, `repair`, `sfx`, `benchmark`, `formats`, `update-check`, `update-download`, `agent` con JSON.
- Installer Windows e menu Explorer raggruppato; aggiornamenti HTTPS con SHA-256.
- Ventinove test unitari e smoke test CLI end-to-end superati localmente su Windows; temi verificati con screenshot reali.

## Ancora pianificato

- Apertura ISO, CAB, ARJ, LZH, UUE e Z tramite backend compatibile documentato.
- Operazioni di modifica/cancellazione/ridenominazione entry e conversione tra formati.
- Metadati estesi, ulteriori scorciatoie e wizard.
- Benchmark comparativi pubblici e ottimizzazioni guidate dai profili.

## Non previsto dalle regole del progetto

- Creazione RAR o ricostruzione dell'algoritmo proprietario RAR.
- Copia di codice, icone, testi o risorse WinRAR.
- Parità dichiarata prima dei test indipendenti per ogni funzione.

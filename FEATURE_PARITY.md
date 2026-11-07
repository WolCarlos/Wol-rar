# Parità funzionale Wol-rar / WinRAR

**Implementato** significa coperto da test eseguiti; le altre voci non vanno considerate disponibili. La parità completa con WinRAR non è ancora raggiunta.

## Implementato nella base Rust 0.5.0

- Formato aperto `.wolrar` v1 con Zstandard, digest BLAKE3 globali e per file, commenti Unicode, timestamp e permessi.
- Cifratura `.wolrar` Argon2id + XChaCha20-Poly1305 con AAD sui metadati; limite di 512 MiB per file cifrato.
- Streaming per file `.wolrar` non cifrati, estrazione transazionale e cleanup verificato dopo password errata, digest alterato, payload corrotto o frame troncato.
- Creazione `.wolrar`, ZIP, 7z, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ senza sovrascrittura, livelli 0-9, una o più origini da CLI, file e cartelle Unicode.
- Apertura/elenco `.wolrar`, ZIP, 7z, RAR, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ.
- Estrazione con rifiuto di path traversal, percorsi assoluti, ADS, link nei componenti di destinazione e sovrascrittura.
- Test di integrità tramite decodifica in una directory temporanea.
- RAR in sola lettura tramite unRAR, inclusi multi-volume gestiti dalla libreria.
- CLI agent-first `list`, `info`, `create`, `extract`, `test`, `formats`, `update-check`, `update-download` e `agent`, con JSON opzionale e commenti.
- Alias CLI `--open`, `--extract`, `--test`, `--create`, `--output`, `--destination`, `--level` e `--comment`.
- GUI `eframe` con apertura, creazione `.wolrar` predefinita, commenti, estrazione, verifica, filtro, Novità e sezione Agenti AI.
- Aggiornamenti HTTPS con SHA-256 e download installer Windows.
- Installer con menu Explorer raggruppato anche per `.wolrar` e binario con icona/metadati Windows.
- AES-256 per ZIP e 7z con round-trip di cifratura/decifratura testati.
- Diciassette test unitari, smoke CLI/GUI e build superati su Windows, Ubuntu 22.04 e macOS 15 Intel; round-trip normale/cifrato e file da 16 MiB verificati.

## In sviluppo o pianificato

- Benchmark di velocità/memoria e ottimizzazioni multi-thread misurate.
- Streaming dei file cifrati oltre 512 MiB.
- Temi, wizard e scorciatoie UX aggiuntive.
- ISO, CAB, ARJ, LZH, UUE e Z tramite backend compatibile documentato.
- Volumi `.wolrar`, metadati estesi e deduplicazione.
- SFX, recovery record/volume e riparazione.

## Non previsto dalle regole del progetto

- Creazione di archivi RAR o ricostruzione dell'algoritmo proprietario RAR.
- Copia di codice, icone, testi o risorse WinRAR.
- Compatibilità garantita con ogni estensione o caso particolare prima dei test indipendenti.

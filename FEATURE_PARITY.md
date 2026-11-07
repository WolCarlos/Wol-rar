# Parità funzionale Wol-rar / WinRAR

Stato della migrazione Rust. **Implementato** significa coperto da test eseguiti; le altre voci non vanno considerate disponibili.

## Implementato nella base Rust 0.4.0

- Creazione ZIP, 7z, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ senza sovrascrittura, livelli 0-9, file e cartelle, Unicode.
- Apertura/elenco ZIP, 7z, RAR, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ.
- Estrazione con rifiuto di path traversal, percorsi assoluti, ADS e sovrascrittura.
- Test di integrità tramite decodifica in una directory temporanea.
- RAR in sola lettura tramite la libreria unRAR, inclusi multi-volume gestiti dalla libreria.
- CLI agent-first `list`, `create`, `extract`, `test`, `formats`, `update-check`, `update-download` e `agent`, con JSON opzionale.
- Alias CLI `--open`, `--extract`, `--test`, `--create`, `--output`, `--destination`, `--level`.
- GUI `eframe` con apertura, creazione, estrazione, verifica, filtro, Novità e sezione Agenti AI.
- Aggiornamenti HTTPS con SHA-256 e download installer Windows.
- Installer aggiornato ai comandi Rust `--gui` e binario con icona/metadati Windows.
- AES-256 per ZIP e 7z con round-trip di cifratura/decifratura testati.
- Nove test unitari superati per round-trip, AES, TAR, traversal, checksum e versioni.

## In migrazione

- Benchmark di velocità/memoria.
- Temi, wizard e scorciatoie UX aggiuntive.
- ISO, CAB, ARJ, LZH, UUE e Z tramite backend compatibile documentato.
- Creazione volumi, commenti e metadati estesi.
- SFX, recovery record/volume e riparazione.
- Ottimizzazioni multi-thread e benchmark pubblici.

## Non previsto dalle regole del progetto

- Creazione di archivi RAR o ricostruzione dell'algoritmo proprietario RAR.
- Copia di codice, icone, testi o risorse WinRAR.
- Compatibilità garantita con ogni estensione o caso particolare prima dei test indipendenti.

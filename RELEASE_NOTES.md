# Wol-rar 0.6.0

- Nuova `.wolrar` v2 con cifratura streaming per chunk, senza il limite v1 di 512 MiB per file cifrato.
- Volumi multipli con manifest e digest, recovery Reed–Solomon e riparazione senza sovrascrittura.
- Deduplicazione opzionale, SFX a volume unico e benchmark offline con verifica digest.
- Nuova specifica pubblica CC0 `WOLRAR_FORMAT_V2.md`; lettura v1 mantenuta.
- Interfaccia completamente rinnovata: tema scuro predefinito, tema chiaro, icone originali, riepilogo compressioni, tabella ordinabile e UX migliorata.
- Nuove opzioni GUI/CLI per volumi, recovery e deduplicazione; nuovi comandi `repair`, `sfx` e `benchmark`.
- Evidenza REA aggiuntiva sulla copia autorizzata di WinRAR, senza copiare codice o risorse proprietarie.
- Ventinove test unitari, lint senza warning e smoke test CLI end-to-end.

Restano pianificati i formati legacy ISO, CAB, ARJ, LZH, UUE e Z e le operazioni di modifica archivio. RAR resta soltanto in lettura; nessuna parità non verificata viene dichiarata.

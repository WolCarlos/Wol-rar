# Wol-rar 0.7.0

- Updater completo: download dell’asset corretto nella cache dedicata, HTTPS e SHA-256 obbligatori, pulsante **Installa**, attesa dell’exit status e retry dopo errore. Ubuntu e macOS mostrano istruzioni specifiche.
- Nuova famiglia originale di 13 icone multi-dimensione per applicazione, installer, disinstallazione, collegamenti, Explorer, file `.wolrar` e SFX; registrazione `DefaultIcon` per il solo formato `.wolrar`.
- Lettura ISO/Joliet, CAB, ARJ, LHA/LZH, UUE e Unix `.Z` con estrazione transazionale, fixture Unicode/traversal/troncamento e backend multipiattaforma documentati.
- Nuove operazioni `delete`, `rename`, `add`, `update` e `convert`, con riscrittura in un nuovo file, verifica completa prima della pubblicazione e rollback dopo errori.
- Commento, cifratura e deduplicazione vengono preservati quando supportati; volumi e recovery vengono rifiutati esplicitamente durante modifica/conversione invece di promettere una migrazione non verificata.
- Metadati estesi nel JSON e nella GUI, wizard per creazione, estrazione, riparazione e aggiornamento, nuove scorciatoie e controlli di modifica entry.
- Benchmark comparativo con sei profili: `.wolrar` normale, cifrato, volumi, recovery, ZIP e 7z; risultati JSON e verifica digest per ogni caso.
- Quarantasei test unitari locali, Clippy senza warning, smoke CLI ampliato, test statici degli asset e smoke GUI dark/light con screenshot reali.

Limiti espliciti: Unix `.Z` non include digest né lunghezza originale, quindi alcuni troncamenti non sono rilevabili. RAR resta soltanto in lettura tramite unRAR. L’installazione/disinstallazione di prova non è stata eseguita durante la consegna; asset e script installer sono verificati staticamente. La parità completa con WinRAR non viene dichiarata.

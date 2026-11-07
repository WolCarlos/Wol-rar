## Wol-rar: operazioni da agente

Usa `wol-rar` per archivi locali senza telemetria. Tutte le funzioni hanno CLI equivalente con `--json`.

- Formati/versione: `wol-rar formats --json`, `wol-rar --version`.
- Elenca: `wol-rar list ARCHIVIO --json`; metadati: `wol-rar info ARCHIVIO.wolrar --json`.
- Crea v2: `wol-rar create ORIGINE [ALTRE...] --output ARCHIVIO.wolrar --level 5 --comment "Testo" --json`.
- Opzioni v2: `--volume-size BYTE` (minimo 65536), `--recovery-percent 0-100` (richiede volumi), `--deduplicate`.
- Estrai/verifica: `wol-rar extract ARCHIVIO --destination CARTELLA --json`, `wol-rar test ARCHIVIO --json`.
- Ripara: `wol-rar repair VOLUMI.wolrar --destination NUOVA-CARTELLA --json`.
- SFX volume unico: `wol-rar sfx ARCHIVIO.wolrar --output AUTO.exe --json`.
- Benchmark offline: `wol-rar benchmark --size-mib 32 --json`.
- Aggiornamenti: `wol-rar update-check --json`; scarica solo dopo conferma con `update-download`.
- Prompt: `wol-rar agent`.

Regole:
- Mai password in argomenti/log: usa `--password-stdin`; non salvarla o committarla.
- Nessuna sovrascrittura: usa destinazioni nuove; non cancellare/rinominare file senza conferma.
- Rifiuta sempre percorsi insicuri; verifica comunque la destinazione.
- `.wolrar` v1 legge file cifrati fino a 512 MiB; v2 usa streaming. Nomi/commenti non sono segreti.
- RAR solo lettura: non crearlo né ricostruirne algoritmi proprietari.
- Dopo operazioni importanti esegui `test` e confronta i file; non dichiarare successi non provati.
- Rete solo per aggiornamenti espliciti, HTTPS e SHA-256 verificato.

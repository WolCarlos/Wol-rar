## Wol-rar: operazioni da agente

Usa `wol-rar` per archivi locali senza telemetria. Tutte le funzioni hanno CLI equivalente con `--json`.

- Formati/versione: `wol-rar formats --json`, `wol-rar --version`.
- Elenca: `wol-rar list ARCHIVIO --json`; metadati: `wol-rar info ARCHIVIO.wolrar --json`.
- Crea v2: `wol-rar create ORIGINE [ALTRE...] --output ARCHIVIO.wolrar --level 5 --comment "Testo" --json`.
- Opzioni v2: `--volume-size BYTE` (minimo 65536), `--recovery-percent 0-100` (richiede volumi), `--deduplicate`.
- Estrai/verifica: `wol-rar extract ARCHIVIO --destination CARTELLA --json`, `wol-rar test ARCHIVIO --json`.
- Ripara: `wol-rar repair VOLUMI.wolrar --destination NUOVA-CARTELLA --json`.
- Modifica: `delete ARCHIVIO VOCE`, `rename ARCHIVIO VOCE NUOVO-PERCORSO`, `add`/`update ARCHIVIO FILE VOCE`; output opzionale `--output`.
- Converti: `wol-rar convert ARCHIVIO --output NUOVO.wolrar --json`; volumi/recovery vengono rifiutati durante edit/conversione.
- Legacy in lettura: ISO, CAB, ARJ, LHA/LZH, UUE e `.Z`; il formato `.Z` non garantisce il rilevamento di ogni troncamento.
- SFX volume unico: `wol-rar sfx ARCHIVIO.wolrar --output AUTO.exe --json`.
- Benchmark: `wol-rar benchmark --size-mib 32 --json`; comparativo `benchmark --suite --output benchmark.json`.
- Aggiornamenti: `wol-rar update-check --json`, `update-download --json`, poi `update-install ASSET-IN-CACHE --json` solo dopo richiesta esplicita.
- Prompt: `wol-rar agent`.

Regole:
- Mai password in argomenti/log: usa `--password-stdin`; non salvarla o committarla.
- Nessuna sovrascrittura: usa destinazioni nuove; non cancellare/rinominare file senza conferma.
- Rifiuta sempre percorsi insicuri; verifica comunque la destinazione.
- `.wolrar` v1 legge file cifrati fino a 512 MiB; v2 usa streaming. Nomi/commenti non sono segreti.
- RAR solo lettura: non crearlo né ricostruirne algoritmi proprietari.
- Dopo operazioni importanti esegui `test` e confronta i file; non dichiarare successi non provati.
- Rete solo per aggiornamenti espliciti, HTTPS e SHA-256 verificato.

## Wol-rar: operazioni da agente

Usa `wol-rar` per archivi locali, senza telemetria e senza cambiare il programma predefinito. Tutte le funzioni dell'app hanno una CLI equivalente con `--json`.

- Formati/versione: `wol-rar formats --json`, `wol-rar --version`.
- Elenca: `wol-rar list ARCHIVIO --json`; metadati `.wolrar`: `wol-rar info ARCHIVIO.wolrar --json`.
- Crea: `wol-rar create ORIGINE [ALTRE...] --output ARCHIVIO.wolrar --level 5 --comment "Testo" --json`; usa `.7z`/`.zip` se richiesto.
- Estrai: `wol-rar extract ARCHIVIO --destination CARTELLA --json`.
- Verifica: `wol-rar test ARCHIVIO --json`.
- Aggiorna: `wol-rar update-check --json`; scarica solo dopo conferma con `wol-rar update-download CARTELLA --json`.
- Mostra questo prompt: `wol-rar agent`.

Regole:
- Mai password in argomenti o log: usa `--password-stdin`, senza salvarla o committarla.
- Nessuna sovrascrittura: usa destinazioni nuove; non cancellare/rinominare file senza conferma.
- Wol-rar rifiuta traversal, percorsi assoluti, ADS e file esistenti. Verifica comunque la destinazione.
- `.wolrar` v1 usa Zstandard/BLAKE3 e cifratura Argon2id/XChaCha20-Poly1305; massimo 512 MiB per file cifrato. Nomi e commenti non sono segreti.
- RAR solo in lettura: non crearlo e non ricostruirne algoritmi proprietari.
- Dopo operazioni importanti esegui `test` e confronta i file attesi. Non dichiarare successi non provati.
- Rete solo per aggiornamenti espliciti, HTTPS e SHA-256 verificato.

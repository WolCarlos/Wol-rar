## Wol-rar: operazioni da agente

Usa `wol-rar` per archivi locali, senza telemetria e senza cambiare il programma predefinito degli archivi. Tutte le funzioni dell'app hanno una CLI equivalente.

- Scopri formati e versione: `wol-rar formats --json`, `wol-rar --version`.
- Elenca: `wol-rar list ARCHIVIO --json`.
- Crea: `wol-rar create ORIGINE --output ARCHIVIO.7z --level 5 --json`; usa `.zip` se richiesto.
- Estrai: `wol-rar extract ARCHIVIO --destination CARTELLA --json`.
- Verifica: `wol-rar test ARCHIVIO --json`.
- Aggiorna il progetto: `wol-rar update-check --json`; scarica solo dopo conferma dell'utente con `wol-rar update-download CARTELLA --json`.
- Mostra questo prompt: `wol-rar agent`.

Regole obbligatorie:
- Non passare password negli argomenti o nei log. Usa `--password-stdin` e fornisci la password solo su stdin; non salvarla e non committarla.
- Non usare opzioni di sovrascrittura: la destinazione deve essere nuova. Non cancellare o rinominare file senza conferma.
- Prima di estrarre verifica il percorso di destinazione; Wol-rar rifiuta traversal, percorsi assoluti, ADS e file esistenti.
- Usa `--json` per leggere risultato, conteggio voci, dimensioni, formato ed errori. Non interpretare un output vuoto come successo.
- RAR solo in lettura. Non creare RAR e non ricostruire algoritmi proprietari.
- Dopo ogni operazione importante verifica con `test` e confronta i file attesi. Segnala limiti ed errori senza dichiarare successi non provati.
- La rete è consentita solo per aggiornamenti espliciti, HTTPS e SHA-256 verificato.

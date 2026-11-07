# Analisi iniziale WinRAR

- **Target osservato:** WinRAR 6.24.0, `WinRAR.exe`, copia locale del binario installato.
- **SHA-256:** `2e5cef05647e2b74a87893b9256e7f7d72214defb7f0f24b7f6fbb4ff0656b2b`.
- **Strumento:** REA 6.8.0, analisi statica senza esecuzione del target.
- **Formato osservato:** PE32+ nativo, AMD64, 2.565.272 byte.
- **Certificato osservato:** una voce Authenticode PKCS#7 con due certificati incorporati; validità crittografica e catena non verificate da questa analisi.
- **Copertura:** inventario dell'artefatto, header desktop, import/delay-import PE e struttura del certificato.
- **Limiti:** nessuna decompilazione, nessuna osservazione runtime e nessuna conclusione sulle funzionalità derivata dai soli import.
- **Decisione clean-room:** implementare un archiviatore originale offline con 7-Zip; creazione 7z/ZIP, RAR in sola lettura tramite il motore, nessun codice o asset WinRAR riutilizzato.

Le evidenze JSON complete e la copia del target sono locali e ignorate da Git.

## Integrazione REA 6.8.0, 7 novembre 2026

- **Nuova evidenza osservata:** `ev_780a90a0f64b2cd4333fd7b59520de1bb75e3cfeb743bcc966e532f99674aefa`, operazione `inspect_pe_resources` sullo stesso target e digest.
- **Risultato:** tabella risorse PE32+ AMD64 a offset 1632256; inventario con una famiglia `PNG` e 111 identificatori di primo livello. Nessuna risorsa viene estratta o riutilizzata.
- **Limite:** l'inventario statico delle risorse non dimostra comportamento runtime né associa autonomamente un'icona a una funzione.
- **Mappa funzionale di progetto:** UI scura/chiara e UX; `.wolrar` v2 con cifratura streaming e volumi; recovery/riparazione; formati e operazioni avanzate; benchmark e test nativi. Ogni voce resta pianificata finché non ha test propri.
- **Confine clean-room:** nessuna decompilazione del compressore RAR, nessuna copia di codice, testi o asset. Analisi REA limitata a identità, struttura e comportamento interoperabile consentito.

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

# Wol-rar 0.2.1

- Corretta la lettura di `SHA256SUMS.txt` nel formato standard `hash *file` prodotto dagli strumenti GNU.
- Aggiunto un test di regressione per impedire il rifiuto dei checksum validi.
- Verificato il download pubblico dalla release GitHub con controllo dell'hash prima del salvataggio.
- Build e smoke test confermati su Windows x64, Ubuntu x64 e macOS.

Include tutte le funzionalità della 0.2.0: build native, scheda Novità e aggiornamenti, motori 7-Zip 26.05 e supporto alla firma Windows/macOS. Nessuna telemetria, account o connessione automatica.

La creazione di archivi RAR resta esclusa; RAR è supportato soltanto in lettura tramite 7-Zip.

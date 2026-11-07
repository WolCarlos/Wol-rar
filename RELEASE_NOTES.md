# Wol-rar 0.3.0

- Nuovo **installer Windows per utente**, senza obbligo di amministratore, con collegamenti e disinstallazione pulita.
- Menu contestuale **Wol-rar** raggruppato in Explorer: Apri, Estrai e Verifica per gli archivi; Comprimi per file e cartelle.
- Icone originali coordinate per applicazione, installer e comandi, con pila di volumi compressi e fascetta.
- UX/UI rinnovata: toolbar iconata, filtro istantaneo, riepilogo della compressione, ordinamento colonne, scorciatoie e indicatore di attività.
- Nuovi comandi CLI `--open`, `--extract`, `--test` e `--create` usati dall'integrazione Explorer.
- Gli aggiornamenti Windows scaricano direttamente il nuovo installer, sempre con verifica SHA-256.
- Build e smoke test su Windows, Ubuntu e macOS.

Su Windows 11 il menu classico può comparire sotto **Mostra altre opzioni**, in base alle impostazioni di Explorer. Nessuna telemetria o connessione automatica. RAR resta disponibile soltanto in lettura.

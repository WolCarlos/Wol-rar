# Parità funzionale Wol-rar / WinRAR

**Implementato** significa coperto da test eseguiti. La parità completa non viene ancora dichiarata.

## Implementato nella base Rust e nel lavoro successivo alla 0.6.0

- `.wolrar` v2 con Zstandard e BLAKE3; lettura compatibile v1.
- Cifratura streaming Argon2id + XChaCha20-Poly1305 per chunk da 1 MiB, senza limite applicativo di 512 MiB della v1.
- Volumi `.wolrar`, `.w02`, `.w03`… con manifest, identità, offset e digest indipendenti.
- Parity Reed–Solomon `.r01`… e riparazione in nuova cartella con test di volumi mancanti/corrotti e limite parity.
- Deduplicazione opzionale dei contenuti identici con riferimenti verificati.
- SFX `.wolrar` v2 a volume unico con apertura dell'archivio incorporato.
- Benchmark offline e suite comparativa `.wolrar` normale/cifrato/volumi/recovery, ZIP e 7z, con report JSON.
- Creazione ZIP, 7z, TAR, TAR.GZ, TAR.BZ2 e TAR.XZ; RAR in sola lettura tramite unRAR.
- Lettura ISO/Joliet, CAB, ARJ, LHA/LZH, UUE e Unix `.Z` tramite backend documentati.
- Estrazione transazionale, protezione da traversal/ADS/link e nessuna sovrascrittura.
- Modifica `delete`, `rename`, `add`, `update` e `convert` con riscrittura, verifica e rollback; CLI JSON e GUI.
- Updater con cache dedicata, download HTTPS/SHA-256, pulsante Installa, exit status reale e retry documentato.
- Famiglia originale di 13 icone multi-dimensione per app, installer, collegamenti, Explorer, `.wolrar` e SFX.
- GUI con tema scuro predefinito e chiaro, metadati estesi, wizard, selezione/modifica entry, scorciatoie e drag-and-drop.
- Quarantasei test unitari locali, smoke CLI esteso e test statici degli asset Windows.

## Checklist delle lacune residue

Ogni voce può essere spuntata soltanto dopo implementazione, test automatici e verifica della CLI/GUI pertinenti.

### 1. Updater, installazione e cache

- [x] Download automatico dell’asset corretto dopo un clic esplicito, con HTTPS e SHA-256 obbligatorio.
- [x] Cache dedicata nel profilo utente, pulizia selettiva prima del download e dopo installazione riuscita.
- [x] Pulsante **Installa**, installer visibile, attesa dell’exit status e stati GUI completi senza operazioni concorrenti.
- [x] Flusso o istruzioni specifiche per Windows, Ubuntu e macOS; CLI JSON per download e installazione.
- [x] Test per asset, checksum, sostituzione/pulizia cache, errori e nessuna cancellazione esterna.

### 2. Icone e integrazione Windows

- [x] Famiglia originale per app, installer, disinstallazione, collegamenti, Explorer, `.wolrar` e SFX.
- [x] File `.ico` multi-dimensione 16, 24, 32, 48, 64, 128 e 256 px, PNG e generazione riproducibile.
- [x] `DefaultIcon` per `.wolrar`, risorse eseguibile/installer e pulizia delle sole chiavi create.
- [ ] Verifica Windows effettiva di registro, installazione/avvio/disinstallazione. Asset e script sono verificati staticamente; l’installazione non è stata eseguita perché la consegna richiede di non installare l’app.

### 3. Formati legacy

- [x] Backend multipiattaforma con provenienza, versioni, licenze e redistribuzione documentate.
- [x] Apertura, elenco, test ed estrazione sicura di ISO, CAB, ARJ, LZH, UUE e Z, con limite `.Z` documentato.
- [ ] Fixture controllate Unicode, traversal e troncamento: test locali superati; attesa conferma CI su Windows, Ubuntu e macOS.

### 4. Modifica e conversione archivi

- [x] Cancellazione e ridenominazione entry; aggiunta e aggiornamento file.
- [x] Conversione tra tutti i formati supportati in scrittura.
- [x] Riscrittura transazionale con originale preservato fino alla verifica finale.
- [x] Preservazione di commenti, metadati, cifratura e deduplicazione quando supportati; volumi/recovery rifiutati esplicitamente durante edit/conversione.
- [x] CLI JSON, controparte GUI, round-trip e cleanup dopo errori.

### 5. UX, metadati e accessibilità

- [x] Metadati estesi nei formati compatibili.
- [x] Wizard per creazione, estrazione, riparazione e aggiornamento.
- [x] Scorciatoie aggiuntive e rifinitura implementati; screenshot dark/light della nuova build verificati in `analysis/results/ui-dark-next.png` e `ui-light-next.png`.

### 6. Prestazioni

- [x] Benchmark comparativi documentati prima delle ottimizzazioni.
- [x] Profili di creazione, cifratura, estrazione, volumi e recovery.
- [x] Streaming e memoria controllata per file grandi; benchmark riproducibili con risultati JSON.

## Limiti espliciti

- Unix `.Z` non contiene digest né lunghezza originale: alcuni troncamenti a metà stream possono non essere rilevabili. Header troppo corti e flussi non decodificabili vengono rifiutati.
- La modifica/conversione rifiuta volumi e recovery invece di prometterne una preservazione non dimostrata. La deduplicazione viene ricreata quando presente.
- Non viene dichiarata parità completa finché restano verifiche aperte.

## Non previsto dalle regole del progetto

- Creazione RAR o ricostruzione dell'algoritmo proprietario RAR.
- Copia di codice, icone, testi o risorse WinRAR.
- Parità dichiarata prima dei test indipendenti per ogni funzione.

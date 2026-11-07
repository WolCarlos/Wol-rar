# Formato aperto `.wolrar`, versione 1

La specifica può essere implementata indipendentemente con qualsiasi linguaggio e licenza. Il testo di questo documento è dedicato al pubblico dominio tramite [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/). Il formato non deriva da RAR e non usa algoritmi proprietari.

## Obiettivi

- Lettura e scrittura streaming per file non cifrati, senza caricare l'intero archivio in memoria.
- Compressione Zstandard, con un frame indipendente per ogni file.
- Integrità per file e per l'intero archivio tramite BLAKE3.
- Cifratura autenticata opzionale XChaCha20-Poly1305, derivazione Argon2id e nonce univoco per file.
- Commenti UTF-8, timestamp Unix, permessi, directory e percorsi Unicode.
- Rifiuto di traversal, percorsi assoluti, ADS, file esistenti e dati troncati o alterati.
- Estensione `.wolrar`; nessuna relazione di compatibilità con `.rar`.

## Struttura

Tutti i numeri interi sono little-endian. Il file è composto da header di 108 byte, commento e sequenza di entry.

### Header

| Offset | Tipo | Campo |
| --- | --- | --- |
| 0 | 8 byte | Magic `WOLRAR01` |
| 8 | `u16` | Versione formato, attualmente 1 |
| 10 | `u16` | Flags: bit 0 cifratura, altri riservati |
| 12 | `i32` | Livello Zstandard |
| 16 | `u32` | Lunghezza commento in byte |
| 20 | 16 byte | Salt casuale per Argon2id |
| 36 | 24 byte | Nonce base casuale |
| 60 | `u64` | Numero di entry; nella v1 al massimo `u32::MAX` |
| 68 | `u64` | `body_len`: commento + entry, escluso header |
| 76 | 32 byte | Digest globale BLAKE3 |

La lunghezza totale deve essere esattamente `108 + body_len`. Il digest globale è `BLAKE3(header[0..76] || commento || entry)`; il campo digest stesso, agli offset 76-107, non partecipa al calcolo. In creazione il campo è inizialmente zero e viene scritto per ultimo.

### Entry

| Tipo | Campo |
| --- | --- |
| `u16` | Lunghezza percorso UTF-8 |
| `u16` | Flags: bit 0 directory, bit 1 cifratura |
| `u64` | Dimensione contenuto originale |
| `u64` | Dimensione payload compresso/cifrato |
| `i64` | Modifica Unix, secondi |
| `u32` | Permessi Unix, oppure 0644/0444 su Windows |
| byte[] | Percorso UTF-8 |
| 32 byte | BLAKE3 del contenuto originale |
| byte[] | Payload di `packed_size` byte |

I metadati fissi prima del percorso occupano 32 byte; il digest inizia quindi a `metadata_offset + 32 + path_len` e il payload a `metadata_offset + 64 + path_len`. Le entry sono consecutive e i percorsi devono essere univoci. Le directory non hanno payload e hanno dimensione e digest a zero. Per file non cifrati il payload è un frame Zstandard. Per file cifrati il payload è `ciphertext || tag Poly1305`, con 16 byte aggiuntivi per il tag.

## Crittografia

- La password è la sequenza di byte UTF-8 fornita dall'utente; nessuna password viene salvata nell'archivio.
- Chiave a 32 byte derivata con **Argon2id v19**, salt di 16 byte, memoria 32.768 KiB, 3 iterazioni, 1 lane e output 32 byte.
- Cifrario **XChaCha20-Poly1305**. Il nonce a 24 byte è il nonce base con i primi 4 byte XOR del contatore entry `u32` little-endian, partendo da zero.
- Ogni file cifrato autentica come dati aggiuntivi (AAD), in ordine: `path_len u16`, percorso UTF-8, `size u64`, `modified i64`, `mode u32`, digest file a 32 byte, `index u32`, flag directory `u8`, flag cifratura `u8`.
- Salt e nonce sono casuali e distinti per archivio; il nonce derivato è distinto per entry. La chiave viene derivata una volta per operazione.
- Nomi, commenti e metadati non sono segreti: la cifratura copre il contenuto compresso, mentre i metadati dei file sono autenticati tramite AAD.
- Password errate, tag non validi, metadati autenticati modificati e contenuto alterato devono causare errore, mai output accettato.
- Questa implementazione limita i file cifrati a 512 MiB originale per file e decodifica con un limite di output. I file non cifrati usano streaming e non hanno tale limite applicativo.

BLAKE3 è un digest non segreto e non sostituisce l'autenticazione AEAD. La resistenza agli attacchi dipende anche dalla robustezza della password.

## Validazione obbligatoria

1. Magic, versione e flags sconosciuti devono essere rifiutati.
2. Lunghezze e offset devono restare nel file; nessuna lettura oltre `body_len` e nessun dato dopo l'ultima entry dichiarata.
3. Percorsi devono essere relativi, senza `..`, drive, ADS o NUL; duplicati e conflitti cifratura/directory non sono validi.
4. Ogni file creato in estrazione usa creazione atomica senza sovrascrittura. I componenti preesistenti che sono link simbolici devono essere rifiutati.
5. Il digest globale deve essere verificato prima dell'estrazione; ogni contenuto deve rispettare dimensione e digest per file.
6. Le implementazioni devono ignorare link simbolici durante la creazione, non seguirli.
7. La decodifica deve rispettare la dimensione originale dichiarata, evitando output oltre tale limite.

L'implementazione Rust estrae prima in una directory temporanea sulla stessa destinazione, verifica ogni file e pubblica soltanto contenuti completi. In caso di errore rimuove i file pubblicati durante il tentativo e non lascia file parziali.

## Limiti della versione 1 ed evoluzione

La v1 non include volumi multipli, recovery record, riparazione, deduplicazione o SFX. La versione 2 potrà aggiungere volumi con manifest, deduplicazione e recovery. Nuove versioni o flags sconosciute non devono essere accettate silenziosamente da lettori precedenti.

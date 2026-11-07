# Formato aperto `.wolrar`, versione 2

Specifica dedicata al pubblico dominio con [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). Non deriva da RAR e non usa algoritmi proprietari. La v1 resta descritta in `WOLRAR_FORMAT.md` e leggibile dall'implementazione Rust.

## Caratteristiche

- Stream logico Zstandard con chunk da 1 MiB e memoria applicativa costante per file.
- Cifratura opzionale Argon2id + XChaCha20-Poly1305 per chunk, senza limite applicativo di 512 MiB.
- Volumi `.wolrar`, `.w02`, `.w03`… con identità, offset e digest indipendenti.
- Deduplicazione opzionale per digest BLAKE3.
- Parity Reed–Solomon in `.r01`, `.r02`… e riparazione senza sovrascrittura.
- SFX a volume unico per il sistema del launcher.
- Digest globali/per file/per volume, commenti Unicode, timestamp e permessi.

## Stream logico

Ogni volume contiene una porzione dello stesso stream. Il primo header logico è di 144 byte.

| Offset | Tipo | Campo |
| --- | --- | --- |
| 0 | 8 byte | Magic `WOLRAR01` |
| 8 | `u16` | Versione 2 |
| 10 | `u16` | Flags: bit 0 cifratura |
| 12 | `i32` | Livello Zstandard |
| 16 | `u32` | Dimensione chunk, 1.048.576 |
| 20 | `u32` | Lunghezza commento UTF-8 |
| 24 | `u64` | Dimensione volume completa, 0 se unico/non dimensionato |
| 32 | 16 byte | Salt Argon2id |
| 48 | 24 byte | Nonce base |
| 72 | 16 byte | Identità casuale archivio |
| 88 | `u64` | Numero entry |
| 96 | `u64` | `body_len`: commento + entry |
| 104 | 32 byte | BLAKE3 globale |
| 136 | 8 byte | Riservati, zero |

Il digest globale è `BLAKE3(header[0..104] || commento || entry)`, escluso quindi il campo digest stesso. La lunghezza logica totale è `144 + body_len`.

## Entry

| Tipo | Campo |
| --- | --- |
| `u16` | Lunghezza percorso UTF-8 |
| `u16` | Flags: bit 0 directory, bit 1 cifratura, bit 2 duplicato |
| `u64` | Dimensione originale |
| `u64` | `packed_size`, inclusi tag AEAD |
| `u64` | `compressed_size`, senza tag |
| `i64` | Modifica Unix |
| `u32` | Permessi |
| `u32` | Numero chunk |
| `u32` | Indice entry sorgente duplicata, oppure `0xffffffff` |
| byte[] | Percorso |
| 32 byte | BLAKE3 originale |
| byte[] | Payload |

I metadati fissi prima del percorso occupano 48 byte. Il payload inizia a `metadata_offset + 80 + path_len`. Ogni file non duplicato contiene un frame Zstandard. I chunk compressi sono da 1 MiB, tranne l'ultimo. Per file cifrati ogni chunk aggiunge un tag di 16 byte: `packed_size = compressed_size + 16 * chunk_count`.

Un duplicato ha bit 2 attivo, nessun payload e `duplicate_of` che indica una entry precedente con stessa dimensione, digest e cifratura. La copia estratta viene verificata nuovamente.

## Crittografia streaming

- Argon2id v19: memoria 32.768 KiB, 3 iterazioni, 1 lane, output 32 byte, salt di 16 byte.
- Per ogni entry e chunk: nonce base con XOR dell'indice entry `u32` LE sui byte 0-3 e dell'indice chunk `u32` LE sui byte 4-7.
- AAD: `path_len`, percorso, `size`, `modified`, `mode`, digest file, indice entry, indice chunk, flag directory e cifratura.
- La derivazione avviene una volta per operazione; chiavi e buffer temporanei sensibili vengono azzerati dove possibile.
- Nomi, commenti e metadati non sono segreti. Ogni tag autentica chunk e metadati statici della entry.

## Volume

Ogni volume ha un header di 84 byte:

| Offset | Tipo | Campo |
| --- | --- | --- |
| 0 | 8 byte | Magic `WOLVOL02` |
| 8 | `u16` | Versione 2 |
| 10 | `u16` | Flags riservate, zero |
| 12 | `u32` | Indice volume, da 1 |
| 16 | `u32` | Numero volumi |
| 20 | 16 byte | Identità archivio |
| 36 | `u64` | Offset logico del payload |
| 44 | `u64` | Lunghezza payload |
| 52 | 32 byte | BLAKE3 di header[0..52] + payload |

Il primo volume usa il nome scelto `.wolrar`; i successivi cambiano estensione in `.w02`, `.w03`…. La dimensione minima configurabile è 65.536 byte, header incluso. Il digest globale viene scritto dopo una prima chiusura dei volumi; i digest dei volumi vengono poi aggiornati nuovamente.

## Recovery

Ogni file `.rNN` contiene uno shard parity Reed–Solomon GF(2^8) sulla rappresentazione completa dei volumi, padding dell'ultimo incluso. Sono supportati al massimo 256 shard totali. La percentuale richiesta determina `ceil(data_volumes * percent / 100)`, minimo uno. La recovery richiede volumi dimensionati.

Header di 80 byte: magic `WOLREC02`, versione e flags a 16 bit, indice/dati/parity a 32 bit, identità a 16 byte, dimensione shard a 64 bit e digest a 32 byte. Il digest è BLAKE3 di header[0..48] + shard. Il payload inizia a offset 80.

La riparazione scarta shard con digest errato, ricostruisce solo se le assenze non superano la parity, rimuove il padding finale e verifica il manifest prima di pubblicare in una nuova cartella.

## SFX

Consentito solo per v2 a volume unico. Il file contiene launcher, intero volume e trailer di 16 byte: magic `WOLSFX02` a 8 byte + lunghezza archivio `u64` LE. All'avvio il payload viene copiato in un file temporaneo e verificato prima dell'apertura. La firma del launcher non si estende automaticamente al nuovo SFX.

## Validazione

Sono obbligatori: rifiuto di versioni/flags sconosciute, controlli overflow, percorsi sicuri e univoci, digest globali/per volume/per file, AEAD per chunk, riferimenti duplicati coerenti, nessuna sovrascrittura e nessun file parziale dopo errori. Volumi mancanti o alterati non devono essere accettati senza una riparazione valida.

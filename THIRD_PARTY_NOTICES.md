# Third-party notices

Wol-rar usa crate Rust mantenuti dai rispettivi autori. Ogni pacchetto conserva la propria licenza e i propri copyright. Le fonti principali sono:

- `eframe`, `egui`, `image`, `rfd`: MIT o Apache-2.0, secondo i rispettivi pacchetti.
- `sevenz-rust2`: MIT; vedere https://github.com/hasenbanck/sevenz-rust2.
- `zip`: MIT; vedere https://github.com/zip-rs/zip2.
- `unarc-rs` 0.8.1: MIT o Apache-2.0; decoder legacy e CAB/ARJ/LZH/Z, https://github.com/mkrueger/unarc-rs.
- `delharc` 0.8.0: MIT o Apache-2.0; parsing header LHA/LZH, https://github.com/royaltm/rust-delharc.
- `hadris-iso` 2.5.0: MIT; lettura/scrittura ISO 9660 e Joliet, https://github.com/hxyulin/hadris.
- `uuencoding` 0.3.0: MIT o Apache-2.0; https://github.com/MarkAtwood/crate-mime.
- `unrar`: MIT o Apache-2.0 per il wrapper; vedere https://github.com/muja/unrar.rs.
- `argon2`: MIT o Apache-2.0; vedere https://docs.rs/argon2/.
- `blake3`: CC0 o Apache-2.0; `chacha20poly1305`, `filetime`: MIT o Apache-2.0, secondo i rispettivi pacchetti.
- `zstd`: MIT per il crate; la libreria nativa conserva i propri avvisi.
- `reed-solomon-erasure`: MIT; vedere https://github.com/mersinvald/reed-solomon-erasure.
- `tar`, `flate2`, `bzip2`, `xz2`, `walkdir`, `clap`, `serde`, `serde_json`, `semver`, `ureq`, `sha2`, `thiserror`, `anyhow`, `zeroize`, `getrandom`, `winres`: licenze indicate nei rispettivi pacchetti crates.io e repository ufficiali.
- Le librerie native incluse transitivamente conservano i rispettivi avvisi; l'elenco completo delle versioni è in `rust/Cargo.lock`.

## unRAR restriction

La decompressione RAR usa la libreria unRAR. I copyright originali appartengono ad Alexander Roshal. Il codice sorgente unRAR non può essere usato per ricreare l'algoritmo proprietario di compressione RAR né per sviluppare un archiviatore RAR-compatibile in scrittura. Wol-rar supporta RAR soltanto in lettura e non crea archivi RAR.

## Privacy

Nessuna dipendenza aggiunge account, tracking o telemetria. La rete viene usata soltanto per controllo e download espliciti degli aggiornamenti da GitHub Releases.

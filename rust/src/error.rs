use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("Formato non supportato: {0}")]
    UnsupportedFormat(String),
    #[error("Percorso non sicuro nell'archivio: {0}")]
    UnsafePath(String),
    #[error("La destinazione esiste già: {0}")]
    DestinationExists(PathBuf),
    #[error("Origine non trovata: {0}")]
    MissingSource(PathBuf),
    #[error("Livello di compressione non valido: {0}; usare 0-9")]
    InvalidCompressionLevel(u32),
    #[error("L'archivio non può includere sé stesso")]
    ArchiveInsideSource,
    #[error("Errore di archivio: {0}")]
    Message(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Seven(#[from] sevenz_rust2::Error),
    #[error(transparent)]
    Unrar(#[from] unrar::error::UnrarError),
    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
}

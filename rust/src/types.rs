use std::collections::BTreeMap;

use zeroize::Zeroizing;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub packed_size: u64,
    pub modified: String,
    pub directory: bool,
    pub encrypted: bool,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct CreateOptions {
    pub level: u32,
    pub password: Option<Zeroizing<String>>,
    pub solid: bool,
    pub comment: Option<String>,
    /// Dimensione massima di ogni volume `.wolrar` v2, inclusi gli header.
    pub volume_size: Option<u64>,
    /// Percentuale di volumi parity Reed–Solomon; 0 disattiva la recovery.
    pub recovery_percent: u8,
    /// Comprime una sola volta i contenuti identici nella `.wolrar` v2.
    pub deduplicate: bool,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            level: 5,
            password: None,
            solid: true,
            comment: None,
            volume_size: None,
            recovery_percent: 0,
            deduplicate: false,
        }
    }
}

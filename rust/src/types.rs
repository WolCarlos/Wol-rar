use zeroize::Zeroizing;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ArchiveEntry {
    pub path: String,
    pub size: u64,
    pub packed_size: u64,
    pub modified: String,
    pub directory: bool,
    pub encrypted: bool,
}

#[derive(Clone)]
pub struct CreateOptions {
    pub level: u32,
    pub password: Option<Zeroizing<String>>,
    pub solid: bool,
    pub comment: Option<String>,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            level: 5,
            password: None,
            solid: true,
            comment: None,
        }
    }
}

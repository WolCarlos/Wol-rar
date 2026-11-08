use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ArchiveError;

const LATEST_RELEASE: &str = "https://api.github.com/repos/WolCarlos/Wol-rar/releases/latest";
const WINDOWS_ASSET: &str = "Wol-rar-Setup-x64.exe";
const LINUX_ASSET: &str = "Wol-rar-Linux-x64.tar.gz";
const MACOS_ASSET: &str = "Wol-rar-macOS.tar.gz";
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub published_at: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDownload {
    pub version: String,
    pub asset: String,
    pub destination: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInstall {
    pub asset: PathBuf,
    pub outcome: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub cache_cleared: bool,
    pub instructions: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePhase {
    Idle,
    Checking,
    Downloading,
    Verifying,
    Ready,
    Installing,
    Completed,
    Error,
}

pub fn update_cache_dir() -> Result<PathBuf, ArchiveError> {
    if cfg!(target_os = "windows") {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or_else(|| {
                ArchiveError::Message("Profilo utente non disponibile per la cache".into())
            })?;
        Ok(PathBuf::from(base).join("Wol-rar").join("updates"))
    } else if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME").ok_or_else(|| {
            ArchiveError::Message("HOME non disponibile per la cache aggiornamenti".into())
        })?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Caches")
            .join("Wol-rar")
            .join("updates"))
    } else {
        let base = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .ok_or_else(|| {
                ArchiveError::Message("Directory cache utente non disponibile".into())
            })?;
        Ok(base.join("wol-rar").join("updates"))
    }
}

pub fn check_latest() -> Result<ReleaseInfo, ArchiveError> {
    let release: ReleaseInfo = serde_json::from_slice(&fetch(LATEST_RELEASE, 2 * 1024 * 1024)?)
        .map_err(|error| ArchiveError::Message(format!("Risposta GitHub non valida: {error}")))?;
    Version::parse(release.tag_name.trim_start_matches('v'))
        .map_err(|error| ArchiveError::Message(format!("Versione release non valida: {error}")))?;
    release.asset(update_asset_name())?;
    release.asset("SHA256SUMS.txt")?;
    Ok(release)
}

pub fn download_latest() -> Result<UpdateDownload, ArchiveError> {
    download_latest_with_progress(|_| {})
}

pub fn download_latest_with_progress(
    progress: impl Fn(UpdatePhase),
) -> Result<UpdateDownload, ArchiveError> {
    let cache = update_cache_dir()?;
    download_latest_to_with_progress(&cache, progress)
}

fn download_latest_to_with_progress(
    cache: &Path,
    progress: impl Fn(UpdatePhase),
) -> Result<UpdateDownload, ArchiveError> {
    let _guard = UPDATE_LOCK.try_lock().map_err(|_| {
        ArchiveError::Message("Un download o un'installazione è già in corso".into())
    })?;
    progress(UpdatePhase::Checking);
    let release = check_latest()?;
    let asset = release.asset(update_asset_name())?;
    let checksums = String::from_utf8(fetch(
        &release.asset("SHA256SUMS.txt")?.browser_download_url,
        128 * 1024,
    )?)
    .map_err(|error| ArchiveError::Message(error.to_string()))?;
    let expected = checksum_for(&checksums, &asset.name)?;
    prepare_cache(cache)?;
    progress(UpdatePhase::Downloading);
    let data = fetch(&asset.browser_download_url, 512 * 1024 * 1024)?;
    progress(UpdatePhase::Verifying);
    let actual = format!("{:x}", Sha256::digest(&data));
    if actual != expected {
        return Err(ArchiveError::Message(
            "Checksum aggiornamento non valido; nessun file è stato conservato".into(),
        ));
    }
    let output = cache.join(&asset.name);
    let partial = cache.join(format!("{}.part", asset.name));
    fs::write(&partial, &data)?;
    verify_file_checksum(&partial, &expected)?;
    fs::rename(&partial, &output)?;
    progress(UpdatePhase::Ready);
    Ok(UpdateDownload {
        version: release.tag_name,
        asset: asset.name,
        destination: output,
        sha256: actual,
    })
}

pub fn install_asset(asset: &Path) -> Result<UpdateInstall, ArchiveError> {
    let cache = update_cache_dir()?;
    install_asset_with(asset, &cache, run_visible_installer)
}

fn install_asset_with(
    asset: &Path,
    cache: &Path,
    runner: impl FnOnce(&Path) -> Result<Option<i32>, ArchiveError>,
) -> Result<UpdateInstall, ArchiveError> {
    let _guard = UPDATE_LOCK.try_lock().map_err(|_| {
        ArchiveError::Message("Un download o un'installazione è già in corso".into())
    })?;
    let asset = validated_cached_asset(asset, cache)?;
    if !cfg!(target_os = "windows") {
        let instructions = if cfg!(target_os = "macos") {
            "Apri il pacchetto macOS scaricato, estrai Wol-rar.app e spostala in Applicazioni dopo aver chiuso Wol-rar."
        } else {
            "Estrai il pacchetto Linux scaricato in una nuova cartella e sostituisci il binario/portabile Wol-rar dopo aver chiuso l'applicazione."
        };
        return Ok(UpdateInstall {
            asset,
            outcome: "manual".into(),
            success: false,
            exit_code: None,
            cache_cleared: false,
            instructions: Some(instructions.into()),
        });
    }
    let exit_code = runner(&asset)?;
    if exit_code != Some(0) {
        return Err(ArchiveError::Message(format!(
            "Installer terminato con esito negativo (codice {}); artefatto conservato nella cache per un nuovo tentativo",
            exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "non disponibile".into())
        )));
    }
    clear_managed_artifacts(cache)?;
    Ok(UpdateInstall {
        asset,
        outcome: "installed".into(),
        success: true,
        exit_code,
        cache_cleared: true,
        instructions: None,
    })
}

fn run_visible_installer(asset: &Path) -> Result<Option<i32>, ArchiveError> {
    let status = Command::new(asset).status()?;
    Ok(status.code())
}

fn validated_cached_asset(asset: &Path, cache: &Path) -> Result<PathBuf, ArchiveError> {
    let cache = fs::canonicalize(cache)?;
    let asset = fs::canonicalize(asset)?;
    let valid_name = asset.file_name().and_then(|name| name.to_str()) == Some(update_asset_name());
    if asset.parent() != Some(cache.as_path()) || !valid_name || !asset.is_file() {
        return Err(ArchiveError::Message(
            "L'aggiornamento deve essere l'asset atteso nella cache gestita da Wol-rar".into(),
        ));
    }
    Ok(asset)
}

fn prepare_cache(cache: &Path) -> Result<(), ArchiveError> {
    fs::create_dir_all(cache)?;
    clear_managed_artifacts(cache)
}

fn clear_managed_artifacts(cache: &Path) -> Result<(), ArchiveError> {
    let cache = fs::canonicalize(cache)?;
    for item in fs::read_dir(&cache)? {
        let item = item?;
        let name = item.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if is_managed_artifact(name) && item.file_type()?.is_file() {
            let path = item.path();
            if path.parent() == Some(cache.as_path()) {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn is_managed_artifact(name: &str) -> bool {
    [WINDOWS_ASSET, LINUX_ASSET, MACOS_ASSET]
        .iter()
        .any(|asset| name == *asset || name == format!("{asset}.part"))
}

fn verify_file_checksum(path: &Path, expected: &str) -> Result<(), ArchiveError> {
    let actual = format!("{:x}", Sha256::digest(fs::read(path)?));
    if actual != expected {
        let _ = fs::remove_file(path);
        return Err(ArchiveError::Message(
            "Checksum del file scaricato non valido".into(),
        ));
    }
    Ok(())
}

impl ReleaseInfo {
    fn asset(&self, name: &str) -> Result<ReleaseAsset, ArchiveError> {
        self.assets
            .iter()
            .find(|asset| asset.name == name)
            .cloned()
            .ok_or_else(|| ArchiveError::Message(format!("Asset mancante nella release: {name}")))
    }
}

fn update_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        WINDOWS_ASSET
    } else if cfg!(target_os = "linux") {
        LINUX_ASSET
    } else if cfg!(target_os = "macos") {
        MACOS_ASSET
    } else {
        ""
    }
}

fn fetch(url: &str, limit: u64) -> Result<Vec<u8>, ArchiveError> {
    if !url.starts_with("https://") {
        return Err(ArchiveError::Message(
            "Sono consentiti soltanto URL HTTPS".into(),
        ));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .user_agent(format!("Wol-rar/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    response
        .body_mut()
        .with_config()
        .limit(limit)
        .read_to_vec()
        .map_err(|error| ArchiveError::Message(error.to_string()))
}

fn checksum_for(contents: &str, asset_name: &str) -> Result<String, ArchiveError> {
    for line in contents.lines() {
        let mut fields = line.split_whitespace();
        let Some(digest) = fields.next() else {
            continue;
        };
        let Some(name) = fields.next() else { continue };
        if name.trim_start_matches('*') == asset_name
            && digest.len() == 64
            && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Ok(digest.to_ascii_lowercase());
        }
    }
    Err(ArchiveError::Message(format!(
        "Checksum mancante per {asset_name}"
    )))
}

pub fn is_newer(candidate: &str) -> Result<bool, ArchiveError> {
    let candidate = Version::parse(candidate.trim_start_matches('v'))
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| ArchiveError::Message(error.to_string()))?;
    Ok(candidate > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gnu_checksums_and_rejects_invalid_digest() {
        let digest = "a".repeat(64);
        assert_eq!(
            checksum_for(&format!("{digest} *app.exe"), "app.exe").unwrap(),
            digest
        );
        assert_eq!(
            checksum_for(&format!("{digest}  app.exe"), "app.exe").unwrap(),
            digest
        );
        assert!(checksum_for(&format!("{} app.exe", "z".repeat(64)), "app.exe").is_err());
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("v99.0.0").unwrap());
        assert!(!is_newer(env!("CARGO_PKG_VERSION")).unwrap());
    }

    #[test]
    fn selects_the_asset_for_each_supported_platform_name() {
        assert!(is_managed_artifact(WINDOWS_ASSET));
        assert!(is_managed_artifact(LINUX_ASSET));
        assert!(is_managed_artifact(MACOS_ASSET));
        assert!(is_managed_artifact(&format!("{WINDOWS_ASSET}.part")));
        assert!(!is_managed_artifact("documento.txt"));
        assert!(!is_managed_artifact("../Wol-rar-Setup-x64.exe"));
    }

    #[test]
    fn cache_replacement_never_deletes_unknown_or_external_files() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let outside = temp.path().join("esterno.txt");
        let unknown = cache.join("conservami.txt");
        fs::write(&outside, "esterno").unwrap();
        fs::write(&unknown, "sconosciuto").unwrap();
        fs::write(cache.join(update_asset_name()), "vecchio").unwrap();
        fs::write(
            cache.join(format!("{}.part", update_asset_name())),
            "parziale",
        )
        .unwrap();
        prepare_cache(&cache).unwrap();
        assert!(!cache.join(update_asset_name()).exists());
        assert_eq!(fs::read_to_string(&outside).unwrap(), "esterno");
        assert_eq!(fs::read_to_string(&unknown).unwrap(), "sconosciuto");
    }

    #[test]
    fn checksum_verification_removes_a_corrupt_partial_file() {
        let temp = tempfile::tempdir().unwrap();
        let partial = temp.path().join("aggiornamento.part");
        fs::write(&partial, "contenuto").unwrap();
        assert!(verify_file_checksum(&partial, &"0".repeat(64)).is_err());
        assert!(!partial.exists());
    }

    #[test]
    fn successful_install_clears_only_managed_cache_files() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let asset = cache.join(update_asset_name());
        let unknown = cache.join("conservami.txt");
        let outside = temp.path().join("esterno.txt");
        fs::write(&asset, "installer").unwrap();
        fs::write(&unknown, "conservato").unwrap();
        fs::write(&outside, "non toccare").unwrap();
        let report = install_asset_with(&asset, &cache, |_| Ok(Some(0))).unwrap();
        if cfg!(target_os = "windows") {
            assert!(report.success && report.cache_cleared);
            assert!(!asset.exists());
        } else {
            assert_eq!(report.outcome, "manual");
            assert!(report.instructions.is_some());
            assert!(asset.exists());
        }
        assert!(unknown.exists());
        assert_eq!(fs::read_to_string(outside).unwrap(), "non toccare");
    }

    #[test]
    fn failed_install_preserves_verified_asset_for_retry() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let asset = cache.join(update_asset_name());
        fs::write(&asset, "installer").unwrap();
        let result = install_asset_with(&asset, &cache, |_| Ok(Some(17)));
        if cfg!(target_os = "windows") {
            assert!(result.is_err());
            assert!(asset.exists());
        } else {
            assert_eq!(result.unwrap().outcome, "manual");
        }
    }

    #[test]
    fn installer_rejects_assets_outside_the_managed_cache() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("cache");
        fs::create_dir(&cache).unwrap();
        let asset = temp.path().join(update_asset_name());
        fs::write(&asset, "non autorizzato").unwrap();
        assert!(install_asset_with(&asset, &cache, |_| Ok(Some(0))).is_err());
        assert!(asset.exists());
    }
}

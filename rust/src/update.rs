use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ArchiveError;

const LATEST_RELEASE: &str = "https://api.github.com/repos/WolCarlos/Wol-rar/releases/latest";

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

pub fn check_latest() -> Result<ReleaseInfo, ArchiveError> {
    let release: ReleaseInfo = serde_json::from_slice(&fetch(LATEST_RELEASE, 2 * 1024 * 1024)?)
        .map_err(|error| ArchiveError::Message(format!("Risposta GitHub non valida: {error}")))?;
    Version::parse(release.tag_name.trim_start_matches('v'))
        .map_err(|error| ArchiveError::Message(format!("Versione release non valida: {error}")))?;
    release.asset(&update_asset_name())?;
    release.asset("SHA256SUMS.txt")?;
    Ok(release)
}

pub fn download_latest(destination: &Path) -> Result<UpdateDownload, ArchiveError> {
    let release = check_latest()?;
    let asset = release.asset(&update_asset_name())?;
    let checksums = String::from_utf8(fetch(
        &release.asset("SHA256SUMS.txt")?.browser_download_url,
        128 * 1024,
    )?)
    .map_err(|error| ArchiveError::Message(error.to_string()))?;
    let expected = checksum_for(&checksums, &asset.name)?;
    let data = fetch(&asset.browser_download_url, 512 * 1024 * 1024)?;
    let actual = format!("{:x}", Sha256::digest(&data));
    if actual != expected {
        return Err(ArchiveError::Message(
            "Checksum aggiornamento non valido".into(),
        ));
    }
    fs::create_dir_all(destination)?;
    let output = destination.join(&asset.name);
    if output.exists() {
        return Err(ArchiveError::DestinationExists(output));
    }
    let partial = destination.join(format!("{}.part", asset.name));
    fs::write(&partial, data)?;
    fs::rename(&partial, &output)?;
    Ok(UpdateDownload {
        version: release.tag_name,
        asset: asset.name,
        destination: output,
        sha256: actual,
    })
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

fn update_asset_name() -> String {
    if cfg!(target_os = "windows") {
        "Wol-rar-Setup-x64.exe".into()
    } else if cfg!(target_os = "linux") {
        "Wol-rar-Linux-x64.tar.gz".into()
    } else if cfg!(target_os = "macos") {
        "Wol-rar-macOS.tar.gz".into()
    } else {
        String::new()
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
        if name.trim_start_matches('*') == asset_name && digest.len() == 64 {
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
    fn parses_gnu_checksums() {
        let digest = "a".repeat(64);
        assert_eq!(
            checksum_for(&format!("{digest} *app.exe"), "app.exe").unwrap(),
            digest
        );
        assert_eq!(
            checksum_for(&format!("{digest}  app.exe"), "app.exe").unwrap(),
            digest
        );
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("v99.0.0").unwrap());
        assert!(!is_newer(env!("CARGO_PKG_VERSION")).unwrap());
    }
}

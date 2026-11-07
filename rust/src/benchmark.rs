use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;

use crate::{ArchiveError, CreateOptions};

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkReport {
    pub size_bytes: u64,
    pub archive_bytes: u64,
    pub ratio_percent: f64,
    pub create_seconds: f64,
    pub extract_seconds: f64,
    pub create_mib_per_second: f64,
    pub extract_mib_per_second: f64,
    pub verified: bool,
}

pub fn run(size_mib: u64) -> Result<BenchmarkReport, ArchiveError> {
    if !(1..=4096).contains(&size_mib) {
        return Err(ArchiveError::Message(
            "Dimensione benchmark: usare 1-4096 MiB".into(),
        ));
    }
    let size_bytes = size_mib * 1024 * 1024;
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("benchmark.bin");
    let mut file = File::create(&source)?;
    let mut buffer = vec![0u8; 1024 * 1024];
    for _ in 0..size_mib {
        getrandom::fill(&mut buffer).map_err(|error| ArchiveError::Message(error.to_string()))?;
        file.write_all(&buffer)?;
    }
    file.sync_all()?;

    let archive = temporary.path().join("benchmark.wolrar");
    let options = CreateOptions::default();
    let start = Instant::now();
    crate::wolformat_v2::create(&archive, std::slice::from_ref(&source), &options, None)?;
    let create_seconds = start.elapsed().as_secs_f64().max(0.000_001);
    let archive_bytes = fs::metadata(&archive)?.len();

    let destination = temporary.path().join("estratto");
    let start = Instant::now();
    crate::wolformat_v2::extract(&archive, &destination, None)?;
    let extract_seconds = start.elapsed().as_secs_f64().max(0.000_001);
    let extracted = destination.join("benchmark.bin");
    if digest(&source)? != digest(&extracted)? {
        return Err(ArchiveError::Message("Digest benchmark non valido".into()));
    }
    let mib = size_mib as f64;
    Ok(BenchmarkReport {
        size_bytes,
        archive_bytes,
        ratio_percent: archive_bytes as f64 * 100.0 / size_bytes as f64,
        create_seconds,
        extract_seconds,
        create_mib_per_second: mib / create_seconds,
        extract_mib_per_second: mib / extract_seconds,
        verified: true,
    })
}

fn digest(path: &std::path::Path) -> Result<[u8; 32], ArchiveError> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(&mut file)?;
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_measures_and_verifies() {
        let report = run(1).unwrap();
        assert!(report.verified);
        assert!(report.create_mib_per_second > 0.0);
        assert!(report.extract_mib_per_second > 0.0);
    }
}

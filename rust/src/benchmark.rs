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

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkCaseReport {
    pub name: String,
    pub format: String,
    pub archive_bytes: u64,
    pub create_seconds: f64,
    pub extract_seconds: f64,
    pub repair_seconds: Option<f64>,
    pub create_mib_per_second: f64,
    pub extract_mib_per_second: f64,
    pub volume_count: u32,
    pub recovery_count: u32,
    pub encrypted: bool,
    pub verified: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkSuiteReport {
    pub size_bytes: u64,
    pub cases: Vec<BenchmarkCaseReport>,
}

pub fn run_suite(size_mib: u64) -> Result<BenchmarkSuiteReport, ArchiveError> {
    if !(1..=4096).contains(&size_mib) {
        return Err(ArchiveError::Message(
            "Dimensione benchmark: usare 1-4096 MiB".into(),
        ));
    }
    let temporary = tempfile::tempdir()?;
    let source = temporary.path().join("benchmark.bin");
    let mut file = File::create(&source)?;
    let mut buffer = vec![0u8; 1024 * 1024];
    for _ in 0..size_mib {
        getrandom::fill(&mut buffer).map_err(|error| ArchiveError::Message(error.to_string()))?;
        file.write_all(&buffer)?;
    }
    file.sync_all()?;

    let plain = CreateOptions::default();
    let encrypted = CreateOptions {
        password: Some(zeroize::Zeroizing::new("benchmark-locale".into())),
        ..CreateOptions::default()
    };
    let volumes = CreateOptions {
        volume_size: Some(65536),
        ..CreateOptions::default()
    };
    let recovery = CreateOptions {
        volume_size: Some(65536),
        recovery_percent: 10,
        ..CreateOptions::default()
    };
    let mut cases = Vec::new();
    for (name, extension, options, repair) in [
        ("wolrar-plain", "wolrar", plain, false),
        ("wolrar-encrypted", "wolrar", encrypted, false),
        ("wolrar-volumes", "wolrar", volumes, false),
        ("wolrar-recovery", "wolrar", recovery, true),
        ("zip-comparative", "zip", CreateOptions::default(), false),
        ("7z-comparative", "7z", CreateOptions::default(), false),
    ] {
        cases.push(run_case(
            temporary.path(),
            &source,
            name,
            extension,
            options,
            repair,
            size_mib,
        )?);
    }
    Ok(BenchmarkSuiteReport {
        size_bytes: size_mib * 1024 * 1024,
        cases,
    })
}

fn run_case(
    root: &std::path::Path,
    source: &std::path::Path,
    name: &str,
    extension: &str,
    options: CreateOptions,
    repair: bool,
    size_mib: u64,
) -> Result<BenchmarkCaseReport, ArchiveError> {
    let archive = root.join(format!("{name}.{extension}"));
    let start = Instant::now();
    crate::RustArchiveEngine::create(
        &archive,
        std::slice::from_ref(&source.to_path_buf()),
        &options,
    )?;
    let create_seconds = start.elapsed().as_secs_f64().max(0.000_001);
    let archive_bytes = artifact_bytes(root, name)?;

    let destination = root.join(format!("{name}-estratto"));
    let password = options.password.as_ref().map(|value| value.as_str());
    let start = Instant::now();
    crate::RustArchiveEngine::extract(&archive, &destination, password)?;
    let extract_seconds = start.elapsed().as_secs_f64().max(0.000_001);
    if digest(source)? != digest(&destination.join("benchmark.bin"))? {
        return Err(ArchiveError::Message(format!(
            "Digest non valido nel benchmark {name}"
        )));
    }

    let (volume_count, recovery_count) = if extension == "wolrar" {
        (
            crate::wolformat_v2::inspect(&archive)?.volume_count,
            crate::wolrecovery::recovery_count(&archive),
        )
    } else {
        (1, 0)
    };
    let repair_seconds = if repair {
        let second = root.join(format!("{name}.w02"));
        if !second.exists() {
            return Err(ArchiveError::Message(
                "Il benchmark recovery non ha creato abbastanza volumi".into(),
            ));
        }
        fs::remove_file(second)?;
        let repair_destination = root.join(format!("{name}-riparato"));
        let start = Instant::now();
        let report = crate::RustArchiveEngine::repair(&archive, &repair_destination)?;
        let elapsed = start.elapsed().as_secs_f64().max(0.000_001);
        if !report.verified {
            return Err(ArchiveError::Message(
                "Verifica recovery benchmark non riuscita".into(),
            ));
        }
        let repaired = repair_destination.join(format!("{name}.wolrar"));
        crate::RustArchiveEngine::test(&repaired, None)?;
        Some(elapsed)
    } else {
        None
    };
    let mib = size_mib as f64;
    Ok(BenchmarkCaseReport {
        name: name.into(),
        format: extension.into(),
        archive_bytes,
        create_seconds,
        extract_seconds,
        repair_seconds,
        create_mib_per_second: mib / create_seconds,
        extract_mib_per_second: mib / extract_seconds,
        volume_count,
        recovery_count,
        encrypted: options.password.is_some(),
        verified: true,
    })
}

fn artifact_bytes(root: &std::path::Path, name: &str) -> Result<u64, ArchiveError> {
    let mut total = 0u64;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == format!("{name}.wolrar")
            || file_name == format!("{name}.zip")
            || file_name == format!("{name}.7z")
            || (file_name.starts_with(&format!("{name}.w"))
                && file_name != format!("{name}.wolrar"))
            || (file_name.starts_with(&format!("{name}.r")) && entry.file_type()?.is_file())
        {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
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

    #[test]
    fn comparative_suite_profiles_encryption_volumes_and_recovery() {
        let report = run_suite(1).unwrap();
        assert_eq!(report.cases.len(), 6);
        assert!(report.cases.iter().all(|case| case.verified));
        assert!(report.cases.iter().any(|case| case.encrypted));
        assert!(report.cases.iter().any(|case| case.volume_count > 1));
        assert!(
            report
                .cases
                .iter()
                .any(|case| case.repair_seconds.is_some())
        );
        assert!(report.cases.iter().any(|case| case.format == "zip"));
        assert!(report.cases.iter().any(|case| case.format == "7z"));
    }
}

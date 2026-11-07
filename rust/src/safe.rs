use std::path::{Component, Path, PathBuf};

use crate::ArchiveError;

pub fn validate_member_path(value: &str) -> Result<PathBuf, ArchiveError> {
    if value.is_empty() || value.contains('\0') || value.contains(':') {
        return Err(ArchiveError::UnsafePath(value.to_owned()));
    }
    let path = Path::new(value);
    if path.is_absolute() {
        return Err(ArchiveError::UnsafePath(value.to_owned()));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            _ => return Err(ArchiveError::UnsafePath(value.to_owned())),
        }
    }
    Ok(path.to_path_buf())
}

pub fn safe_destination(base: &Path, member: &str) -> Result<PathBuf, ArchiveError> {
    let relative = validate_member_path(member)?;
    let destination = base.join(relative);
    if !destination.starts_with(base) {
        return Err(ArchiveError::UnsafePath(member.to_owned()));
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_absolute_and_ads() {
        for value in ["../fuori", "a/../../fuori", "/assoluto", "C:/Windows/x", "file:ads", ""] {
            assert!(validate_member_path(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn accepts_normal_paths() {
        assert!(validate_member_path("cartella/documento.txt").is_ok());
        assert!(validate_member_path("cartella\\documento.txt").is_ok());
    }
}

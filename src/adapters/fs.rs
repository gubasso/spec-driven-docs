//! Filesystem primitives the lifecycle services share.
//!
//! Hashing, guarded destination checks, and parent-creating writes — the
//! mechanics only. Which destinations exist and what to do about a refusal
//! is the installer's and upgrader's business.

use camino::Utf8Path;

use crate::domain::ownership::Sha256;

/// Hash a file's bytes.
///
/// # Errors
///
/// Any I/O error reading the file.
pub fn sha256_file(path: &Utf8Path) -> std::io::Result<Sha256> {
    Ok(Sha256::of(&std::fs::read(path)?))
}

/// Write a file, creating its parent directories.
///
/// # Errors
///
/// Any I/O error creating directories or writing.
pub fn write_file(path: &Utf8Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)
}

/// Why a destination cannot be touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinationRefusal {
    /// A symlink on the path would resolve the write outside the target.
    SymlinkEscape,
    /// A non-directory sits where a directory is needed.
    FileBlocksDirectory(String),
    /// The destination exists and is not a regular file.
    NotARegularFile,
}

/// Check that touching `destination` under `target` stays inside the target.
///
/// No symlink on any component, directories where directories are needed,
/// and nothing but a regular file (or nothing) at the end.
///
/// # Errors
///
/// A [`DestinationRefusal`] naming what was found.
pub fn check_destination(
    target: &Utf8Path,
    destination: &Utf8Path,
) -> Result<(), DestinationRefusal> {
    let mut prefix = target.to_path_buf();
    let components: Vec<&str> = destination.as_str().split('/').collect();
    for part in &components[..components.len().saturating_sub(1)] {
        prefix.push(part);
        if prefix.is_symlink() {
            return Err(DestinationRefusal::SymlinkEscape);
        }
        if prefix.exists() && !prefix.is_dir() {
            let blocked = prefix
                .as_str()
                .strip_prefix(target.as_str())
                .map_or(prefix.as_str(), |rest| rest.trim_start_matches('/'));
            return Err(DestinationRefusal::FileBlocksDirectory(blocked.to_string()));
        }
    }
    let full = target.join(destination);
    if full.is_symlink() {
        return Err(DestinationRefusal::SymlinkEscape);
    }
    if full.exists() && !full.is_file() {
        return Err(DestinationRefusal::NotARegularFile);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;

    use super::*;

    fn root(dir: &tempfile::TempDir) -> Utf8PathBuf {
        Utf8PathBuf::from(dir.path().to_str().unwrap())
    }

    #[test]
    fn accepts_a_fresh_and_an_existing_regular_destination() {
        let dir = tempfile::tempdir().unwrap();
        let target = root(&dir);
        assert_eq!(
            check_destination(&target, Utf8Path::new("a/b/c.md")),
            Ok(())
        );
        write_file(&target.join("a/b/c.md"), b"x").unwrap();
        assert_eq!(
            check_destination(&target, Utf8Path::new("a/b/c.md")),
            Ok(())
        );
    }

    #[test]
    fn refuses_a_symlinked_component_and_a_symlinked_destination() {
        let dir = tempfile::tempdir().unwrap();
        let target = root(&dir);
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), target.join("a").as_std_path()).unwrap();
        assert_eq!(
            check_destination(&target, Utf8Path::new("a/c.md")),
            Err(DestinationRefusal::SymlinkEscape)
        );
        std::os::unix::fs::symlink("/etc/hosts", target.join("link.md").as_std_path()).unwrap();
        assert_eq!(
            check_destination(&target, Utf8Path::new("link.md")),
            Err(DestinationRefusal::SymlinkEscape)
        );
    }

    #[test]
    fn refuses_a_file_where_a_directory_is_needed_and_a_directory_destination() {
        let dir = tempfile::tempdir().unwrap();
        let target = root(&dir);
        write_file(&target.join("a"), b"file").unwrap();
        assert_eq!(
            check_destination(&target, Utf8Path::new("a/c.md")),
            Err(DestinationRefusal::FileBlocksDirectory("a".to_string()))
        );
        std::fs::create_dir(target.join("d.md")).unwrap();
        assert_eq!(
            check_destination(&target, Utf8Path::new("d.md")),
            Err(DestinationRefusal::NotARegularFile)
        );
    }

    #[test]
    fn hashes_match_the_domain_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("x");
        write_file(&path, b"payload").unwrap();
        assert_eq!(sha256_file(&path).unwrap(), Sha256::of(b"payload"));
    }
}

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

/// Write a file through a sibling temporary file and a rename, creating its
/// parent directories.
///
/// The scratch file is created exclusively, so a path that already exists
/// there, a symlink to somewhere else included, refuses the write rather
/// than being followed or truncated. A failure partway leaves the
/// destination as it was rather than half-written, and an interruption
/// leaves either the old bytes or the new.
///
/// # Errors
///
/// Any I/O error creating directories, creating or writing the scratch
/// file, or renaming it into place. A scratch path that already exists is
/// [`std::io::ErrorKind::AlreadyExists`].
pub fn write_atomic(path: &Utf8Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let scratch = path.with_extension(format!("{}.sdd-tmp", path.extension().unwrap_or_default()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&scratch)
        .map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("{scratch}: {error}; remove the scratch file to retry"),
            )
        })?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(error) = written {
        let _ = std::fs::remove_file(&scratch);
        return Err(error);
    }
    std::fs::rename(&scratch, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&scratch);
    })
}

/// Write a repository-relative destination under a target, refusing a path
/// that leaves it.
///
/// [`check_destination`] runs first, so a symlinked parent directory is
/// refused before a byte lands, and the write itself is [`write_atomic`].
///
/// # Errors
///
/// A [`std::io::ErrorKind::PermissionDenied`] error naming the refusal when
/// the destination cannot be touched, and any I/O error of the write.
pub fn write_within(
    target: &Utf8Path,
    destination: &Utf8Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    check_destination(target, destination).map_err(|refusal| refused(destination, &refusal))?;
    write_atomic(&target.join(destination), bytes)
}

/// Remove a repository-relative file under a target, refusing a path that
/// leaves it.
///
/// # Errors
///
/// A [`std::io::ErrorKind::PermissionDenied`] error naming the refusal when
/// the destination cannot be touched, and any I/O error of the removal.
pub fn remove_within(target: &Utf8Path, destination: &Utf8Path) -> std::io::Result<()> {
    check_destination(target, destination).map_err(|refusal| refused(destination, &refusal))?;
    std::fs::remove_file(target.join(destination))
}

fn refused(destination: &Utf8Path, refusal: &DestinationRefusal) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        format!("{destination}: {refusal}"),
    )
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

impl std::fmt::Display for DestinationRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SymlinkEscape => f.write_str("reached through a symlink that leaves the target"),
            Self::FileBlocksDirectory(blocked) => {
                write!(f, "a file blocks a directory the write needs: {blocked}")
            }
            Self::NotARegularFile => f.write_str("exists and is not a regular file"),
        }
    }
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
    fn an_atomic_write_lands_the_bytes_and_leaves_no_scratch_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("a/debt.yaml");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        let siblings: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(siblings, vec!["debt.yaml".to_string()]);
    }

    #[test]
    fn a_pre_existing_scratch_symlink_is_refused_and_nothing_outside_is_touched() {
        let dir = tempfile::tempdir().unwrap();
        let target = root(&dir);
        let outside = tempfile::tempdir().unwrap();
        let victim = outside.path().join("victim");
        std::fs::write(&victim, b"keep").unwrap();
        std::os::unix::fs::symlink(&victim, target.join("debt.yaml.sdd-tmp").as_std_path())
            .unwrap();
        let error = write_atomic(&target.join("debt.yaml"), b"new").unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            std::fs::read(&victim).unwrap(),
            b"keep",
            "the scratch symlink was followed"
        );
        assert!(!target.join("debt.yaml").exists());
        // A stale regular scratch file refuses the same way, and stays.
        std::fs::remove_file(target.join("debt.yaml.sdd-tmp")).unwrap();
        std::fs::write(target.join("debt.yaml.sdd-tmp"), b"stale").unwrap();
        assert!(write_atomic(&target.join("debt.yaml"), b"new").is_err());
    }

    #[test]
    fn a_write_within_refuses_a_symlinked_parent_before_any_byte_lands() {
        let dir = tempfile::tempdir().unwrap();
        let target = root(&dir);
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), target.join("docs").as_std_path()).unwrap();
        let error = write_within(&target, Utf8Path::new("docs/x.md"), b"x").unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(!outside.path().join("x.md").exists(), "the write escaped");
        assert!(
            remove_within(&target, Utf8Path::new("docs/x.md")).is_err(),
            "the removal followed the symlink"
        );
        write_within(&target, Utf8Path::new("inside/x.md"), b"x").unwrap();
        assert_eq!(std::fs::read(target.join("inside/x.md")).unwrap(), b"x");
    }

    #[test]
    fn hashes_match_the_domain_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = root(&dir).join("x");
        write_file(&path, b"payload").unwrap();
        assert_eq!(sha256_file(&path).unwrap(), Sha256::of(b"payload"));
    }
}

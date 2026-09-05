//! A narrow, hardened boundary to `git ls-remote`.
//!
//! The only network operation this binary performs. It reads one reference
//! from one credential-free `https://` repository and returns the object ID
//! the reference points at. Everything about it is bounded: the transport is
//! allowlisted, the process runs with no shell, no credential prompt, and no
//! inherited Git configuration, and its time and output are capped. It writes
//! nothing and downloads no code — a reference is a name and a hash.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use thiserror::Error;

/// A bound on how long the lookup may run.
const TIMEOUT: Duration = Duration::from_secs(20);
/// A bound on the bytes read from the child's streams.
const MAX_OUTPUT: usize = 1024 * 1024;

/// Why a lookup could not produce a result.
#[derive(Debug, Error)]
pub enum GitError {
    /// The repository URL or the reference is not one this adapter accepts.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// `git` is not on `PATH`.
    #[error("git is not available on PATH")]
    MissingGit,
    /// The lookup exceeded its time bound.
    #[error("git ls-remote timed out")]
    Timeout,
    /// `git` failed or the transport did.
    #[error("git ls-remote failed: {0}")]
    Transport(String),
    /// The output was not the `<sha>\t<ref>` shape expected.
    #[error("git ls-remote produced output this adapter cannot read")]
    Malformed,
}

/// Validate that a repository URL is a credential-free `https://` URL and a
/// reference is a full `refs/...` name. Returns the reason on refusal.
///
/// # Errors
///
/// [`GitError::Unsupported`] naming what was rejected.
pub fn accept(repository: &str, reference: &str) -> Result<(), GitError> {
    let refuse = |m: &str| Err(GitError::Unsupported(m.to_string()));
    if repository.starts_with('-') || reference.starts_with('-') {
        return refuse("an option-like value");
    }
    if repository.bytes().any(|b| b.is_ascii_control())
        || reference.bytes().any(|b| b.is_ascii_control())
    {
        return refuse("a control character");
    }
    if !repository.starts_with("https://") {
        return refuse("the repository is not an https:// URL");
    }
    // `https://user:pass@host` carries credentials; `ext::`, `file://`, and
    // scp-style `host:path` are other transports.
    let after_scheme = &repository["https://".len()..];
    if after_scheme.contains('@') {
        return refuse("the repository URL carries credentials");
    }
    if !reference.starts_with("refs/") {
        return refuse("the reference is not a full refs/... name");
    }
    Ok(())
}

fn read_capped<R: Read>(stream: Option<R>) -> String {
    let mut buf = Vec::new();
    if let Some(s) = stream {
        let _ = s.take(MAX_OUTPUT as u64).read_to_end(&mut buf);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// The full object ID `reference` points at in `repository`, or `None` when
/// the reference is absent.
///
/// # Errors
///
/// [`GitError`] for a refused input, a missing `git`, a timeout, a transport
/// failure, or output this adapter cannot read.
pub fn ls_remote(repository: &str, reference: &str) -> Result<Option<String>, GitError> {
    accept(repository, reference)?;

    let mut child = Command::new("git")
        // No inherited config can rewrite the URL or install a helper.
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ASKPASS", "/bin/true")
        .env_remove("GIT_CONFIG")
        .arg("-c")
        .arg("credential.helper=")
        .arg("-c")
        .arg("protocol.ext.allow=never")
        .arg("-c")
        .arg("protocol.file.allow=never")
        .arg("ls-remote")
        // End option parsing, so a repository value can never be read as a flag.
        .arg("--")
        .arg(repository)
        .arg(reference)
        // Run outside any repository, so a local checkout cannot answer.
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::MissingGit
            } else {
                GitError::Transport(e.to_string())
            }
        })?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if start.elapsed() > TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(GitError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(GitError::Transport(e.to_string())),
        }
    }

    let stdout = read_capped(child.stdout.take());
    let stderr = read_capped(child.stderr.take());
    let status = child
        .wait()
        .map_err(|e| GitError::Transport(e.to_string()))?;

    if !status.success() {
        // Redact everything but a short, fixed reason: remote stderr can echo
        // a URL or a credential prompt.
        let _ = stderr;
        return Err(GitError::Transport(
            "the remote could not be reached".to_string(),
        ));
    }

    // A present reference is one `<40|64 hex>\t<ref>` line. No line means the
    // reference is absent, which is a report state, not an error.
    let mut found = None;
    for line in stdout.lines() {
        let Some((sha, name)) = line.split_once('\t') else {
            return Err(GitError::Malformed);
        };
        let is_hex =
            |s: &str| (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit());
        if !is_hex(sha) {
            return Err(GitError::Malformed);
        }
        if name == reference {
            if found.is_some() {
                return Err(GitError::Malformed);
            }
            found = Some(sha.to_lowercase());
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_plain_https_url_and_a_full_ref() {
        assert!(accept("https://github.com/o/r", "refs/tags/v1").is_ok());
    }

    #[test]
    fn rejects_every_non_allowlisted_input() {
        for (repo, reference) in [
            ("http://github.com/o/r", "refs/tags/v1"),
            ("https://user:pass@github.com/o/r", "refs/tags/v1"),
            ("ext::sh -c whoami", "refs/tags/v1"),
            ("file:///etc", "refs/tags/v1"),
            ("git@github.com:o/r", "refs/tags/v1"),
            ("-oProxyCommand=x", "refs/tags/v1"),
            ("https://github.com/o/r", "v1"),
            ("https://github.com/o/r", "-x"),
            ("https://github.com/o/r\n", "refs/tags/v1"),
        ] {
            assert!(
                matches!(accept(repo, reference), Err(GitError::Unsupported(_))),
                "accepted {repo:?} {reference:?}"
            );
        }
    }
}

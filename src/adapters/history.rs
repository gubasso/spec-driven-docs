//! A narrow, read-only boundary to one repository's local Git history.
//!
//! It answers whether any commit touched a path, and the few questions a
//! migration asks of the history it rests on: the commit a reference
//! names, which listed paths differ from it, and the bytes a path held at
//! a commit. It runs
//! `git` with no shell, no inherited configuration, and no network, reads
//! only the target's own object store, and writes nothing. Where `git` is
//! absent or the target is no repository, it says which, because a missing
//! history is a fact the report states rather than a history that is empty.

use std::process::{Command, Stdio};

use camino::Utf8Path;

/// Why no history can be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HistoryMissing {
    /// `git` is not on `PATH`.
    #[error("git is not on PATH")]
    NoGit,
    /// The target is not inside a Git work tree.
    #[error("the target is not a Git repository")]
    NoRepository,
}

/// One repository's history, opened once.
#[derive(Debug, Clone)]
pub struct History {
    root: camino::Utf8PathBuf,
    /// Whether the repository has a commit at all.
    committed: bool,
}

/// The error a `git` run that exited unsuccessfully stands for.
fn failed(run: &str, status: std::process::ExitStatus) -> std::io::Error {
    std::io::Error::other(format!("{run} failed ({status})"))
}

/// A `git` invocation that reads only the repository it names.
///
/// Every `GIT_*` variable the caller inherited is dropped first. A Git hook
/// exports `GIT_DIR` and `GIT_INDEX_FILE` to its children, and a command
/// that kept them would act on the repository running the hook rather than
/// the one it names.
fn git(root: &Utf8Path) -> Command {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(root.as_str())
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

impl History {
    /// Open the history of the repository `root` sits in.
    ///
    /// # Errors
    ///
    /// [`HistoryMissing`] naming why there is none to read.
    pub fn open(root: &Utf8Path) -> Result<Self, HistoryMissing> {
        let inside = git(root)
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .map_err(|_| HistoryMissing::NoGit)?;
        if !inside.status.success() || inside.stdout.trim_ascii() != b"true" {
            return Err(HistoryMissing::NoRepository);
        }
        let committed = git(root)
            .args(["rev-parse", "--verify", "--quiet", "HEAD"])
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        Ok(Self {
            root: root.to_owned(),
            committed,
        })
    }

    /// Whether any commit reachable from `HEAD` touched `path`.
    ///
    /// A path is passed after `--`, so no value is ever read as an option.
    /// A read that fails answers no, which lowers confidence rather than
    /// raising it.
    #[must_use]
    pub fn touches(&self, path: &Utf8Path) -> bool {
        if !self.committed {
            return false;
        }
        git(&self.root)
            .args(["log", "-1", "--format=%H", "--"])
            .arg(path.as_str())
            .output()
            .is_ok_and(|out| out.status.success() && !out.stdout.trim_ascii().is_empty())
    }

    /// The full object ID `HEAD` names, where the repository has a commit.
    #[must_use]
    pub fn head(&self) -> Option<String> {
        self.revision("HEAD")
    }

    /// The full object ID one revision expression names.
    fn revision(&self, expression: &str) -> Option<String> {
        if !self.committed {
            return None;
        }
        let out = git(&self.root)
            .args(["rev-parse", "--verify", "--quiet", "--end-of-options"])
            .arg(expression)
            .output()
            .ok()?;
        let id = String::from_utf8(out.stdout).ok()?.trim().to_string();
        (out.status.success() && !id.is_empty()).then_some(id)
    }

    /// The local branch the remote's `HEAD` names, as `refs/heads/<name>`,
    /// where `origin` names one and this clone holds that branch.
    ///
    /// A local branch rather than the remote's copy, so an edit integrated
    /// here counts before it is pushed.
    #[must_use]
    pub fn default_trunk(&self) -> Option<String> {
        let out = git(&self.root)
            .args(["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"])
            .output()
            .ok()?;
        let named = String::from_utf8(out.stdout).ok()?;
        let branch = named.trim().strip_prefix("refs/remotes/origin/")?;
        let local = format!("refs/heads/{branch}");
        (out.status.success() && self.commit_of(&local).is_some()).then_some(local)
    }

    /// The full object ID of the commit `reference` names, where it
    /// resolves to one.
    #[must_use]
    pub fn commit_of(&self, reference: &str) -> Option<String> {
        self.revision(&format!("{reference}^{{commit}}"))
    }

    /// Every listed path whose working-tree bytes are not, byte for byte, a
    /// regular file `commit` holds: absent there, a link there, edited,
    /// ignored, or converted on checkout.
    ///
    /// The bytes are hashed with no filter, so a checkout that converts line
    /// endings reads as different: a rebuild from Git would recreate the
    /// blob, not the converted copy. A path Git's line-based input cannot
    /// carry is reported too, because this cannot vouch for it.
    ///
    /// # Errors
    ///
    /// The error of a `git` run that fails, so a broken tool is never
    /// reported as edited documents.
    pub fn differing(
        &self,
        commit: &str,
        paths: &[&Utf8Path],
    ) -> std::io::Result<Vec<camino::Utf8PathBuf>> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        if !self.committed {
            return Ok(paths.iter().map(|path| path.to_path_buf()).collect());
        }
        let (carried, uncarried): (Vec<&Utf8Path>, Vec<&Utf8Path>) = paths
            .iter()
            .copied()
            .partition(|path| !path.as_str().contains('\n'));
        let held = self.regular_blobs(commit, &carried)?;
        let hashed = self.hashed(&carried)?;
        let mut found: Vec<camino::Utf8PathBuf> = carried
            .iter()
            .zip(hashed)
            .filter(|(path, id)| held.get(path.as_str()) != Some(id))
            .map(|(path, _)| path.to_path_buf())
            .chain(uncarried.iter().map(|path| path.to_path_buf()))
            .collect();
        found.sort();
        Ok(found)
    }

    /// The blob ID of each listed path `commit` holds as a regular file, or
    /// of every such file where none is listed.
    fn regular_blobs(
        &self,
        commit: &str,
        paths: &[&Utf8Path],
    ) -> std::io::Result<std::collections::BTreeMap<String, String>> {
        let out = git(&self.root)
            .args([
                "--literal-pathspecs",
                "ls-tree",
                "-r",
                "-z",
                "--end-of-options",
            ])
            .arg(commit)
            .args(paths.iter().map(|path| path.as_str()))
            .output()?;
        if !out.status.success() {
            return Err(failed(&format!("git ls-tree {commit}"), out.status));
        }
        let mut held = std::collections::BTreeMap::new();
        for record in out.stdout.split(|byte| *byte == 0) {
            let Ok(record) = std::str::from_utf8(record) else {
                continue;
            };
            let Some((meta, path)) = record.split_once('\t') else {
                continue;
            };
            let mut fields = meta.split(' ');
            if let (Some("100644" | "100755"), Some("blob"), Some(id)) =
                (fields.next(), fields.next(), fields.next())
            {
                held.insert(path.to_string(), id.to_string());
            }
        }
        Ok(held)
    }

    /// The blob ID each listed path's working-tree bytes hash to, unfiltered,
    /// in the order listed.
    ///
    /// Git prints each ID as it reads each path, so the paths are written
    /// from a second thread while this one drains the output: one thread
    /// doing both blocks once a corpus fills both pipes.
    fn hashed(&self, paths: &[&Utf8Path]) -> std::io::Result<Vec<String>> {
        use std::io::Write as _;
        if paths.is_empty() {
            return Ok(Vec::new());
        }
        let mut child = git(&self.root)
            .args(["hash-object", "--no-filters", "--stdin-paths"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("git hash-object has no input"))?;
        let listed: Vec<u8> = paths
            .iter()
            .flat_map(|path| path.as_str().bytes().chain(std::iter::once(b'\n')))
            .collect();
        let writer = std::thread::spawn(move || stdin.write_all(&listed));
        // The writer is joined before either result is read, so a failed
        // wait never discards what the writer reports.
        let waited = child.wait_with_output();
        let written = writer.join().unwrap_or_else(|_| {
            Err(std::io::Error::other(
                "writing the paths to git hash-object panicked",
            ))
        });
        let out = match (waited, written) {
            (Ok(out), Ok(())) if out.status.success() => out,
            (Ok(out), Ok(())) => return Err(failed("git hash-object", out.status)),
            (Ok(out), Err(write)) => {
                return Err(std::io::Error::other(format!(
                    "git hash-object exited with {}, and writing its paths failed: {write}",
                    out.status
                )));
            }
            (Err(wait), Ok(())) => return Err(wait),
            (Err(wait), Err(write)) => {
                return Err(std::io::Error::other(format!(
                    "waiting for git hash-object failed: {wait}, and writing its paths failed: {write}"
                )));
            }
        };
        let ids: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect();
        if ids.len() != paths.len() {
            return Err(std::io::Error::other(format!(
                "git hash-object answered {} of {} paths",
                ids.len(),
                paths.len()
            )));
        }
        Ok(ids)
    }

    /// Every regular file `commit` holds, relative to the repository root.
    /// A link is no file here, as it is none to the walk an inventory takes.
    ///
    /// # Errors
    ///
    /// The error of a `git` run that fails.
    pub fn files_at(&self, commit: &str) -> std::io::Result<Vec<camino::Utf8PathBuf>> {
        Ok(self
            .regular_blobs(commit, &[])?
            .into_keys()
            .map(camino::Utf8PathBuf::from)
            .collect())
    }

    /// The bytes `path` held at `commit`, where the commit holds the path.
    #[must_use]
    pub fn show(&self, commit: &str, path: &Utf8Path) -> Option<Vec<u8>> {
        let out = git(&self.root)
            .arg("show")
            .arg(format!("{commit}:{path}"))
            .output()
            .ok()?;
        out.status.success().then_some(out.stdout)
    }

    /// The first parent of the oldest commit on `HEAD`'s first-parent chain
    /// that added `needle` to `path`, which is the commit every byte the
    /// change removed was still held in.
    #[must_use]
    pub fn before_adding(&self, path: &Utf8Path, needle: &str) -> Option<String> {
        if !self.committed {
            return None;
        }
        let out = git(&self.root)
            // The walk follows first parents only, and a merge shows its
            // diff against its first parent. A key a branch recorded is then
            // found at the merge that brought it in, whose first parent holds
            // the trunk's bytes as they were at the merge, never older ones.
            .args([
                "log",
                "--first-parent",
                "--reverse",
                "--format=%H",
                "--diff-merges=first-parent",
                "-S",
            ])
            .arg(needle)
            .args(["--"])
            .arg(path.as_str())
            .output()
            .ok()?;
        let text = String::from_utf8(out.stdout).ok()?;
        let adding = text.lines().next()?.trim().to_string();
        self.revision(&format!("{adding}^"))
    }
}

/// Whether Git's ignore rules cover `path` in the repository at `root`.
///
/// `None` where no answer can be read: no `git`, no repository, or a check
/// that failed for another reason. The caller then reads the ignore file
/// itself rather than guessing.
#[must_use]
pub fn ignores(root: &Utf8Path, path: &Utf8Path) -> Option<bool> {
    let status = git(root)
        .args(["check-ignore", "-q", "--no-index", "--"])
        .arg(path.as_str())
        .stdout(Stdio::null())
        .status()
        .ok()?;
    match status.code() {
        Some(0) => Some(true),
        Some(1) => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "a test panics as its failure signal, not as control flow"
    )]

    use super::*;

    fn git_runs() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    #[test]
    fn a_directory_that_is_no_repository_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let expected = if git_runs() {
            HistoryMissing::NoRepository
        } else {
            HistoryMissing::NoGit
        };
        assert_eq!(History::open(root).unwrap_err(), expected);
    }

    #[test]
    fn a_committed_path_is_touched_and_an_uncommitted_one_is_not() {
        if !git_runs() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let run = |args: &[&str]| {
            assert!(git(root).args(args).status().unwrap().success(), "{args:?}");
        };
        run(&["init", "-q"]);
        std::fs::write(root.join("kept.md"), "x\n").unwrap();
        std::fs::write(root.join("new.md"), "y\n").unwrap();
        let empty = History::open(root).unwrap();
        assert!(!empty.touches(Utf8Path::new("kept.md")), "no commit yet");
        run(&["add", "kept.md"]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "x",
        ]);
        let history = History::open(root).unwrap();
        assert!(history.touches(Utf8Path::new("kept.md")));
        assert!(!history.touches(Utf8Path::new("new.md")));
    }

    #[test]
    fn a_corpus_that_fills_both_pipes_still_hashes() {
        if !git_runs() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap().to_owned();
        assert!(git(&root).args(["init", "-q"]).status().unwrap().success());
        std::fs::create_dir(root.join("docs")).unwrap();
        // About 92 KiB of paths in and 160 KiB of IDs out, past the 64 KiB
        // a pipe holds on Linux.
        let paths: Vec<camino::Utf8PathBuf> = (0..4000)
            .map(|n| camino::Utf8PathBuf::from(format!("docs/document-{n:05}.md")))
            .collect();
        for path in &paths {
            std::fs::write(root.join(path), path.as_str()).unwrap();
        }
        let (sent, received) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let history = History::open(&root).unwrap();
            let listed: Vec<&Utf8Path> = paths.iter().map(camino::Utf8PathBuf::as_path).collect();
            let _ = sent.send(history.hashed(&listed).map(|ids| ids.len()).ok());
        });
        let hashed = received
            .recv_timeout(std::time::Duration::from_secs(120))
            .expect("hashing the corpus hung");
        assert_eq!(hashed, Some(4000));
    }

    #[test]
    fn a_tree_git_cannot_read_is_an_error_and_no_list_of_edits() {
        if !git_runs() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(dir.path()).unwrap();
        let run = |args: &[&str]| {
            assert!(git(root).args(args).status().unwrap().success(), "{args:?}");
        };
        run(&["init", "-q"]);
        std::fs::write(root.join("kept.md"), "x\n").unwrap();
        run(&["add", "kept.md"]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-m",
            "x",
        ]);
        let history = History::open(root).unwrap();
        let paths = [Utf8Path::new("kept.md")];
        assert!(history.differing("HEAD", &paths).unwrap().is_empty());
        assert!(history.differing("no-such-commit", &paths).is_err());
    }
}

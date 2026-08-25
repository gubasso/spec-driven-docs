//! Shared per-invocation context handed to every command handler.
//!
//! Holds only what every handler needs: where the process runs and how loud
//! it should be. Configuration files, color policy, and runtimes are
//! deliberately absent — this binary has none.

use camino::Utf8PathBuf;

use crate::error::AppError;

/// The resolved invocation environment.
#[derive(Debug, Clone)]
pub struct AppContext {
    /// The working directory the process was invoked from.
    pub cwd: Utf8PathBuf,
    /// The `-v` flag count.
    pub verbosity: u8,
}

impl AppContext {
    /// Resolve the context from the process environment.
    ///
    /// # Errors
    ///
    /// [`AppError::Io`] when the working directory cannot be read, and
    /// [`AppError::Usage`] when its path is not UTF-8.
    pub fn new(verbosity: u8) -> Result<Self, AppError> {
        let cwd = std::env::current_dir()?;
        let cwd = Utf8PathBuf::from_path_buf(cwd).map_err(|p| {
            AppError::Usage(format!("working directory is not UTF-8: {}", p.display()))
        })?;
        Ok(Self { cwd, verbosity })
    }
}

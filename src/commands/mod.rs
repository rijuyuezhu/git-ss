//! Subcommand orchestration modules and shared command errors.

use thiserror::Error;

use crate::git::GitSsError;

/// Result type returned by command handlers.
pub type Result<T> = std::result::Result<T, CommandError>;

/// Errors returned while running subcommands.
#[derive(Debug, Error)]
pub enum CommandError {
    /// A Git operation failed.
    #[error(transparent)]
    Git(#[from] GitSsError),
    /// A filesystem or output operation failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// CSV output could not be written.
    #[error(transparent)]
    Csv(#[from] csv::Error),
    /// JSON output could not be written.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// `--include-ignored` is only valid for working-directory uploads.
    #[error("--include-ignored can only be used with upload workdir")]
    IncludeIgnoredRequiresWorkdir,
}

/// Implementation of `git-ss download`.
pub mod download;
/// Implementation of `git-ss list`.
pub mod list;
/// Implementation of `git-ss upload`.
pub mod upload;

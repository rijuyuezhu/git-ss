//! Command-line interface definitions for `git-ss`.

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Top-level command-line parser.
#[derive(Debug, Parser)]
#[command(
    name = "git-ss",
    version,
    about = "Upload and download temporary Git snapshots"
)]
pub struct Cli {
    /// Selected `git-ss` subcommand and its arguments.
    #[command(subcommand)]
    pub command: Command,
}

/// Supported top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Upload a snapshot from an existing ref or the working directory.
    Upload(UploadArgs),
    /// List remote snapshot branches for a remote.
    List(ListArgs),
    /// Download a snapshot by id into a detached HEAD checkout.
    Download(DownloadArgs),
    /// Delete all remote snapshot branches for a remote.
    Clean(CleanArgs),
}

/// Arguments for `git-ss upload`.
#[derive(Debug, Args)]
pub struct UploadArgs {
    /// Remote to push the snapshot branch to.
    #[arg(long, default_value = "origin")]
    pub remote: String,

    /// Snapshot id to use instead of the timestamp-based default.
    #[arg(long)]
    pub id: Option<String>,

    /// Include ignored files when uploading a working-directory snapshot.
    #[arg(long)]
    pub include_ignored: bool,

    /// Upload target, either `workdir` or any ref expression that resolves to a commit.
    pub target: String,
}

/// Parsed upload target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadTarget<'a> {
    /// Snapshot the current working directory and index state.
    Workdir,
    /// Snapshot the tree reachable from a ref or revision expression.
    Ref(&'a str),
}

impl UploadArgs {
    /// Interprets the raw upload target as either `workdir` or a ref.
    pub fn parsed_target(&self) -> UploadTarget<'_> {
        if self.target == "workdir" {
            UploadTarget::Workdir
        } else {
            UploadTarget::Ref(&self.target)
        }
    }
}

/// Arguments for `git-ss list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Remote to fetch snapshot branches from.
    #[arg(long, default_value = "origin")]
    pub remote: String,

    /// Output format for listed snapshots.
    #[arg(long, value_enum, default_value = "human")]
    pub format: ListFormat,
}

/// Supported output formats for `git-ss list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ListFormat {
    /// Human-readable terminal table.
    Human,
    /// Comma-separated raw snapshot data.
    Csv,
    /// JSON raw snapshot data.
    Json,
}

/// Arguments for `git-ss download`.
#[derive(Debug, Args)]
pub struct DownloadArgs {
    /// Remote to fetch the snapshot branch from.
    #[arg(long, default_value = "origin")]
    pub remote: String,

    /// Overwrite local worktree changes and untracked files during checkout.
    #[arg(short, long)]
    pub force: bool,

    /// Snapshot id to download from `refs/heads/gitss/<id>` on the remote.
    pub id: String,
}

/// Arguments for `git-ss clean`.
#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Remote to delete snapshot branches from.
    #[arg(long, default_value = "origin")]
    pub remote: String,
}

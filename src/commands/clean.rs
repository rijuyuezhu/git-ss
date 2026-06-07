//! Command handler for cleaning remote snapshot branches.

use super::Result;
use crate::cli::CleanArgs;
use crate::git::{clean_snapshots, discover_repo};

/// Deletes all snapshot branches from the selected remote.
pub fn run(args: CleanArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo = discover_repo(&current_dir)?;
    let deleted = clean_snapshots(&repo, &args.remote)?;

    println!("deleted {deleted} snapshot branches");
    Ok(())
}

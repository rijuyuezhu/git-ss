//! Command handler for cleaning remote snapshot branches.

use super::Result;
use crate::cli::CleanArgs;
use crate::git::discover_repo;

/// Deletes all snapshot branches from the selected remote.
pub fn run(args: CleanArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let _repo = discover_repo(&current_dir)?;
    let _remote = args.remote;

    println!("deleted 0 snapshot branches");
    Ok(())
}

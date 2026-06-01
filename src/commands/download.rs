//! Command handler for downloading snapshot branches.

use crate::{
    cli::DownloadArgs,
    git::{discover_repo, download_snapshot},
};

use super::Result;

/// Downloads a snapshot and checks it out as a detached HEAD.
pub fn run(args: DownloadArgs) -> Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo = discover_repo(&current_dir)?;
    let oid = download_snapshot(&repo, &args.remote, &args.id, args.force)?;

    println!("checked out {} at {}", args.id, oid);
    Ok(())
}

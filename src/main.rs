//! Binary entry point for the `git-ss` Git plugin.

use clap::Parser;
use git_ss::cli::{Cli, Command};
use git_ss::commands;

/// Parses command-line arguments and dispatches to the selected subcommand.
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Upload(args) => commands::upload::run(args),
        Command::List(args) => commands::list::run(args),
        Command::Download(args) => commands::download::run(args),
        Command::Clean(args) => commands::clean::run(args),
    }?;

    Ok(())
}

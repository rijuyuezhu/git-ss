use clap::Parser;

use git_ss::cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Upload(args) => git_ss::commands::upload::run(args),
        Command::List(args) => git_ss::commands::list::run(args),
        Command::Download(args) => git_ss::commands::download::run(args),
    }
}

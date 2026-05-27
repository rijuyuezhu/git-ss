use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "git-ss",
    version,
    about = "Upload and download temporary Git snapshots"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Upload(UploadArgs),
    List(RemoteArgs),
    Download(DownloadArgs),
}

#[derive(Debug, Args)]
pub struct UploadArgs {
    #[arg(long, default_value = "origin")]
    pub remote: String,

    #[arg(long)]
    pub id: Option<String>,

    #[arg(long)]
    pub include_ignored: bool,

    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadTarget<'a> {
    Workdir,
    Ref(&'a str),
}

impl UploadArgs {
    pub fn parsed_target(&self) -> UploadTarget<'_> {
        if self.target == "workdir" {
            UploadTarget::Workdir
        } else {
            UploadTarget::Ref(&self.target)
        }
    }
}

#[derive(Debug, Args)]
pub struct RemoteArgs {
    #[arg(long, default_value = "origin")]
    pub remote: String,
}

#[derive(Debug, Args)]
pub struct DownloadArgs {
    #[arg(long, default_value = "origin")]
    pub remote: String,

    #[arg(short, long)]
    pub force: bool,

    pub id: String,
}

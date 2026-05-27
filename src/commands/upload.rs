use crate::cli::{UploadArgs, UploadTarget};

pub fn run(args: UploadArgs) -> anyhow::Result<()> {
    if args.include_ignored && !matches!(args.parsed_target(), UploadTarget::Workdir) {
        anyhow::bail!("--include-ignored can only be used with upload workdir");
    }

    anyhow::bail!("upload is not implemented yet")
}

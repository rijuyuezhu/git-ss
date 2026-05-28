//! Command handler for uploading snapshot branches.

use chrono::Local;

use crate::{
    cli::{UploadArgs, UploadTarget},
    git::{SnapshotUpload, discover_repo, upload_ref_snapshot, upload_workdir_snapshot},
    metadata::format_default_id,
};

/// Uploads either an existing ref snapshot or a working-directory snapshot.
pub fn run(args: UploadArgs) -> anyhow::Result<()> {
    let UploadArgs {
        remote,
        id,
        include_ignored,
        target,
    } = args;
    let parsed_target = if target == "workdir" {
        UploadTarget::Workdir
    } else {
        UploadTarget::Ref(&target)
    };

    if include_ignored && !matches!(parsed_target, UploadTarget::Workdir) {
        anyhow::bail!("--include-ignored can only be used with upload workdir");
    }

    match parsed_target {
        UploadTarget::Workdir => {
            let id = id.unwrap_or_else(|| format_default_id(Local::now().fixed_offset()));
            let current_dir = std::env::current_dir()?;
            let repo = discover_repo(&current_dir)?;
            let result = upload_workdir_snapshot(
                &repo,
                SnapshotUpload {
                    id: &id,
                    remote: &remote,
                    source: "HEAD",
                    include_ignored,
                },
            )?;

            println!("{}", result.id);
            Ok(())
        }
        UploadTarget::Ref(source) => {
            let id = id.unwrap_or_else(|| format_default_id(Local::now().fixed_offset()));
            let current_dir = std::env::current_dir()?;
            let repo = discover_repo(&current_dir)?;
            let result = upload_ref_snapshot(
                &repo,
                SnapshotUpload {
                    id: &id,
                    remote: &remote,
                    source,
                    include_ignored: false,
                },
            )?;

            println!("{}", result.id);
            Ok(())
        }
    }
}

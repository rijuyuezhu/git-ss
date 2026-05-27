use crate::{
    cli::RemoteArgs,
    git::{discover_repo, list_snapshots},
    metadata::UploadKind,
};

pub fn run(args: RemoteArgs) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?;
    let repo = discover_repo(&current_dir)?;
    let snapshots = list_snapshots(&repo, &args.remote)?;

    println!("ID\tCREATED\tTYPE\tSOURCE\tCOMMIT\tREF");
    for snapshot in snapshots {
        match snapshot.metadata {
            Ok(metadata) => {
                let kind = match metadata.kind {
                    UploadKind::Workdir => "workdir",
                    UploadKind::Ref => "ref",
                };
                println!(
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    snapshot.id,
                    metadata.created_at.to_rfc3339(),
                    kind,
                    metadata.source,
                    metadata.source_commit,
                    snapshot.remote_ref
                );
            }
            Err(err) => println!(
                "{}\tWARN\tWARN\t{}\t{}\t{}",
                snapshot.id, err, snapshot.commit, snapshot.remote_ref
            ),
        }
    }

    Ok(())
}

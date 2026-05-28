use std::path::Path;

use chrono::Local;
use git2::build::CheckoutBuilder;
use git2::{
    Cred, CredentialType, Direction, ErrorCode, FetchOptions, FetchPrune, IndexAddOption,
    PushOptions, RemoteCallbacks, Repository, Signature, StatusOptions,
};
use thiserror::Error;

use crate::metadata::{self, SnapshotMetadata, UploadKind};

#[derive(Debug, Error)]
pub enum GitSsError {
    #[error("not inside a Git repository")]
    NotRepository,
    #[error("repository has no HEAD; git-ss does not support empty repositories yet")]
    UnbornHead,
    #[error("remote '{0}' is not configured; pass --remote <name> to select another remote")]
    MissingRemote(String),
    #[error("invalid snapshot id '{0}'")]
    InvalidId(String),
    #[error("working tree has local changes; use --force to overwrite them")]
    DirtyWorktree,
    #[error("snapshot '{0}' was not found on the selected remote")]
    MissingSnapshot(String),
    #[error("cannot resolve source ref '{0}' to a commit")]
    UnresolvableRef(String),
    #[error("remote snapshot branch '{0}' already exists")]
    RemoteBranchExists(String),
    #[error(transparent)]
    Git(#[from] git2::Error),
}

pub struct SnapshotUpload<'a> {
    pub id: &'a str,
    pub remote: &'a str,
    pub source: &'a str,
    pub include_ignored: bool,
}

pub struct UploadResult {
    pub id: String,
    pub remote_ref: String,
    pub commit: git2::Oid,
}

#[derive(Debug)]
pub struct ListedSnapshot {
    pub id: String,
    pub remote_ref: String,
    pub commit: git2::Oid,
    pub metadata: Result<SnapshotMetadata, String>,
}

pub fn discover_repo(start: &Path) -> Result<Repository, GitSsError> {
    Repository::discover(start).map_err(|err| match err.code() {
        ErrorCode::NotFound => GitSsError::NotRepository,
        _ => GitSsError::Git(err),
    })
}

pub fn require_head(repo: &Repository) -> Result<git2::Oid, GitSsError> {
    repo.head()
        .map_err(map_head_lookup)?
        .peel_to_commit()
        .map_err(GitSsError::Git)
        .map(|commit| commit.id())
}

pub fn resolve_remote<'repo>(
    repo: &'repo Repository,
    remote_name: &str,
) -> Result<git2::Remote<'repo>, GitSsError> {
    repo.find_remote(remote_name)
        .map_err(|err| match err.code() {
            ErrorCode::NotFound => GitSsError::MissingRemote(remote_name.to_owned()),
            _ => GitSsError::Git(err),
        })
}

pub fn upload_ref_snapshot(
    repo: &Repository,
    input: SnapshotUpload<'_>,
) -> Result<UploadResult, GitSsError> {
    metadata::validate_id(input.id).map_err(|_| GitSsError::InvalidId(input.id.to_owned()))?;
    require_head(repo)?;

    resolve_remote(repo, input.remote)?;
    let remote_ref = format!("refs/heads/gitss/{}", input.id);
    if remote_ref_exists(repo, input.remote, &remote_ref)? {
        return Err(GitSsError::RemoteBranchExists(remote_ref));
    }

    let source_commit = repo
        .revparse_single(input.source)
        .and_then(|object| object.peel_to_commit())
        .map_err(|_| GitSsError::UnresolvableRef(input.source.to_owned()))?;
    let tree = source_commit.tree()?;
    let signature = snapshot_signature(repo)?;
    let metadata = SnapshotMetadata {
        id: input.id.to_owned(),
        kind: UploadKind::Ref,
        source: input.source.to_owned(),
        source_commit: source_commit.id().to_string(),
        created_at: Local::now().fixed_offset(),
        remote: input.remote.to_owned(),
        include_ignored: false,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let message = metadata.to_commit_message();
    let snapshot_commit = repo.commit(
        None,
        &signature,
        &signature,
        &message,
        &tree,
        &[&source_commit],
    )?;

    let temp_ref = format!("refs/gitss/tmp/{}", input.id);
    repo.reference(&temp_ref, snapshot_commit, true, "git-ss snapshot upload")?;
    push_temp_ref(repo, input.remote, &temp_ref, &remote_ref)?;

    Ok(UploadResult {
        id: input.id.to_owned(),
        remote_ref,
        commit: snapshot_commit,
    })
}

pub fn upload_workdir_snapshot(
    repo: &Repository,
    input: SnapshotUpload<'_>,
) -> Result<UploadResult, GitSsError> {
    metadata::validate_id(input.id).map_err(|_| GitSsError::InvalidId(input.id.to_owned()))?;
    let head_id = require_head(repo)?;

    resolve_remote(repo, input.remote)?;
    let remote_ref = format!("refs/heads/gitss/{}", input.id);
    if remote_ref_exists(repo, input.remote, &remote_ref)? {
        return Err(GitSsError::RemoteBranchExists(remote_ref));
    }

    let head_commit = repo.find_commit(head_id)?;
    let mut index = repo.index()?;
    index.update_all(["*"].iter(), None)?;
    let add_option = if input.include_ignored {
        IndexAddOption::FORCE
    } else {
        IndexAddOption::DEFAULT
    };
    index.add_all(["*"].iter(), add_option, None)?;
    let tree_id = index.write_tree_to(repo)?;
    let tree = repo.find_tree(tree_id)?;
    let signature = snapshot_signature(repo)?;
    let metadata = SnapshotMetadata {
        id: input.id.to_owned(),
        kind: UploadKind::Workdir,
        source: "HEAD".to_owned(),
        source_commit: head_commit.id().to_string(),
        created_at: Local::now().fixed_offset(),
        remote: input.remote.to_owned(),
        include_ignored: input.include_ignored,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let message = metadata.to_commit_message();
    let snapshot_commit = repo.commit(
        None,
        &signature,
        &signature,
        &message,
        &tree,
        &[&head_commit],
    )?;

    let temp_ref = format!("refs/gitss/tmp/{}", input.id);
    repo.reference(&temp_ref, snapshot_commit, true, "git-ss snapshot upload")?;
    push_temp_ref(repo, input.remote, &temp_ref, &remote_ref)?;

    Ok(UploadResult {
        id: input.id.to_owned(),
        remote_ref,
        commit: snapshot_commit,
    })
}

pub fn list_snapshots(
    repo: &Repository,
    remote_name: &str,
) -> Result<Vec<ListedSnapshot>, GitSsError> {
    let mut remote = resolve_remote(repo, remote_name)?;
    let refspec = format!("+refs/heads/gitss/*:refs/remotes/{remote_name}/gitss/*");
    let refspecs = [refspec.as_str()];
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(remote_callbacks(repo)?);
    fetch_options.prune(FetchPrune::On);
    remote.fetch(&refspecs, Some(&mut fetch_options), Some("git-ss list"))?;

    let prefix = format!("refs/remotes/{remote_name}/gitss/");
    let glob = format!("{prefix}*");
    let mut snapshots = Vec::new();
    for reference in repo.references_glob(&glob)? {
        let reference = reference?;
        let name = reference.name()?;
        let remote_ref = name.to_owned();
        let id = name.strip_prefix(&prefix).unwrap_or(name).to_owned();
        let commit = reference.peel_to_commit()?;
        let metadata = commit
            .message()
            .map_err(|err| err.to_string())
            .and_then(|message| metadata::parse_metadata(message).map_err(|err| err.to_string()));

        snapshots.push(ListedSnapshot {
            id,
            remote_ref,
            commit: commit.id(),
            metadata,
        });
    }

    snapshots.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(snapshots)
}

pub fn download_snapshot(
    repo: &Repository,
    remote_name: &str,
    id: &str,
    force: bool,
) -> Result<git2::Oid, GitSsError> {
    metadata::validate_id(id).map_err(|_| GitSsError::InvalidId(id.to_owned()))?;

    let mut remote = resolve_remote(repo, remote_name)?;
    let remote_ref = format!("refs/heads/gitss/{id}");
    let local_ref = format!("refs/remotes/{remote_name}/gitss/{id}");
    let refspec = format!("+{remote_ref}:{local_ref}");
    let refspecs = [refspec.as_str()];
    delete_reference_if_exists(repo, &local_ref)?;

    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(remote_callbacks(repo)?);
    remote
        .fetch(&refspecs, Some(&mut fetch_options), Some("git-ss download"))
        .map_err(|err| match err.code() {
            ErrorCode::NotFound => GitSsError::MissingSnapshot(id.to_owned()),
            _ => GitSsError::Git(err),
        })?;

    let snapshot_ref = repo
        .find_reference(&local_ref)
        .map_err(|err| match err.code() {
            ErrorCode::NotFound => GitSsError::MissingSnapshot(id.to_owned()),
            _ => GitSsError::Git(err),
        })?;
    let commit = snapshot_ref.peel_to_commit()?;

    if !force {
        if is_worktree_dirty(repo)? {
            return Err(GitSsError::DirtyWorktree);
        }

        let mut dry_run = CheckoutBuilder::new();
        dry_run.safe().overwrite_ignored(false).dry_run();
        repo.checkout_tree(commit.as_object(), Some(&mut dry_run))
            .map_err(map_checkout_error)?;
    }

    let mut checkout = CheckoutBuilder::new();
    if force {
        checkout.force().remove_untracked(true);
    } else {
        checkout.safe().overwrite_ignored(false);
    }

    repo.checkout_tree(commit.as_object(), Some(&mut checkout))
        .map_err(map_checkout_error)?;
    repo.set_head_detached(commit.id())?;

    Ok(commit.id())
}

fn is_worktree_dirty(repo: &Repository) -> Result<bool, GitSsError> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options))?;

    Ok(statuses.iter().any(|entry| !entry.status().is_empty()))
}

fn remote_ref_exists(
    repo: &Repository,
    remote_name: &str,
    remote_ref: &str,
) -> Result<bool, GitSsError> {
    let mut remote = resolve_remote(repo, remote_name)?;
    let callbacks = remote_callbacks(repo)?;
    let connection = remote.connect_auth(Direction::Push, Some(callbacks), None)?;

    Ok(connection
        .list()?
        .iter()
        .any(|head| head.name() == remote_ref))
}

fn push_ref(
    repo: &Repository,
    remote_name: &str,
    source_ref: &str,
    remote_ref: &str,
) -> Result<(), GitSsError> {
    let mut remote = resolve_remote(repo, remote_name)?;
    let refspec = format!("{source_ref}:{remote_ref}");
    let mut push_failure = None;

    {
        let mut callbacks = remote_callbacks(repo)?;
        callbacks.push_update_reference(|refname, status| {
            if let Some(status) = status {
                push_failure = Some(format!("failed to push {refname}: {status}"));
            }
            Ok(())
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);
        remote.push(&[&refspec], Some(&mut push_options))?;
    }

    if let Some(message) = push_failure {
        return Err(GitSsError::Git(git2::Error::from_str(&message)));
    }

    Ok(())
}

fn push_temp_ref(
    repo: &Repository,
    remote_name: &str,
    temp_ref: &str,
    remote_ref: &str,
) -> Result<(), GitSsError> {
    let push_result = push_ref(repo, remote_name, temp_ref, remote_ref);
    let cleanup_result = delete_reference(repo, temp_ref);

    push_result?;
    cleanup_result?;
    Ok(())
}

fn delete_reference(repo: &Repository, refname: &str) -> Result<(), GitSsError> {
    let mut reference = repo.find_reference(refname)?;
    reference.delete()?;
    Ok(())
}

fn delete_reference_if_exists(repo: &Repository, refname: &str) -> Result<(), GitSsError> {
    match repo.find_reference(refname) {
        Ok(mut reference) => reference.delete().map_err(GitSsError::Git),
        Err(err) if err.code() == ErrorCode::NotFound => Ok(()),
        Err(err) => Err(GitSsError::Git(err)),
    }
}

fn map_checkout_error(err: git2::Error) -> GitSsError {
    match err.code() {
        ErrorCode::Conflict
        | ErrorCode::IndexDirty
        | ErrorCode::Modified
        | ErrorCode::Uncommitted => GitSsError::DirtyWorktree,
        _ => GitSsError::Git(err),
    }
}

fn map_head_lookup(err: git2::Error) -> GitSsError {
    match err.code() {
        ErrorCode::UnbornBranch | ErrorCode::NotFound => GitSsError::UnbornHead,
        _ => GitSsError::Git(err),
    }
}

fn snapshot_signature(repo: &Repository) -> Result<Signature<'_>, GitSsError> {
    match repo.signature() {
        Ok(signature) => Ok(signature),
        Err(_) => Signature::now("git-ss", "git-ss@example.invalid").map_err(GitSsError::Git),
    }
}

fn remote_callbacks(repo: &Repository) -> Result<RemoteCallbacks<'static>, GitSsError> {
    let config = repo.config()?;
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT)
            && let Ok(credential) = Cred::credential_helper(&config, url, username_from_url)
        {
            return Ok(credential);
        }

        if allowed.contains(CredentialType::SSH_KEY) {
            let username = username_from_url.unwrap_or("git");
            if let Ok(credential) = Cred::ssh_key_from_agent(username) {
                return Ok(credential);
            }
        }

        if allowed.contains(CredentialType::USERNAME)
            && let Some(username) = username_from_url
        {
            return Cred::username(username);
        }

        if allowed.contains(CredentialType::DEFAULT) {
            return Cred::default();
        }

        Err(git2::Error::from_str(
            "unsupported credential type for remote",
        ))
    });
    Ok(callbacks)
}

//! Git operation boundary backed by libgit2.
//!
//! This module owns repository discovery, remote authentication callbacks,
//! snapshot commit creation, fetch/push operations, and checkout behavior so
//! command modules do not depend directly on raw `git2` APIs.

use std::path::Path;

use chrono::Local;
use git2::build::CheckoutBuilder;
use git2::{
    Cred, CredentialType, Direction, ErrorCode, FetchOptions, FetchPrune, IndexAddOption,
    PushOptions, RemoteCallbacks, Repository, Signature, StatusOptions,
};
use thiserror::Error;

use crate::metadata::{self, SnapshotMetadata, UploadKind};

/// Errors returned by git-ss Git operations.
#[derive(Debug, Error)]
pub enum GitSsError {
    /// The starting directory is not inside a Git repository.
    #[error("not inside a Git repository")]
    NotRepository,
    /// The repository has no usable `HEAD` commit.
    #[error("repository has no HEAD; git-ss does not support empty repositories yet")]
    UnbornHead,
    /// The selected remote is not configured in the repository.
    #[error("remote '{0}' is not configured; pass --remote <name> to select another remote")]
    MissingRemote(String),
    /// The supplied snapshot id is not safe for use as a branch suffix.
    #[error("invalid snapshot id '{0}'")]
    InvalidId(String),
    /// The working tree cannot be overwritten without `--force`.
    #[error("working tree has local changes; use --force to overwrite them")]
    DirtyWorktree,
    /// The selected remote does not have the requested snapshot branch.
    #[error("snapshot '{0}' was not found on the selected remote")]
    MissingSnapshot(String),
    /// The upload source did not resolve to a commit.
    #[error("cannot resolve source ref '{0}' to a commit")]
    UnresolvableRef(String),
    /// The remote already contains the snapshot branch being uploaded.
    #[error("remote snapshot branch '{0}' already exists")]
    RemoteBranchExists(String),
    /// An underlying libgit2 operation failed.
    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// Parameters used when creating and uploading a snapshot.
pub struct SnapshotUpload<'a> {
    /// Snapshot id and remote branch suffix.
    pub id: &'a str,
    /// Remote name to push to.
    pub remote: &'a str,
    /// Source ref or logical source name recorded in metadata.
    pub source: &'a str,
    /// Whether ignored files should be included for working-directory snapshots.
    pub include_ignored: bool,
}

/// Result returned after a snapshot upload succeeds.
pub struct UploadResult {
    /// Snapshot id that was uploaded.
    pub id: String,
    /// Fully qualified remote branch ref that was created.
    pub remote_ref: String,
    /// Snapshot commit id created by git-ss.
    pub commit: git2::Oid,
}

/// Snapshot entry returned by `list_snapshots`.
#[derive(Debug)]
pub struct ListedSnapshot {
    /// Snapshot id derived from the remote-tracking ref suffix.
    pub id: String,
    /// Remote-tracking ref holding the fetched snapshot commit.
    pub remote_ref: String,
    /// Snapshot commit id.
    pub commit: git2::Oid,
    /// Parsed metadata, or a parse error for malformed snapshot commits.
    pub metadata: Result<SnapshotMetadata, String>,
}

/// Discovers a Git repository starting at `start` or one of its parents.
pub fn discover_repo(start: &Path) -> Result<Repository, GitSsError> {
    Repository::discover(start).map_err(|err| match err.code() {
        ErrorCode::NotFound => GitSsError::NotRepository,
        _ => GitSsError::Git(err),
    })
}

/// Resolves `HEAD` to a commit id, returning a git-ss empty-repository error for unborn heads.
pub fn require_head(repo: &Repository) -> Result<git2::Oid, GitSsError> {
    repo.head()
        .map_err(map_head_lookup)?
        .peel_to_commit()
        .map_err(GitSsError::Git)
        .map(|commit| commit.id())
}

/// Opens a configured remote by name.
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

/// Creates a snapshot commit from an existing ref and pushes it to the selected remote.
pub fn upload_ref_snapshot(
    repo: &Repository,
    input: SnapshotUpload<'_>,
) -> Result<UploadResult, GitSsError> {
    metadata::validate_id(input.id).map_err(|_| GitSsError::InvalidId(input.id.to_owned()))?;
    require_head(repo)?;

    let mut remote = resolve_remote(repo, input.remote)?;
    let remote_ref = format!("refs/heads/gitss/{}", input.id);
    if remote_ref_exists(repo, &mut remote, &remote_ref)? {
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
    push_temp_ref(repo, &mut remote, &temp_ref, &remote_ref)?;

    Ok(UploadResult {
        id: input.id.to_owned(),
        remote_ref,
        commit: snapshot_commit,
    })
}

/// Creates a snapshot commit from the current working directory and pushes it to the selected remote.
pub fn upload_workdir_snapshot(
    repo: &Repository,
    input: SnapshotUpload<'_>,
) -> Result<UploadResult, GitSsError> {
    metadata::validate_id(input.id).map_err(|_| GitSsError::InvalidId(input.id.to_owned()))?;
    let head_id = require_head(repo)?;

    let mut remote = resolve_remote(repo, input.remote)?;
    let remote_ref = format!("refs/heads/gitss/{}", input.id);
    if remote_ref_exists(repo, &mut remote, &remote_ref)? {
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
    push_temp_ref(repo, &mut remote, &temp_ref, &remote_ref)?;

    Ok(UploadResult {
        id: input.id.to_owned(),
        remote_ref,
        commit: snapshot_commit,
    })
}

/// Fetches and returns all `gitss/*` snapshot branches for a remote.
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

/// Fetches one snapshot branch and checks it out as a detached HEAD.
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

/// Returns whether the working tree has changes that should block a safe download.
fn is_worktree_dirty(repo: &Repository) -> Result<bool, GitSsError> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options))?;

    Ok(statuses.iter().any(|entry| !entry.status().is_empty()))
}

/// Checks whether a remote advertises a specific ref.
fn remote_ref_exists(
    repo: &Repository,
    remote: &mut git2::Remote<'_>,
    remote_ref: &str,
) -> Result<bool, GitSsError> {
    let callbacks = remote_callbacks(repo)?;
    let connection = remote.connect_auth(Direction::Push, Some(callbacks), None)?;

    Ok(connection
        .list()?
        .iter()
        .any(|head| head.name() == remote_ref))
}

/// Pushes a local ref to a remote ref and converts push callback failures into errors.
fn push_ref(
    repo: &Repository,
    remote: &mut git2::Remote<'_>,
    source_ref: &str,
    remote_ref: &str,
) -> Result<(), GitSsError> {
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

/// Pushes a temporary snapshot ref and deletes it locally after a successful push attempt.
fn push_temp_ref(
    repo: &Repository,
    remote: &mut git2::Remote<'_>,
    temp_ref: &str,
    remote_ref: &str,
) -> Result<(), GitSsError> {
    let push_result = push_ref(repo, remote, temp_ref, remote_ref);
    let cleanup_result = delete_reference(repo, temp_ref);

    push_result?;
    cleanup_result?;
    Ok(())
}

/// Deletes an existing local reference.
fn delete_reference(repo: &Repository, refname: &str) -> Result<(), GitSsError> {
    let mut reference = repo.find_reference(refname)?;
    reference.delete()?;
    Ok(())
}

/// Deletes a local reference if it exists, ignoring missing refs.
fn delete_reference_if_exists(repo: &Repository, refname: &str) -> Result<(), GitSsError> {
    match repo.find_reference(refname) {
        Ok(mut reference) => reference.delete().map_err(GitSsError::Git),
        Err(err) if err.code() == ErrorCode::NotFound => Ok(()),
        Err(err) => Err(GitSsError::Git(err)),
    }
}

/// Maps checkout conflict-style errors to the user-facing dirty-worktree error.
fn map_checkout_error(err: git2::Error) -> GitSsError {
    match err.code() {
        ErrorCode::Conflict
        | ErrorCode::IndexDirty
        | ErrorCode::Modified
        | ErrorCode::Uncommitted => GitSsError::DirtyWorktree,
        _ => GitSsError::Git(err),
    }
}

/// Maps libgit2 HEAD lookup errors into git-ss repository state errors.
fn map_head_lookup(err: git2::Error) -> GitSsError {
    match err.code() {
        ErrorCode::UnbornBranch | ErrorCode::NotFound => GitSsError::UnbornHead,
        _ => GitSsError::Git(err),
    }
}

/// Builds the signature used for synthetic snapshot commits.
fn snapshot_signature(repo: &Repository) -> Result<Signature<'_>, GitSsError> {
    match repo.signature() {
        Ok(signature) => Ok(signature),
        Err(_) => Signature::now("git-ss", "git-ss@example.invalid").map_err(GitSsError::Git),
    }
}

/// Creates libgit2 remote callbacks for SSH and HTTPS credential lookup.
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

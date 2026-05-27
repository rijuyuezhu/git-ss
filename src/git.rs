use std::path::Path;

use chrono::Local;
use git2::{
    Cred, CredentialType, Direction, ErrorCode, PushOptions, RemoteCallbacks, Repository, Signature,
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

    push_ref(repo, input.remote, &temp_ref, &remote_ref)?;

    Ok(UploadResult {
        id: input.id.to_owned(),
        remote_ref,
        commit: snapshot_commit,
    })
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

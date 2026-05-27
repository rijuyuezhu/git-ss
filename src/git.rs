use std::path::Path;

use git2::{ErrorCode, Repository};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitSsError {
    #[error("not inside a Git repository")]
    NotRepository,
    #[error("repository has no HEAD; git-ss does not support empty repositories yet")]
    UnbornHead,
    #[error("remote '{0}' is not configured; pass --remote <name> to select another remote")]
    MissingRemote(String),
    #[error(transparent)]
    Git(#[from] git2::Error),
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

fn map_head_lookup(err: git2::Error) -> GitSsError {
    match err.code() {
        ErrorCode::UnbornBranch | ErrorCode::NotFound => GitSsError::UnbornHead,
        _ => GitSsError::Git(err),
    }
}

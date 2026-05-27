use std::path::Path;

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
    Discover(Box<gix::discover::Error>),
    #[error(transparent)]
    FindHead(Box<gix::reference::find::existing::Error>),
    #[error(transparent)]
    PeelHead(Box<gix::head::peel::Error>),
    #[error(transparent)]
    FindRemote(Box<gix::remote::find::existing::Error>),
}

pub fn discover_repo(start: &Path) -> Result<gix::Repository, GitSsError> {
    gix::discover(start).map_err(|err| match err {
        gix::discover::Error::Discover(
            gix::discover::upwards::Error::NoGitRepository { .. }
            | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
            | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. },
        ) => GitSsError::NotRepository,
        err => GitSsError::Discover(Box::new(err)),
    })
}

pub fn require_head(repo: &gix::Repository) -> Result<gix::ObjectId, GitSsError> {
    let head_id = repo
        .head()
        .map_err(|err| GitSsError::FindHead(Box::new(err)))?
        .try_into_peeled_id()
        .map_err(|err| GitSsError::PeelHead(Box::new(err)))?;
    head_id.map(gix::Id::detach).ok_or(GitSsError::UnbornHead)
}

pub fn resolve_remote<'repo>(
    repo: &'repo gix::Repository,
    remote_name: &str,
) -> Result<gix::Remote<'repo>, GitSsError> {
    repo.find_remote(remote_name).map_err(|err| match err {
        gix::remote::find::existing::Error::NotFound { .. } => {
            GitSsError::MissingRemote(remote_name.to_owned())
        }
        err => GitSsError::FindRemote(Box::new(err)),
    })
}

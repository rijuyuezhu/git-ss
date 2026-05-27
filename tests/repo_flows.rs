use git_ss::git::{discover_repo, require_head, resolve_remote};

#[test]
fn discover_repo_fails_outside_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let err = expect_err(discover_repo(temp.path()), "not a repo");
    assert!(err.to_string().contains("not inside a Git repository"));
}

#[test]
fn discover_repo_fails_for_missing_path_as_not_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");

    let err = expect_err(discover_repo(&missing), "missing path");

    assert!(err.to_string().contains("not inside a Git repository"));
}

#[test]
fn resolving_missing_origin_reports_remote_hint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    let err = expect_err(resolve_remote(&repo, "origin"), "origin missing");
    assert!(
        err.to_string()
            .contains("remote 'origin' is not configured")
    );
    assert!(err.to_string().contains("--remote <name>"));
}

#[test]
fn require_head_reports_empty_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");

    let err = expect_err(require_head(&repo), "head should be unborn");

    assert!(
        err.to_string()
            .contains("repository has no HEAD; git-ss does not support empty repositories yet")
    );
}

#[test]
fn require_head_preserves_missing_head_target_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = git2::Repository::init(temp.path()).expect("repo init");
    let heads = repo.path().join("refs/heads");
    std::fs::create_dir_all(&heads).expect("heads dir");
    std::fs::write(
        heads.join("main"),
        "1111111111111111111111111111111111111111\n",
    )
    .expect("bad ref");
    repo.set_head("refs/heads/main").expect("set head");

    let err = expect_err(require_head(&repo), "head target is missing");

    assert!(
        !err.to_string()
            .contains("repository has no HEAD; git-ss does not support empty repositories yet")
    );
}

fn expect_err<T, E>(result: Result<T, E>, message: &str) -> E {
    match result {
        Ok(_) => panic!("{message}"),
        Err(err) => err,
    }
}

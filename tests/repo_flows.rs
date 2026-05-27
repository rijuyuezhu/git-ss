use git_ss::git::{discover_repo, resolve_remote};

#[test]
fn discover_repo_fails_outside_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let err = discover_repo(temp.path()).expect_err("not a repo");
    assert!(err.to_string().contains("not inside a Git repository"));
}

#[test]
fn discover_repo_preserves_inaccessible_path_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = temp.path().join("missing");

    let err = discover_repo(&missing).expect_err("inaccessible path");

    assert!(!err.to_string().contains("not inside a Git repository"));
    assert!(err.to_string().contains("Failed to access"));
}

#[test]
fn resolving_missing_origin_reports_remote_hint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repo = gix::init(temp.path()).expect("repo init");
    let err = resolve_remote(&repo, "origin").expect_err("origin missing");
    assert!(
        err.to_string()
            .contains("remote 'origin' is not configured")
    );
    assert!(err.to_string().contains("--remote <name>"));
}

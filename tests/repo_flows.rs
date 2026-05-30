use std::path::{Path, PathBuf};

use assert_cmd::Command;
use git_ss::git::{discover_repo, require_head, resolve_remote};
use predicates::prelude::*;

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

#[test]
fn upload_ref_creates_snapshot_branch_in_local_bare_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_path = temp.path().join("remote.git");
    git2::Repository::init_bare(&remote_path).expect("bare remote init");

    let work_path = temp.path().join("work");
    let repo = git2::Repository::init(&work_path).expect("work repo init");
    create_initial_commit(&repo, &work_path);

    let remote_path_str = remote_path.to_str().expect("remote path is utf-8");
    repo.remote("origin", remote_path_str)
        .expect("origin remote");
    let source_commit = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("source commit");

    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.current_dir(&work_path)
        .args(["upload", "--id", "ref-demo", "HEAD"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ref-demo"));

    let remote_repo = git2::Repository::open_bare(&remote_path).expect("open bare remote");
    let snapshot_ref = remote_repo
        .find_reference("refs/heads/gitss/ref-demo")
        .expect("snapshot branch exists");
    let snapshot_commit = snapshot_ref.peel_to_commit().expect("snapshot commit");
    let message = snapshot_commit.message().expect("snapshot message");

    assert_eq!(snapshot_commit.tree_id(), source_commit.tree_id());
    assert_eq!(snapshot_commit.parent_count(), 1);
    assert_eq!(
        snapshot_commit.parent_id(0).expect("parent id"),
        source_commit.id()
    );
    assert!(message.contains("Git-SS-Id: ref-demo"));
    assert!(message.contains("Git-SS-Type: ref"));
    assert!(message.contains("Git-SS-Source: HEAD"));
    assert!(repo.find_reference("refs/gitss/tmp/ref-demo").is_err());
}

#[test]
fn upload_ref_rejects_existing_snapshot_branch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_path = temp.path().join("remote.git");
    git2::Repository::init_bare(&remote_path).expect("bare remote init");

    let work_path = temp.path().join("work");
    let repo = git2::Repository::init(&work_path).expect("work repo init");
    create_initial_commit(&repo, &work_path);

    let remote_path_str = remote_path.to_str().expect("remote path is utf-8");
    repo.remote("origin", remote_path_str)
        .expect("origin remote");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "duplicate", "HEAD"])
        .assert()
        .success();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "duplicate", "HEAD"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "remote snapshot branch 'refs/heads/gitss/duplicate' already exists",
        ));
}

#[test]
fn list_displays_snapshot_metadata() {
    let (_temp, work_path, _remote_path, repo) = create_repo_with_bare_origin();
    let source_commit = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("source commit")
        .id()
        .to_string();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "list-demo", "HEAD"])
        .assert()
        .success();

    let output = Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .output()
        .expect("run list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("list-demo"));
    assert!(stdout.contains("ref"));
    assert!(stdout.contains("HEAD"));
    assert!(stdout.contains(&source_commit[..7]));
    assert!(stdout.contains("initial commit"));
    assert!(stdout.contains("no changes"));
}

#[test]
fn list_summarizes_workdir_changes() {
    let (_temp, work_path, _remote_path, _repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join("scratch.txt"), "scratch\n").expect("write scratch file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "workdir-list", "workdir"])
        .assert()
        .success();

    let output = Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .output()
        .expect("run list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("workdir-list"));
    assert!(stdout.contains("workdir"));
    assert!(stdout.contains("initial commit"));
    assert!(stdout.contains("1 file, +1 -0"));
}

#[test]
fn list_displays_include_ignored_flag() {
    let (_temp, work_path, _remote_path, repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join(".gitignore"), "ignored.txt\n").expect("write gitignore");
    commit_paths(&repo, &[Path::new(".gitignore")], "ignore ignored.txt");
    std::fs::write(work_path.join("ignored.txt"), "ignored\n").expect("write ignored file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args([
            "upload",
            "--id",
            "ignored-list",
            "--include-ignored",
            "workdir",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .output()
        .expect("run list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("IGNORED"));
    assert!(stdout.contains("ignored-list"));
    assert!(stdout.contains("yes"));
}

#[test]
fn list_wraps_long_base_commit_summaries() {
    let (_temp, work_path, _remote_path, repo) = create_repo_with_bare_origin();
    let long_message = "implement a very long base commit title with many words that should wrap inside the base column";
    std::fs::write(work_path.join("long.txt"), "long\n").expect("write long file");
    commit_paths(&repo, &[Path::new("long.txt")], long_message);

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "long-base", "HEAD"])
        .assert()
        .success();

    let output = Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .output()
        .expect("run list");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("long-base"));
    assert!(stdout.contains("implement a very long base"));
    assert!(!stdout.contains(long_message));
}

#[test]
fn list_prunes_deleted_remote_snapshots() {
    let (_temp, work_path, remote_path, _repo) = create_repo_with_bare_origin();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "prune-demo", "HEAD"])
        .assert()
        .success();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("prune-demo"));

    let remote_repo = git2::Repository::open_bare(&remote_path).expect("open bare remote");
    remote_repo
        .find_reference("refs/heads/gitss/prune-demo")
        .expect("remote snapshot ref")
        .delete()
        .expect("delete remote snapshot ref");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("prune-demo").not());
}

#[test]
fn list_warns_for_malformed_snapshot_metadata() {
    let (_temp, work_path, remote_path, repo) = create_repo_with_bare_origin();
    let source_commit = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("source commit");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "seed", "HEAD"])
        .assert()
        .success();

    let remote_repo = git2::Repository::open_bare(&remote_path).expect("open bare remote");
    remote_repo
        .reference(
            "refs/heads/gitss/malformed",
            source_commit.id(),
            true,
            "malformed snapshot",
        )
        .expect("malformed ref");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("malformed"))
        .stdout(predicate::str::contains("WARN"));
}

#[test]
fn download_refuses_dirty_worktree_without_force() {
    let (_temp, work_path, _remote_path, _repo) = create_repo_with_bare_origin();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "download-dirty", "HEAD"])
        .assert()
        .success();
    std::fs::write(work_path.join("file.txt"), "local changes\n").expect("dirty file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["download", "download-dirty"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("working tree has local changes"));
}

#[test]
fn download_force_checks_out_snapshot_detached() {
    let (_temp, work_path, _remote_path, _repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join("server.txt"), "from server\n").expect("write server file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "download-force", "workdir"])
        .assert()
        .success();
    std::fs::remove_file(work_path.join("server.txt")).expect("remove server file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["download", "--force", "download-force"])
        .assert()
        .success();

    let restored = std::fs::read_to_string(work_path.join("server.txt")).expect("server file");
    assert_eq!(restored, "from server\n");
    let repo = git2::Repository::open(&work_path).expect("repo open");
    assert!(!repo.head().expect("head").is_branch());
}

#[test]
fn download_rejects_deleted_remote_snapshot_with_stale_tracking_ref() {
    let (_temp, work_path, remote_path, _repo) = create_repo_with_bare_origin();

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "stale-download", "HEAD"])
        .assert()
        .success();
    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("stale-download"));

    let remote_repo = git2::Repository::open_bare(&remote_path).expect("open bare remote");
    remote_repo
        .find_reference("refs/heads/gitss/stale-download")
        .expect("remote snapshot ref")
        .delete()
        .expect("delete remote snapshot ref");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["download", "stale-download"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "snapshot 'stale-download' was not found on the selected remote",
        ));
}

#[test]
fn download_refuses_to_overwrite_ignored_file_without_force() {
    let (_temp, work_path, _remote_path, repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join(".gitignore"), "ignored.txt\n").expect("write gitignore");
    commit_paths(&repo, &[Path::new(".gitignore")], "ignore ignored.txt");
    std::fs::write(work_path.join("ignored.txt"), "snapshot version\n")
        .expect("write ignored snapshot file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args([
            "upload",
            "--id",
            "ignored-overwrite",
            "--include-ignored",
            "workdir",
        ])
        .assert()
        .success();
    std::fs::write(work_path.join("ignored.txt"), "local ignored version\n")
        .expect("write local ignored file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["download", "ignored-overwrite"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("working tree has local changes"));

    let contents = std::fs::read_to_string(work_path.join("ignored.txt")).expect("ignored file");
    assert_eq!(contents, "local ignored version\n");
}

#[test]
fn upload_workdir_includes_untracked_nonignored_files() {
    let (_temp, work_path, remote_path, _repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join("scratch.txt"), "scratch\n").expect("write scratch file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "workdir-demo", "workdir"])
        .assert()
        .success()
        .stdout(predicate::eq("workdir-demo\n"));

    with_remote_snapshot_commit(&remote_path, "workdir-demo", |snapshot_commit| {
        let snapshot_tree = snapshot_commit.tree().expect("snapshot tree");
        let message = snapshot_commit.message().expect("snapshot message");

        assert!(snapshot_tree.get_name("scratch.txt").is_some());
        assert!(message.contains("Git-SS-Type: workdir"));
        assert!(message.contains("Git-SS-Include-Ignored: false"));
    });
    assert!(_repo.find_reference("refs/gitss/tmp/workdir-demo").is_err());
}

#[test]
fn upload_workdir_excludes_ignored_files_by_default() {
    let (_temp, work_path, remote_path, repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join(".gitignore"), "ignored.txt\n").expect("write gitignore");
    commit_paths(&repo, &[Path::new(".gitignore")], "ignore ignored.txt");
    std::fs::write(work_path.join("ignored.txt"), "ignored\n").expect("write ignored file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "ignored-default", "workdir"])
        .assert()
        .success()
        .stdout(predicate::eq("ignored-default\n"));

    with_remote_snapshot_commit(&remote_path, "ignored-default", |snapshot_commit| {
        let snapshot_tree = snapshot_commit.tree().expect("snapshot tree");

        assert!(snapshot_tree.get_name("ignored.txt").is_none());
    });
}

#[test]
fn upload_workdir_include_ignored_includes_ignored_files() {
    let (_temp, work_path, remote_path, repo) = create_repo_with_bare_origin();
    std::fs::write(work_path.join(".gitignore"), "ignored.txt\n").expect("write gitignore");
    commit_paths(&repo, &[Path::new(".gitignore")], "ignore ignored.txt");
    std::fs::write(work_path.join("ignored.txt"), "ignored\n").expect("write ignored file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args([
            "upload",
            "--id",
            "ignored-included",
            "--include-ignored",
            "workdir",
        ])
        .assert()
        .success()
        .stdout(predicate::eq("ignored-included\n"));

    with_remote_snapshot_commit(&remote_path, "ignored-included", |snapshot_commit| {
        let snapshot_tree = snapshot_commit.tree().expect("snapshot tree");
        let message = snapshot_commit.message().expect("snapshot message");

        assert!(snapshot_tree.get_name("ignored.txt").is_some());
        assert!(message.contains("Git-SS-Include-Ignored: true"));
    });
}

#[test]
fn upload_workdir_reflects_tracked_deletions() {
    let (_temp, work_path, remote_path, _repo) = create_repo_with_bare_origin();
    std::fs::remove_file(work_path.join("file.txt")).expect("remove tracked file");

    Command::cargo_bin("git-ss")
        .expect("binary exists")
        .current_dir(&work_path)
        .args(["upload", "--id", "deleted-tracked", "workdir"])
        .assert()
        .success()
        .stdout(predicate::eq("deleted-tracked\n"));

    with_remote_snapshot_commit(&remote_path, "deleted-tracked", |snapshot_commit| {
        let snapshot_tree = snapshot_commit.tree().expect("snapshot tree");

        assert!(snapshot_tree.get_name("file.txt").is_none());
    });
}

fn expect_err<T, E>(result: Result<T, E>, message: &str) -> E {
    match result {
        Ok(_) => panic!("{message}"),
        Err(err) => err,
    }
}

fn create_initial_commit(repo: &git2::Repository, work_path: &Path) -> git2::Oid {
    std::fs::write(work_path.join("file.txt"), "hello\n").expect("write fixture file");

    let mut index = repo.index().expect("repo index");
    index.add_path(Path::new("file.txt")).expect("add file");
    index.write().expect("persist index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    let signature = git2::Signature::now("Test User", "test@example.com").expect("signature");

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "initial commit",
        &tree,
        &[],
    )
    .expect("initial commit")
}

fn create_repo_with_bare_origin() -> (tempfile::TempDir, PathBuf, PathBuf, git2::Repository) {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_path = temp.path().join("remote.git");
    git2::Repository::init_bare(&remote_path).expect("bare remote init");

    let work_path = temp.path().join("work");
    let repo = git2::Repository::init(&work_path).expect("work repo init");
    create_initial_commit(&repo, &work_path);

    let remote_path_str = remote_path.to_str().expect("remote path is utf-8");
    repo.remote("origin", remote_path_str)
        .expect("origin remote");

    (temp, work_path, remote_path, repo)
}

fn commit_paths(repo: &git2::Repository, paths: &[&Path], message: &str) -> git2::Oid {
    let mut index = repo.index().expect("repo index");
    for path in paths {
        index.add_path(path).expect("add path");
    }
    index.write().expect("persist index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");
    let signature = git2::Signature::now("Test User", "test@example.com").expect("signature");
    let parent = repo
        .head()
        .expect("head")
        .peel_to_commit()
        .expect("parent commit");

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &[&parent],
    )
    .expect("commit paths")
}

fn with_remote_snapshot_commit<T>(
    remote_path: &Path,
    id: &str,
    assert_commit: impl FnOnce(&git2::Commit<'_>) -> T,
) -> T {
    let remote_repo = git2::Repository::open_bare(remote_path).expect("open bare remote");
    let snapshot_ref = remote_repo
        .find_reference(&format!("refs/heads/gitss/{id}"))
        .expect("snapshot branch exists");
    let commit = snapshot_ref.peel_to_commit().expect("snapshot commit");

    assert_commit(&commit)
}

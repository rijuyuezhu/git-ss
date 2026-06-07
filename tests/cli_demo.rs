use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_mentions_core_commands() {
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("upload"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("download"))
        .stdout(predicate::str::contains("clean"));
}

#[test]
fn upload_workdir_requires_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.current_dir(temp.path())
        .args(["upload", "workdir"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not inside a Git repository"));
}

#[test]
fn include_ignored_is_rejected_for_ref_uploads() {
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.args(["upload", "--include-ignored", "HEAD"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--include-ignored can only be used with upload workdir",
        ));
}

#[test]
fn list_requires_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.current_dir(temp.path())
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not inside a Git repository"));
}

#[test]
fn download_requires_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.current_dir(temp.path())
        .args(["download", "20260528-153012"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not inside a Git repository"));
}

#[test]
fn clean_requires_git_repository() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.current_dir(temp.path())
        .arg("clean")
        .assert()
        .failure()
        .stderr(predicate::str::contains("not inside a Git repository"));
}

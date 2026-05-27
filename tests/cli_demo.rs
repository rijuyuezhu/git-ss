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
        .stdout(predicate::str::contains("download"));
}

#[test]
fn upload_workdir_demo_returns_not_implemented() {
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.args(["upload", "workdir"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("upload is not implemented yet"));
}

#[test]
fn list_demo_returns_not_implemented() {
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("list is not implemented yet"));
}

#[test]
fn download_demo_returns_not_implemented() {
    let mut cmd = Command::cargo_bin("git-ss").expect("binary exists");
    cmd.args(["download", "20260528-153012"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("download is not implemented yet"));
}

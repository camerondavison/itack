//! End-to-end CLI tests.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn itack() -> Command {
    Command::cargo_bin("itack").unwrap()
}

fn setup_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to init git repo");

    // Configure git user for the repo
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to configure git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("Failed to configure git name");

    dir
}

#[test]
fn test_init_creates_itack_directory() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized itack project"));

    assert!(dir.path().join(".itack").exists());
    assert!(dir.path().join(".itack/metadata.toml").exists());
}

#[test]
fn test_init_fails_without_git() {
    let dir = TempDir::new().unwrap();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn test_init_repairs_if_already_initialized() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Running init again should repair/succeed (not fail)
    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired database"));
}

#[test]
fn test_create_and_show_issue() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue", "--epic", "MVP"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));

    // Verify file was created
    assert!(dir.path().join(".itack/1.md").exists());

    // Show the issue
    itack()
        .args(["show", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test issue"))
        .stdout(predicate::str::contains("MVP"));

    // Show as JSON
    itack()
        .args(["show", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\": \"Test issue\""));
}

#[test]
fn test_show_nonexistent_issue() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["show", "999"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue 999 not found"));
}

#[test]
fn test_list_issues() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "First issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Second issue", "--epic", "MVP"])
        .current_dir(dir.path())
        .assert()
        .success();

    // List all
    itack()
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("First issue"))
        .stdout(predicate::str::contains("Second issue"));

    // List with epic filter
    itack()
        .args(["list", "--epic", "MVP"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Second issue"))
        .stdout(predicate::str::contains("First issue").not());

    // List as JSON
    itack()
        .args(["list", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": 1"))
        .stdout(predicate::str::contains("\"id\": 2"));
}

#[test]
fn test_status_update() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Update status
    itack()
        .args(["status", "1", "in-progress"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("open -> in-progress"));

    // Verify in list
    itack()
        .args(["list", "--status", "in-progress"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test issue"));

    // Update to done
    itack()
        .args(["status", "1", "done"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn test_claim_and_release() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Claim the issue
    itack()
        .args(["claim", "1", "agent-1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Claimed issue #1 for agent-1"));

    // Verify status changed to in-progress
    itack()
        .args(["show", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"in-progress\""))
        .stdout(predicate::str::contains("\"assignee\": \"agent-1\""));

    // Release the claim
    itack()
        .args(["release", "1"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Released issue #1"));

    // Verify assignee is removed
    itack()
        .args(["show", "1", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"assignee\": null"));
}

#[test]
fn test_claim_conflict_returns_exit_code_2() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    // First claim succeeds
    itack()
        .args(["claim", "1", "agent-1"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Second claim fails with exit code 2
    itack()
        .args(["claim", "1", "agent-2"])
        .current_dir(dir.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already claimed by agent-1"));
}

#[test]
fn test_release_unclaimed_issue() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Release unclaimed issue fails
    itack()
        .args(["release", "1"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not claimed"));
}

#[test]
fn test_board_command() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Issue 1"])
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Issue 2"])
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["status", "1", "done"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Board shows summary
    itack()
        .arg("board")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Open"))
        .stdout(predicate::str::contains("Done"));

    // Board as JSON
    itack()
        .args(["board", "--json"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"open\": 1"))
        .stdout(predicate::str::contains("\"done\": 1"));
}

#[test]
fn test_invalid_status() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue"])
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["status", "1", "invalid-status"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn test_issue_ids_increment() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "First"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#1"));

    itack()
        .args(["create", "Second"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#2"));

    itack()
        .args(["create", "Third"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#3"));
}

#[test]
fn test_markdown_file_format() {
    let dir = setup_git_repo();

    itack()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    itack()
        .args(["create", "Test issue", "--epic", "MVP"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = fs::read_to_string(dir.path().join(".itack/1.md")).unwrap();

    // Check YAML front matter format
    assert!(content.starts_with("---\n"));
    assert!(content.contains("id: 1"));
    assert!(content.contains("title: Test issue"));
    assert!(content.contains("epic: MVP"));
    assert!(content.contains("status: open"));
}

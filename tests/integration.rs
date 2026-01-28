//! End-to-end CLI tests.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Read an issue file from the data/itack branch.
/// Returns the content of the issue file.
fn read_issue_from_data_branch(repo_path: &Path, id: u32) -> Option<String> {
    use std::process::Command;

    let suffix = format!("-issue-{:03}.md", id);

    // List files in .itack/ on data/itack branch
    let output = Command::new("git")
        .args(["ls-tree", "--name-only", "data/itack", ".itack/"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let files = String::from_utf8_lossy(&output.stdout);
    let matching_file = files.lines().find(|f| f.ends_with(&suffix))?;

    // Read the file content from the branch
    let output = Command::new("git")
        .args(["show", &format!("data/itack:{}", matching_file)])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Test environment with isolated git repo and database directory.
struct TestEnv {
    /// Temporary git repository.
    repo: TempDir,
    /// Temporary directory for ITACK_HOME (database storage).
    itack_home: TempDir,
}

impl TestEnv {
    fn path(&self) -> &Path {
        self.repo.path()
    }

    fn itack_home_str(&self) -> &str {
        self.itack_home.path().to_str().unwrap()
    }
}

fn itack(env: &TestEnv) -> Command {
    let mut cmd = Command::cargo_bin("itack").unwrap();
    cmd.env("ITACK_HOME", env.itack_home_str());
    cmd
}

fn setup_git_repo() -> TestEnv {
    let repo = TempDir::new().unwrap();
    let itack_home = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to init git repo");

    // Configure git user for the repo
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to configure git email");

    std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo.path())
        .output()
        .expect("Failed to configure git name");

    TestEnv { repo, itack_home }
}

#[test]
fn test_init_creates_itack_directory() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Initialized itack project"));

    assert!(env.path().join(".itack").exists());
    assert!(env.path().join(".itack/metadata.toml").exists());
}

#[test]
fn test_init_fails_without_git() {
    let dir = TempDir::new().unwrap();
    let itack_home = TempDir::new().unwrap();

    Command::cargo_bin("itack")
        .unwrap()
        .env("ITACK_HOME", itack_home.path())
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Not in a git repository"));
}

#[test]
fn test_init_repairs_if_already_initialized() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    // Running init again should repair/succeed (not fail)
    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Repaired database"));
}

#[test]
fn test_create_and_show_issue() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue", "--epic", "MVP"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Created issue #1"));

    // Verify file was created in data branch
    let content = read_issue_from_data_branch(env.path(), 1);
    assert!(content.is_some(), "Issue file should exist in data branch");

    // Show the issue
    itack(&env)
        .args(["show", "1"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test issue"))
        .stdout(predicate::str::contains("MVP"));

    // Show as JSON
    itack(&env)
        .args(["show", "1", "--json"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\": \"Test issue\""));
}

#[test]
fn test_show_nonexistent_issue() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["show", "999"])
        .current_dir(env.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue 999 not found"));
}

#[test]
fn test_list_issues() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "First issue"])
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Second issue", "--epic", "MVP"])
        .current_dir(env.path())
        .assert()
        .success();

    // List all
    itack(&env)
        .arg("list")
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("First issue"))
        .stdout(predicate::str::contains("Second issue"));

    // List with epic filter
    itack(&env)
        .args(["list", "--epic", "MVP"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Second issue"))
        .stdout(predicate::str::contains("First issue").not());

    // List as JSON
    itack(&env)
        .args(["list", "--json"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"id\": 1"))
        .stdout(predicate::str::contains("\"id\": 2"));
}

#[test]
fn test_done_command() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue"])
        .current_dir(env.path())
        .assert()
        .success();

    // Mark as done
    itack(&env)
        .args(["done", "1"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("open -> done"));

    // Verify in list
    itack(&env)
        .args(["list", "--status", "done"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Test issue"));
}

#[test]
fn test_claim_and_release() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue"])
        .current_dir(env.path())
        .assert()
        .success();

    // Claim the issue
    itack(&env)
        .args(["claim", "1", "agent-1"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Claimed issue #1 for agent-1"));

    // Verify status changed to in-progress
    itack(&env)
        .args(["show", "1", "--json"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"in-progress\""))
        .stdout(predicate::str::contains("\"assignee\": \"agent-1\""));

    // Release the claim
    itack(&env)
        .args(["release", "1"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Released issue #1"));

    // Verify assignee is removed
    itack(&env)
        .args(["show", "1", "--json"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"assignee\": null"));
}

#[test]
fn test_claim_conflict_returns_exit_code_2() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue"])
        .current_dir(env.path())
        .assert()
        .success();

    // First claim succeeds
    itack(&env)
        .args(["claim", "1", "agent-1"])
        .current_dir(env.path())
        .assert()
        .success();

    // Second claim fails with exit code 2
    itack(&env)
        .args(["claim", "1", "agent-2"])
        .current_dir(env.path())
        .assert()
        .code(2)
        .stderr(predicate::str::contains("already claimed by agent-1"));
}

#[test]
fn test_release_unclaimed_issue() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue"])
        .current_dir(env.path())
        .assert()
        .success();

    // Release unclaimed issue fails
    itack(&env)
        .args(["release", "1"])
        .current_dir(env.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not claimed"));
}

#[test]
fn test_board_command() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Issue 1"])
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Issue 2"])
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["done", "1"])
        .current_dir(env.path())
        .assert()
        .success();

    // Board shows summary
    itack(&env)
        .arg("board")
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Open"))
        .stdout(predicate::str::contains("Done"));

    // Board as JSON
    itack(&env)
        .args(["board", "--json"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"open\": 1"))
        .stdout(predicate::str::contains("\"done\": 1"));
}

#[test]
fn test_done_nonexistent_issue() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["done", "999"])
        .current_dir(env.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Issue 999 not found"));
}

#[test]
fn test_issue_ids_increment() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "First"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#1"));

    itack(&env)
        .args(["create", "Second"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#2"));

    itack(&env)
        .args(["create", "Third"])
        .current_dir(env.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("#3"));
}

#[test]
fn test_markdown_file_format() {
    let env = setup_git_repo();

    itack(&env)
        .arg("init")
        .current_dir(env.path())
        .assert()
        .success();

    itack(&env)
        .args(["create", "Test issue", "--epic", "MVP"])
        .current_dir(env.path())
        .assert()
        .success();

    // Read issue from data branch
    let content =
        read_issue_from_data_branch(env.path(), 1).expect("Issue file should exist in data branch");

    // Check YAML front matter format (title is NOT in YAML, it's in markdown body)
    assert!(content.starts_with("---\n"));
    assert!(content.contains("id: 1"));
    assert!(
        !content.contains("title:"),
        "Title should not be in YAML front matter"
    );
    assert!(content.contains("epic: MVP"));
    assert!(content.contains("status: open"));
    // Title should be in markdown body as H1 heading
    assert!(content.contains("# Test issue"));
}

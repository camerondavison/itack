//! itack claim command.

use crate::core::{Project, Status, cleanup_working_file, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::write_issue;

/// Arguments for the claim command.
pub struct ClaimArgs {
    pub id: u32,
    pub assignee: String,
    pub session: Option<String>,
}

/// Claim an issue with SQLite-backed locking.
pub fn run(args: ClaimArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut db = project.open_db()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load the issue from data branch (source of truth) and sync to working directory
    let mut issue_info =
        load_issue_from_data_branch(&project.repo_root, &project.itack_dir, data_branch, args.id)?;

    // Try to claim in database (atomic operation)
    db.claim(args.id, &args.assignee)?;

    // Update issue fields
    issue_info.issue.assignee = Some(args.assignee.clone());
    issue_info.issue.branch = project.current_branch();
    issue_info.issue.session = args.session.clone();
    if issue_info.issue.status == Status::Open {
        issue_info.issue.status = Status::InProgress;
    }

    // Get relative path for commit
    let relative_path = issue_info
        .path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&issue_info.path)
        .to_path_buf();

    // Write to working directory
    write_issue(
        &issue_info.path,
        &issue_info.issue,
        &issue_info.title,
        &issue_info.body,
    )?;

    // Read back the content for data branch commit
    let content = std::fs::read(&issue_info.path)?;

    // Commit to data branch only (feature branches get updated on 'done')
    let message = format!("Claim issue #{} for {}", args.id, args.assignee);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &message,
    )?;

    // Restore file to HEAD state if it exists on this branch, otherwise delete
    cleanup_working_file(&project.repo_root, &relative_path)?;

    println!("Claimed issue #{} for {}", args.id, args.assignee);

    Ok(())
}

//! itack claim command.

use crate::core::{Project, Status, cherry_pick_to_head, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue;
use crate::storage::format_issue;

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

    // Load the issue from working directory
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

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

    // Format content
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;

    // Commit to data branch
    let message = format!("Claim issue #{} for {}", args.id, args.assignee);
    let commit_oid = commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &message,
    )?;

    // Cherry-pick onto current branch (updates working dir, index, and HEAD)
    if let Some(oid) = commit_oid {
        cherry_pick_to_head(&project.repo_root, oid, &message)?;
    }

    println!("Claimed issue #{} for {}", args.id, args.assignee);

    Ok(())
}

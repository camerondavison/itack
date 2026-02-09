//! itack claim command.

use crate::core::{Project, Status, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::markdown::format_issue;

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

    // Load the issue from data branch (source of truth)
    let mut issue_info = load_issue_from_data_branch(&project.repo_root, data_branch, args.id)?;

    // Try to claim in database (atomic operation)
    db.claim(args.id, &args.assignee)?;

    // Update issue fields
    issue_info.issue.assignee = Some(args.assignee.clone());
    issue_info.issue.branch = project.current_branch();
    issue_info.issue.session = args.session.clone();
    if issue_info.issue.status == Status::Open {
        issue_info.issue.status = Status::InProgress;
    }

    // Format issue content in memory and commit directly to data branch
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;
    let message = format!("Claim issue #{} for {}", args.id, args.assignee);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &issue_info.relative_path,
        content.as_bytes(),
        &message,
    )?;

    println!("Claimed issue #{} for {}", args.id, args.assignee);

    Ok(())
}

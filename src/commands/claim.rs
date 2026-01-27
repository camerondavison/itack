//! itack claim command.

use crate::core::{Project, Status};
use crate::error::Result;
use crate::storage::db::load_issue;
use crate::storage::write_issue;

/// Arguments for the claim command.
pub struct ClaimArgs {
    pub id: u32,
    pub assignee: String,
}

/// Claim an issue with SQLite-backed locking.
pub fn run(args: ClaimArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut db = project.open_db()?;

    // Load the issue first to verify it exists
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    // Try to claim in database (atomic operation)
    db.claim(args.id, &args.assignee)?;

    // Update markdown file
    issue_info.issue.assignee = Some(args.assignee.clone());
    issue_info.issue.branch = project.current_branch();
    if issue_info.issue.status == Status::Open {
        issue_info.issue.status = Status::InProgress;
    }

    write_issue(
        &issue_info.path,
        &issue_info.issue,
        &issue_info.title,
        &issue_info.body,
    )?;

    println!("Claimed issue #{} for {}", args.id, args.assignee);

    Ok(())
}

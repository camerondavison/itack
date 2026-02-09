//! itack release command.

use crate::core::{Project, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::markdown::format_issue;

/// Arguments for the release command.
pub struct ReleaseArgs {
    pub id: u32,
}

/// Release a claim on an issue.
pub fn run(args: ReleaseArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut db = project.open_db()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load the issue from data branch (source of truth)
    let mut issue_info = load_issue_from_data_branch(&project.repo_root, data_branch, args.id)?;

    // Release in database
    db.release(args.id)?;

    // Update issue fields
    let old_assignee = issue_info.issue.assignee.take();
    issue_info.issue.branch = None;
    issue_info.issue.session = None;

    // Format issue content in memory and commit directly to data branch
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;
    let message = format!("Release issue #{}", args.id);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &issue_info.relative_path,
        content.as_bytes(),
        &message,
    )?;

    if let Some(assignee) = old_assignee {
        println!("Released issue #{} from {}", args.id, assignee);
    } else {
        println!("Released issue #{}", args.id);
    }

    Ok(())
}

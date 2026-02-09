//! itack done command.

use crate::core::{Project, Status, commit_to_branch};
use crate::error::{ItackError, Result};
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::markdown::format_issue;

/// Arguments for the done command.
pub struct DoneArgs {
    pub id: u32,
}

/// Mark an issue as done.
pub fn run(args: DoneArgs) -> Result<()> {
    let project = Project::discover()?;
    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load issue from data branch (source of truth)
    let mut issue_info = load_issue_from_data_branch(&project.repo_root, data_branch, args.id)?;

    if issue_info.issue.status == Status::Done {
        return Err(ItackError::AlreadyDone(args.id));
    }

    let old_status = issue_info.issue.status;
    issue_info.issue.status = Status::Done;

    // Format issue content in memory and commit directly to data branch
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;
    let message = format!("Mark issue #{} as done", args.id);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &issue_info.relative_path,
        content.as_bytes(),
        &message,
    )?;

    println!(
        "Updated issue #{} status: {} -> {}",
        args.id,
        old_status,
        Status::Done
    );

    Ok(())
}

//! itack done command.

use crate::core::{Project, Status, cherry_pick_to_head, commit_to_branch};
use crate::error::{ItackError, Result};
use crate::storage::db::load_issue;
use crate::storage::format_issue;

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

    // Load issue from working directory
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    if issue_info.issue.status == Status::Done {
        return Err(ItackError::AlreadyDone(args.id));
    }

    let old_status = issue_info.issue.status;
    issue_info.issue.status = Status::Done;

    // Get relative path for commit
    let relative_path = issue_info
        .path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&issue_info.path)
        .to_path_buf();

    // Format content
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;

    // Commit to data branch
    let message = format!("Mark issue #{} as done", args.id);
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

    println!(
        "Updated issue #{} status: {} -> {}",
        args.id,
        old_status,
        Status::Done
    );

    Ok(())
}

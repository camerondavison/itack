//! itack set-session command.

use crate::core::{Project, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::markdown::format_issue;

/// Arguments for the set-session command.
pub struct SetSessionArgs {
    pub id: u32,
    pub session: String,
}

/// Set the session for an issue.
pub fn run(args: SetSessionArgs) -> Result<()> {
    let project = Project::discover()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load the issue from data branch (source of truth)
    let mut issue_info = load_issue_from_data_branch(&project.repo_root, data_branch, args.id)?;

    // Update session field
    issue_info.issue.session = Some(args.session.clone());

    // Format issue content in memory and commit directly to data branch
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;
    let message = format!("Set session for issue #{} to {}", args.id, args.session);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &issue_info.relative_path,
        content.as_bytes(),
        &message,
    )?;

    println!("Set session for issue #{} to {}", args.id, args.session);

    Ok(())
}

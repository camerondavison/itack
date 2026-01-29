//! itack set-session command.

use crate::core::{Project, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::write_issue;

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

    // Load the issue from data branch (source of truth) and sync to working directory
    let mut issue_info =
        load_issue_from_data_branch(&project.repo_root, &project.itack_dir, data_branch, args.id)?;

    // Update session field
    issue_info.issue.session = Some(args.session.clone());

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

    // Commit to data branch
    let message = format!("Set session for issue #{} to {}", args.id, args.session);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &message,
    )?;

    // Delete from working directory (only exists in data branch)
    std::fs::remove_file(&issue_info.path)?;

    println!("Set session for issue #{} to {}", args.id, args.session);

    Ok(())
}

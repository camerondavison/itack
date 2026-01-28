//! itack done command.

use crate::core::{
    Project, Status, commit_to_branch, find_issue_file_in_branch, merge_branches,
    read_file_from_branch,
};
use crate::error::{ItackError, Result};
use crate::storage::db::{IssueInfo, load_issue};
use crate::storage::markdown::parse_issue;
use crate::storage::{format_issue, write_issue};

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

    // Load issue - in data-only mode, load from branch
    let mut issue_info = if project.config.merge_branch.is_none() {
        load_issue_from_branch(&project, data_branch, args.id)?
    } else {
        load_issue(&project.itack_dir, args.id)?
    };

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

    // Write to working directory if merge_branch is set
    if project.config.merge_branch.is_some() {
        write_issue(
            &issue_info.path,
            &issue_info.issue,
            &issue_info.title,
            &issue_info.body,
        )?;
    }

    // Commit to data branch
    let message = format!("Mark issue #{} as done", args.id);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &message,
    )?;

    // Merge into main if configured
    if let Some(ref merge_branch) = project.config.merge_branch
        && !merge_branch.is_empty()
    {
        merge_branches(&project.repo_root, data_branch, merge_branch)?;
    }

    println!(
        "Updated issue #{} status: {} -> {}",
        args.id,
        old_status,
        Status::Done
    );

    Ok(())
}

/// Load issue from data branch (for data-only mode).
fn load_issue_from_branch(project: &Project, branch: &str, id: u32) -> Result<IssueInfo> {
    // Find the issue file in the branch
    let relative_path = find_issue_file_in_branch(&project.repo_root, branch, id)?;
    let path = project.repo_root.join(&relative_path);

    let content = read_file_from_branch(&project.repo_root, branch, &relative_path)?;
    let content_str = String::from_utf8_lossy(&content);
    let (issue, title, body) = parse_issue(&content_str)?;

    Ok(IssueInfo {
        path,
        issue,
        title,
        body,
    })
}

//! itack create command.

use crate::core::{Issue, Project, commit_to_branch, merge_branches};
use crate::error::Result;
use crate::storage::{format_issue, write_issue};

/// Arguments for the create command.
pub struct CreateArgs {
    pub title: String,
    pub epic: Option<String>,
    pub body: Option<String>,
    pub message: Option<String>,
}

/// Create a new issue.
pub fn run(args: CreateArgs) -> Result<()> {
    let project = Project::discover()?;
    let db = project.open_db()?;

    // Get next issue ID atomically
    let id = db.next_issue_id()?;

    // Create the issue (title is stored in markdown, not in Issue struct)
    let issue = Issue::with_epic(id, args.epic);

    // Get the file path (relative to repo root)
    let path = project.issue_path_with_date(id, &issue.created);
    let relative_path = path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&path)
        .to_path_buf();

    // Format the issue content
    let body = args.body.as_deref().unwrap_or("");
    let content = format_issue(&issue, &args.title, body)?;

    // Write to working directory if merge_branch is set
    if project.config.merge_branch.is_some() {
        write_issue(&path, &issue, &args.title, body)?;
    }

    // Auto-commit to data branch
    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Create issue #{}: {}", id, args.title));

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &commit_message,
    )?;

    // Merge into main if configured
    if let Some(ref merge_branch) = project.config.merge_branch
        && !merge_branch.is_empty()
    {
        merge_branches(&project.repo_root, data_branch, merge_branch)?;
    }

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

//! itack create command.

use crate::core::{Issue, Project, commit_to_branch};
use crate::error::Result;
use crate::storage::markdown::format_issue;

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

    // Get the relative path for the git tree
    let relative_path = Project::issue_relative_path(id, &issue.created);

    // Format issue content in memory
    let body = args.body.as_deref().unwrap_or("");
    let content = format_issue(&issue, &args.title, body)?;

    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Create issue #{}: {}", id, args.title));

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Commit directly to data branch
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &commit_message,
    )?;

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

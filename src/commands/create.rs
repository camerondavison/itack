//! itack create command.

use crate::core::{Issue, Project, cherry_pick_to_head, commit_to_branch};
use crate::error::Result;
use crate::storage::format_issue;

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

    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Create issue #{}: {}", id, args.title));

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Commit to data branch
    let commit_oid = commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &commit_message,
    )?;

    // Cherry-pick onto current branch (updates working dir, index, and HEAD)
    if let Some(oid) = commit_oid {
        cherry_pick_to_head(&project.repo_root, oid, &commit_message)?;
    }

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

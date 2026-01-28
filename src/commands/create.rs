//! itack create command.

use crate::core::{Issue, Project, commit_to_branch};
use crate::error::Result;
use crate::storage::write_issue;

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

    // Write to working directory
    let body = args.body.as_deref().unwrap_or("");
    write_issue(&path, &issue, &args.title, body)?;

    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Create issue #{}: {}", id, args.title));

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Read back the content for data branch commit
    let content = std::fs::read(&path)?;

    // Commit to data branch only (feature branches get updated on 'done')
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &commit_message,
    )?;

    // Delete from working directory (only exists in data branch until 'done')
    std::fs::remove_file(&path)?;

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

//! itack create command.

use crate::core::{commit_file, Issue, Project};
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

    // Write to markdown file (use new filename format with date)
    let path = project.issue_path_with_date(id, &issue.created);
    write_issue(
        &path,
        &issue,
        &args.title,
        args.body.as_deref().unwrap_or(""),
    )?;

    // Auto-commit the new issue
    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Create issue #{}: {}", id, args.title));
    commit_file(&path, &commit_message)?;

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

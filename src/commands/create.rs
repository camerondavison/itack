//! itack create command.

use crate::core::{Issue, Project};
use crate::error::Result;
use crate::storage::write_issue;

/// Arguments for the create command.
pub struct CreateArgs {
    pub title: String,
    pub epic: Option<String>,
    pub body: Option<String>,
}

/// Create a new issue.
pub fn run(args: CreateArgs) -> Result<()> {
    let project = Project::discover()?;
    let db = project.open_db()?;

    // Get next issue ID atomically
    let id = db.next_issue_id()?;

    // Create the issue
    let issue = Issue::with_epic(id, args.title.clone(), args.epic);

    // Write to markdown file
    let path = project.issue_path(id);
    write_issue(&path, &issue, args.body.as_deref().unwrap_or(""))?;

    println!("Created issue #{}: {}", id, args.title);

    Ok(())
}

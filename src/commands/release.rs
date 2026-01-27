//! itack release command.

use crate::core::Project;
use crate::error::Result;
use crate::storage::db::load_issue;
use crate::storage::write_issue;

/// Arguments for the release command.
pub struct ReleaseArgs {
    pub id: u32,
}

/// Release a claim on an issue.
pub fn run(args: ReleaseArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut db = project.open_db()?;

    // Load the issue first to verify it exists
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    // Release in database
    db.release(args.id)?;

    // Update markdown file
    let old_assignee = issue_info.issue.assignee.take();

    write_issue(
        &issue_info.path,
        &issue_info.issue,
        &issue_info.title,
        &issue_info.body,
    )?;

    if let Some(assignee) = old_assignee {
        println!("Released issue #{} from {}", args.id, assignee);
    } else {
        println!("Released issue #{}", args.id);
    }

    Ok(())
}

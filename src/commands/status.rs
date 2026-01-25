//! itack status command.

use crate::core::{Project, Status};
use crate::error::Result;
use crate::storage::db::load_issue;
use crate::storage::write_issue;

/// Arguments for the status command.
pub struct StatusArgs {
    pub id: u32,
    pub status: Status,
}

/// Update an issue's status.
pub fn run(args: StatusArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    let old_status = issue_info.issue.status;
    issue_info.issue.status = args.status;

    write_issue(&issue_info.path, &issue_info.issue, &issue_info.body)?;

    println!(
        "Updated issue #{} status: {} -> {}",
        args.id, old_status, args.status
    );

    Ok(())
}

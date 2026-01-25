//! itack done command.

use crate::core::{Project, Status};
use crate::error::Result;
use crate::storage::db::load_issue;
use crate::storage::write_issue;

/// Arguments for the done command.
pub struct DoneArgs {
    pub id: u32,
}

/// Mark an issue as done.
pub fn run(args: DoneArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    let old_status = issue_info.issue.status;
    issue_info.issue.status = Status::Done;

    write_issue(&issue_info.path, &issue_info.issue, &issue_info.body)?;

    println!(
        "Updated issue #{} status: {} -> {}",
        args.id,
        old_status,
        Status::Done
    );

    Ok(())
}

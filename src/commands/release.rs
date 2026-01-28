//! itack release command.

use crate::core::{Project, commit_file_to_head, commit_to_branch};
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

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load the issue from working directory
    let mut issue_info = load_issue(&project.itack_dir, args.id)?;

    // Release in database
    db.release(args.id)?;

    // Update issue fields
    let old_assignee = issue_info.issue.assignee.take();
    issue_info.issue.branch = None;
    issue_info.issue.session = None;

    // Get relative path for commit
    let relative_path = issue_info
        .path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&issue_info.path)
        .to_path_buf();

    // Write to working directory
    write_issue(
        &issue_info.path,
        &issue_info.issue,
        &issue_info.title,
        &issue_info.body,
    )?;

    // Read back the content for data branch commit
    let content = std::fs::read(&issue_info.path)?;

    // Commit to data branch
    let message = format!("Release issue #{}", args.id);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &message,
    )?;

    // Commit to HEAD (stage and commit)
    commit_file_to_head(&project.repo_root, &relative_path, &message)?;

    if let Some(assignee) = old_assignee {
        println!("Released issue #{} from {}", args.id, assignee);
    } else {
        println!("Released issue #{}", args.id);
    }

    Ok(())
}

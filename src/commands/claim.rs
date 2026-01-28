//! itack claim command.

use crate::core::{
    Project, Status, commit_to_branch, find_issue_file_in_branch, merge_branches,
    read_file_from_branch,
};
use crate::error::Result;
use crate::storage::db::{IssueInfo, load_issue};
use crate::storage::markdown::parse_issue;
use crate::storage::{format_issue, write_issue};

/// Arguments for the claim command.
pub struct ClaimArgs {
    pub id: u32,
    pub assignee: String,
    pub session: Option<String>,
}

/// Claim an issue with SQLite-backed locking.
pub fn run(args: ClaimArgs) -> Result<()> {
    let project = Project::discover()?;
    let mut db = project.open_db()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load the issue first to verify it exists
    let mut issue_info = if project.config.merge_branch.is_none() {
        load_issue_from_branch(&project, data_branch, args.id)?
    } else {
        load_issue(&project.itack_dir, args.id)?
    };

    // Try to claim in database (atomic operation)
    db.claim(args.id, &args.assignee)?;

    // Update issue fields
    issue_info.issue.assignee = Some(args.assignee.clone());
    issue_info.issue.branch = project.current_branch();
    issue_info.issue.session = args.session.clone();
    if issue_info.issue.status == Status::Open {
        issue_info.issue.status = Status::InProgress;
    }

    // Get relative path for commit
    let relative_path = issue_info
        .path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&issue_info.path)
        .to_path_buf();

    // Format content
    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;

    // Write to working directory if merge_branch is set
    if project.config.merge_branch.is_some() {
        write_issue(
            &issue_info.path,
            &issue_info.issue,
            &issue_info.title,
            &issue_info.body,
        )?;
    }

    // Commit to data branch
    let message = format!("Claim issue #{} for {}", args.id, args.assignee);
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        content.as_bytes(),
        &message,
    )?;

    // Merge into main if configured
    if let Some(ref merge_branch) = project.config.merge_branch
        && !merge_branch.is_empty()
    {
        merge_branches(&project.repo_root, data_branch, merge_branch)?;
    }

    println!("Claimed issue #{} for {}", args.id, args.assignee);

    Ok(())
}

/// Load issue from data branch (for data-only mode).
fn load_issue_from_branch(project: &Project, branch: &str, id: u32) -> Result<IssueInfo> {
    // Find the issue file in the branch
    let relative_path = find_issue_file_in_branch(&project.repo_root, branch, id)?;
    let path = project.repo_root.join(&relative_path);

    let content = read_file_from_branch(&project.repo_root, branch, &relative_path)?;
    let content_str = String::from_utf8_lossy(&content);
    let (issue, title, body) = parse_issue(&content_str)?;

    Ok(IssueInfo {
        path,
        issue,
        title,
        body,
    })
}

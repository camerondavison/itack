//! itack undepend command.

use crate::core::{Project, commit_to_branch};
use crate::error::Result;
use crate::storage::db::load_issue_from_data_branch;
use crate::storage::markdown::format_issue;

/// Arguments for the undepend command.
pub struct UndependArgs {
    pub id: u32,
    pub deps: Vec<u32>,
}

/// Remove dependencies from an issue.
pub fn run(args: UndependArgs) -> Result<()> {
    let project = Project::discover()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    let mut issue_info = load_issue_from_data_branch(&project.repo_root, data_branch, args.id)?;

    // Remove specified deps
    issue_info
        .issue
        .depends_on
        .retain(|d| !args.deps.contains(d));

    let content = format_issue(&issue_info.issue, &issue_info.title, &issue_info.body)?;
    let dep_list: Vec<String> = args.deps.iter().map(|d| format!("#{d}")).collect();
    let message = format!(
        "Remove dependencies {} from issue #{}",
        dep_list.join(", "),
        args.id
    );
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &issue_info.relative_path,
        content.as_bytes(),
        &message,
    )?;

    println!(
        "Issue #{} now depends on: {:?}",
        args.id, issue_info.issue.depends_on
    );

    Ok(())
}

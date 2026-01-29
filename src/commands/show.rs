//! itack show command.

use crate::core::{Project, cleanup_working_file};
use crate::error::Result;
use crate::output::{self, OutputFormat};
use crate::storage::db::load_issue_from_data_branch;

/// Arguments for the show command.
pub struct ShowArgs {
    pub id: u32,
    pub format: OutputFormat,
}

/// Show an issue.
pub fn run(args: ShowArgs) -> Result<()> {
    let project = Project::discover()?;
    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Load issue from data branch (source of truth) - also syncs to working directory
    let issue_info =
        load_issue_from_data_branch(&project.repo_root, &project.itack_dir, data_branch, args.id)?;

    match args.format {
        OutputFormat::Table => {
            output::print_issue_detail(&issue_info.issue, &issue_info.title, &issue_info.body);
        }
        OutputFormat::Json => {
            output::print_issue_json(&issue_info.issue, &issue_info.title, &issue_info.body)?;
        }
    }

    // Restore file to HEAD state if it exists on this branch, otherwise delete
    let relative_path = issue_info
        .path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&issue_info.path)
        .to_path_buf();
    let _ = cleanup_working_file(&project.repo_root, &relative_path);

    Ok(())
}

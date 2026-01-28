//! itack list command.

use crate::core::{Project, Status};
use crate::error::Result;
use crate::output::{self, OutputFormat};
use crate::storage::db::load_all_issues_from_data_branch;

/// Arguments for the list command.
pub struct ListArgs {
    pub status: Option<Status>,
    pub epic: Option<String>,
    pub assignee: Option<String>,
    pub format: OutputFormat,
}

/// List issues with optional filters.
pub fn run(args: ListArgs) -> Result<()> {
    let project = Project::discover()?;
    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");
    let mut issues = load_all_issues_from_data_branch(&project.repo_root, data_branch)?;

    // Apply filters
    if let Some(status) = args.status {
        issues.retain(|i| i.issue.status == status);
    }

    if let Some(epic) = &args.epic {
        issues.retain(|i| i.issue.epic.as_ref() == Some(epic));
    }

    if let Some(assignee) = &args.assignee {
        issues.retain(|i| i.issue.assignee.as_ref() == Some(assignee));
    }

    match args.format {
        OutputFormat::Table => {
            output::print_issues_table(&issues);
        }
        OutputFormat::Json => {
            output::print_issues_json(&issues)?;
        }
    }

    Ok(())
}

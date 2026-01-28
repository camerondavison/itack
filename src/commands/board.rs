//! itack board command.

use crate::core::{Project, Status};
use crate::error::Result;
use crate::output::{self, OutputFormat};
use crate::storage::db::load_all_issues_from_data_branch;

/// Arguments for the board command.
pub struct BoardArgs {
    pub format: OutputFormat,
}

/// Board summary data.
pub struct BoardSummary {
    pub project_id: String,
    pub open_count: usize,
    pub in_progress_count: usize,
    pub done_count: usize,
    pub wontfix_count: usize,
    pub total_count: usize,
}

/// Show project board overview.
pub fn run(args: BoardArgs) -> Result<()> {
    let project = Project::discover()?;

    // Verify database exists (will error with helpful message if not)
    let _db = project.open_db()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");
    let issues = load_all_issues_from_data_branch(&project.repo_root, data_branch)?;

    let summary = BoardSummary {
        project_id: project.metadata.project_id.clone(),
        open_count: issues
            .iter()
            .filter(|i| i.issue.status == Status::Open)
            .count(),
        in_progress_count: issues
            .iter()
            .filter(|i| i.issue.status == Status::InProgress)
            .count(),
        done_count: issues
            .iter()
            .filter(|i| i.issue.status == Status::Done)
            .count(),
        wontfix_count: issues
            .iter()
            .filter(|i| i.issue.status == Status::WontFix)
            .count(),
        total_count: issues.len(),
    };

    match args.format {
        OutputFormat::Table => {
            output::print_board(&summary, &issues);
        }
        OutputFormat::Json => {
            output::print_board_json(&summary, &issues)?;
        }
    }

    Ok(())
}

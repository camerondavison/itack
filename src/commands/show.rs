//! itack show command.

use crate::core::Project;
use crate::error::Result;
use crate::output::{self, OutputFormat};
use crate::storage::db::load_issue;

/// Arguments for the show command.
pub struct ShowArgs {
    pub id: u32,
    pub format: OutputFormat,
}

/// Show an issue.
pub fn run(args: ShowArgs) -> Result<()> {
    let project = Project::discover()?;
    let issue_info = load_issue(&project.itack_dir, args.id)?;

    match args.format {
        OutputFormat::Table => {
            output::print_issue_detail(&issue_info.issue, &issue_info.body);
        }
        OutputFormat::Json => {
            output::print_issue_json(&issue_info.issue, &issue_info.body)?;
        }
    }

    Ok(())
}

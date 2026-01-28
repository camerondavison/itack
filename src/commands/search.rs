//! itack search command.

use std::process::Command;

use crate::core::Project;
use crate::error::Result;
use crate::output::{self, OutputFormat};
use crate::storage::db::load_all_issues_from_data_branch;

/// Arguments for the search command.
pub struct SearchArgs {
    pub query: String,
    pub all_branches: bool,
    pub format: OutputFormat,
}

/// Search for issues by query.
pub fn run(args: SearchArgs) -> Result<()> {
    let project = Project::discover()?;

    if args.all_branches {
        // Use git grep to search across all branches
        search_all_branches(&args.query)?;
    } else {
        // Search current issues by title and body
        let data_branch = project
            .config
            .data_branch
            .as_deref()
            .unwrap_or("data/itack");
        let issues = load_all_issues_from_data_branch(&project.repo_root, data_branch)?;
        let query_lower = args.query.to_lowercase();

        let matching: Vec<_> = issues
            .into_iter()
            .filter(|info| {
                info.title.to_lowercase().contains(&query_lower)
                    || info.body.to_lowercase().contains(&query_lower)
            })
            .collect();

        match args.format {
            OutputFormat::Table => {
                output::print_issues_table(&matching);
            }
            OutputFormat::Json => {
                output::print_issues_json(&matching)?;
            }
        }
    }

    Ok(())
}

/// Search for issues across all git branches using git grep.
fn search_all_branches(query: &str) -> Result<()> {
    let output = Command::new("git")
        .args([
            "grep",
            "-i",          // case insensitive
            "-n",          // show line numbers
            "--all-match", // all patterns must match
            query,
            "--",
            ".itack/",
        ])
        .output()?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        // git grep returns exit code 1 when no matches found
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        } else {
            println!("No matches found.");
        }
    }

    Ok(())
}

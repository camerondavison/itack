//! Table and JSON formatting.

use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, ContentArrangement, Table};
use serde::Serialize;

use crate::commands::board::BoardSummary;
use crate::core::Issue;
use crate::error::Result;
use crate::storage::db::IssueInfo;

/// Output format options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

/// Print a list of issues as a table.
pub fn print_issues_table(issues: &[IssueInfo]) {
    if issues.is_empty() {
        println!("No issues found.");
        return;
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);

table.set_header(vec![
        "ID",
        "Status",
        "Title",
        "Epic",
        "Assignee",
        "Depends On",
        "Session",
    ]);

    for info in issues {
        let issue = &info.issue;
        let depends_on = if issue.depends_on.is_empty() {
            "-".to_string()
        } else {
            issue
                .depends_on
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        table.add_row(vec![
            Cell::new(issue.id),
            Cell::new(issue.status.to_string()),
            Cell::new(&info.title),
            Cell::new(issue.epic.as_deref().unwrap_or("-")),
            Cell::new(issue.assignee.as_deref().unwrap_or("-")),
Cell::new(depends_on),
            Cell::new(issue.session.as_deref().unwrap_or("-")),
        ]);
    }

    println!("{}", table);
}

/// Print a list of issues as JSON.
pub fn print_issues_json(issues: &[IssueInfo]) -> Result<()> {
    #[derive(Serialize)]
    struct IssueOutput<'a> {
        id: u32,
        title: &'a str,
        status: String,
        epic: Option<&'a str>,
        assignee: Option<&'a str>,
        session: Option<&'a str>,
        created: String,
        depends_on: &'a [u32],
    }

    let output: Vec<IssueOutput> = issues
        .iter()
        .map(|info| IssueOutput {
            id: info.issue.id,
            title: &info.title,
            status: info.issue.status.to_string(),
            epic: info.issue.epic.as_deref(),
            assignee: info.issue.assignee.as_deref(),
            session: info.issue.session.as_deref(),
            created: info.issue.created.to_rfc3339(),
            depends_on: &info.issue.depends_on,
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print issue detail as a table.
pub fn print_issue_detail(issue: &Issue, title: &str, body: &str) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![Cell::new("ID"), Cell::new(issue.id)]);
    table.add_row(vec![Cell::new("Title"), Cell::new(title)]);
    table.add_row(vec![
        Cell::new("Status"),
        Cell::new(issue.status.to_string()),
    ]);
    table.add_row(vec![
        Cell::new("Epic"),
        Cell::new(issue.epic.as_deref().unwrap_or("-")),
    ]);
    table.add_row(vec![
        Cell::new("Assignee"),
        Cell::new(issue.assignee.as_deref().unwrap_or("-")),
    ]);
    let depends_on = if issue.depends_on.is_empty() {
        "-".to_string()
    } else {
        issue
            .depends_on
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    table.add_row(vec![Cell::new("Depends On"), Cell::new(depends_on)]);
    table.add_row(vec![
        Cell::new("Session"),
        Cell::new(issue.session.as_deref().unwrap_or("-")),
    ]);
    table.add_row(vec![
        Cell::new("Created"),
        Cell::new(issue.created.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
    ]);

    println!("{}", table);

    if !body.trim().is_empty() {
        println!("\nDescription:");
        println!("{}", body);
    }
}

/// Print issue detail as JSON.
pub fn print_issue_json(issue: &Issue, title: &str, body: &str) -> Result<()> {
    #[derive(Serialize)]
    struct IssueDetail<'a> {
        id: u32,
        title: &'a str,
        status: String,
        epic: Option<&'a str>,
        assignee: Option<&'a str>,
depends_on: &'a [u32],
        session: Option<&'a str>,
        created: String,
        body: &'a str,
    }

    let output = IssueDetail {
        id: issue.id,
        title,
        status: issue.status.to_string(),
        epic: issue.epic.as_deref(),
        assignee: issue.assignee.as_deref(),
depends_on: &issue.depends_on,
        session: issue.session.as_deref(),
        created: issue.created.to_rfc3339(),
        body,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Print board overview as a table.
pub fn print_board(summary: &BoardSummary, issues: &[IssueInfo]) {
    println!("Project: {}", summary.project_id);
    println!();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);

    table.set_header(vec!["Status", "Count"]);
    table.add_row(vec![Cell::new("Open"), Cell::new(summary.open_count)]);
    table.add_row(vec![
        Cell::new("In Progress"),
        Cell::new(summary.in_progress_count),
    ]);
    table.add_row(vec![Cell::new("Done"), Cell::new(summary.done_count)]);
    table.add_row(vec![Cell::new("Total"), Cell::new(summary.total_count)]);

    println!("{}", table);

    if !issues.is_empty() {
        println!("\nRecent Issues:");
        print_issues_table(&issues[..std::cmp::min(10, issues.len())]);
    }
}

/// Print board overview as JSON.
pub fn print_board_json(summary: &BoardSummary, issues: &[IssueInfo]) -> Result<()> {
    #[derive(Serialize)]
    struct BoardOutput<'a> {
        project_id: &'a str,
        counts: Counts,
        issues: Vec<IssueOutput<'a>>,
    }

    #[derive(Serialize)]
    struct Counts {
        open: usize,
        in_progress: usize,
        done: usize,
        total: usize,
    }

    #[derive(Serialize)]
    struct IssueOutput<'a> {
        id: u32,
        title: &'a str,
        status: String,
        epic: Option<&'a str>,
        assignee: Option<&'a str>,
        session: Option<&'a str>,
    }

    let output = BoardOutput {
        project_id: &summary.project_id,
        counts: Counts {
            open: summary.open_count,
            in_progress: summary.in_progress_count,
            done: summary.done_count,
            total: summary.total_count,
        },
        issues: issues
            .iter()
            .map(|info| IssueOutput {
                id: info.issue.id,
                title: &info.title,
                status: info.issue.status.to_string(),
                epic: info.issue.epic.as_deref(),
                assignee: info.issue.assignee.as_deref(),
                session: info.issue.session.as_deref(),
            })
            .collect(),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

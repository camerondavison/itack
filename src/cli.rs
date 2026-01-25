//! Clap argument definitions.

use clap::{Parser, Subcommand};

use crate::core::Status;

/// Git-backed issue tracker for multi-agent coordination.
#[derive(Parser, Debug)]
#[command(name = "itack")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new itack project in the current git repository
    Init,

    /// Create a new issue
    Create {
        /// Issue title
        title: String,

        /// Epic/category for grouping
        #[arg(short, long)]
        epic: Option<String>,
    },

    /// Show issue details
    Show {
        /// Issue ID
        id: u32,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Open issue in editor
    Edit {
        /// Issue ID
        id: u32,
    },

    /// Update issue status
    Status {
        /// Issue ID
        id: u32,

        /// New status (open, in-progress, done)
        #[arg(value_parser = parse_status)]
        status: Status,
    },

    /// Claim an issue for an assignee
    Claim {
        /// Issue ID
        id: u32,

        /// Assignee name
        assignee: String,
    },

    /// Release a claimed issue
    Release {
        /// Issue ID
        id: u32,
    },

    /// List issues
    List {
        /// Filter by status
        #[arg(short, long, value_parser = parse_status)]
        status: Option<Status>,

        /// Filter by epic
        #[arg(short, long)]
        epic: Option<String>,

        /// Filter by assignee
        #[arg(short, long)]
        assignee: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show project board overview
    Board {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn parse_status(s: &str) -> Result<Status, String> {
    s.parse::<Status>().map_err(|e| e.to_string())
}

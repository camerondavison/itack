//! Clap argument definitions.

use clap::{Parser, Subcommand};
use clap_complete::Shell;

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

        /// Issue body/description
        #[arg(short, long)]
        body: Option<String>,

        /// Custom git commit message (defaults to "Create issue #N: <title>")
        #[arg(short, long)]
        message: Option<String>,

        /// Issue IDs this issue depends on (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        depends_on: Vec<u32>,
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

        /// Set the issue body directly (skips editor)
        #[arg(short, long)]
        body: Option<String>,

        /// Custom git commit message (defaults to "Edit issue #N")
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Mark issue as done
    Done {
        /// Issue ID
        id: u32,
    },

    /// Mark issue as wont-fix
    WontFix {
        /// Issue ID
        id: u32,
    },

    /// Claim an issue for an assignee
    Claim {
        /// Issue ID
        id: u32,

        /// Assignee name
        assignee: String,

        /// Session ID (e.g., Claude Code session working on this issue)
        #[arg(short, long)]
        session: Option<String>,
    },

    /// Release a claimed issue
    Release {
        /// Issue ID
        id: u32,
    },

    /// Add dependencies to an issue
    Depend {
        /// Issue ID
        id: u32,

        /// Dependency issue IDs to add
        deps: Vec<u32>,
    },

    /// Remove dependencies from an issue
    Undepend {
        /// Issue ID
        id: u32,

        /// Dependency issue IDs to remove
        deps: Vec<u32>,
    },

    /// Set the session for an issue
    SetSession {
        /// Issue ID
        id: u32,

        /// Session name (e.g., ccx session name)
        session: String,
    },

    /// List issues
    List {
        /// Filter by status
        #[arg(short, long)]
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

    /// Check database health and issue synchronization
    Doctor,

    /// Search for issues by query
    Search {
        /// Search query
        query: String,

        /// Search across all git branches (uses git grep)
        #[arg(short, long)]
        all_branches: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

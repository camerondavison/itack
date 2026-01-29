//! Error types and exit codes for itack.

use std::process::ExitCode;
use thiserror::Error;

/// Exit codes for the CLI.
pub mod exit_codes {
    /// Success.
    pub const SUCCESS: u8 = 0;
    /// General error.
    pub const ERROR: u8 = 1;
    /// Conflict error (e.g., claim already held).
    pub const CONFLICT: u8 = 2;
}

/// Main error type for itack operations.
#[derive(Error, Debug)]
pub enum ItackError {
    #[error("Not in a git repository")]
    NotInGitRepo,

    #[error("Project not initialized. Run 'itack init' first.")]
    NotInitialized,

    #[error("Issue {0} not found")]
    IssueNotFound(u32),

    #[error("Issue {0} is already claimed by {1}")]
    AlreadyClaimed(u32, String),

    #[error("Issue {0} is not claimed")]
    NotClaimed(u32),

    #[error("Issue {0} is already done")]
    AlreadyDone(u32),

    #[error("Data branch '{0}' not found. Run 'itack init' to create it.")]
    DataBranchNotFound(String),

    #[error("No .itack directory found on data branch '{0}'. Run 'itack init' to repair.")]
    DataBranchEmpty(String),

    #[error("Database not found at {0}. Run 'itack init' to fix.")]
    DatabaseNotFound(std::path::PathBuf),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Editor failed: {0}")]
    EditorFailed(String),

    #[error("Invalid markdown format: {0}")]
    InvalidMarkdown(String),

    #[error("{0}")]
    Other(String),
}

impl ItackError {
    /// Get the exit code for this error.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            ItackError::AlreadyClaimed(_, _) => ExitCode::from(exit_codes::CONFLICT),
            _ => ExitCode::from(exit_codes::ERROR),
        }
    }
}

/// Result type alias for itack operations.
pub type Result<T> = std::result::Result<T, ItackError>;

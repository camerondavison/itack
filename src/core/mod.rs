//! Core types and functionality.

pub mod config;
pub mod git;
pub mod issue;
pub mod project;
pub mod status;

pub use config::Config;
pub use git::{
    cleanup_working_file, commit_file_to_head, commit_to_branch, find_issue_in_branch,
    read_file_from_branch,
};
pub use issue::Issue;
pub use project::Project;
pub use status::Status;

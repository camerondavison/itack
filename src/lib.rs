//! itack - Git-backed issue tracker library.

pub mod cli;
pub mod commands;
pub mod core;
pub mod error;
pub mod output;
pub mod storage;

// Re-export commonly used types
pub use core::{Config, Issue, Project, Status};
pub use error::{ItackError, Result};

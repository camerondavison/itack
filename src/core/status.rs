//! Issue status enum with sort priority.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::{ItackError, Result};

/// Issue status with defined sort priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Done,
    WontFix,
}

impl Status {
    /// Get the sort priority (lower = higher priority).
    /// in-progress=0, open=1, done=2, wontfix=3
    pub fn sort_priority(&self) -> u8 {
        match self {
            Status::InProgress => 0,
            Status::Open => 1,
            Status::Done => 2,
            Status::WontFix => 3,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Open => write!(f, "open"),
            Status::InProgress => write!(f, "in-progress"),
            Status::Done => write!(f, "done"),
            Status::WontFix => write!(f, "wontfix"),
        }
    }
}

impl FromStr for Status {
    type Err = ItackError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Status::Open),
            "in-progress" | "inprogress" | "in_progress" => Ok(Status::InProgress),
            "done" => Ok(Status::Done),
            "wontfix" | "wont-fix" | "wont_fix" => Ok(Status::WontFix),
            _ => Err(ItackError::InvalidStatus(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_sort_priority() {
        assert_eq!(Status::InProgress.sort_priority(), 0);
        assert_eq!(Status::Open.sort_priority(), 1);
        assert_eq!(Status::Done.sort_priority(), 2);
        assert_eq!(Status::WontFix.sort_priority(), 3);
    }

    #[test]
    fn test_status_from_str() {
        assert_eq!(Status::from_str("open").unwrap(), Status::Open);
        assert_eq!(Status::from_str("in-progress").unwrap(), Status::InProgress);
        assert_eq!(Status::from_str("done").unwrap(), Status::Done);
        assert_eq!(Status::from_str("wontfix").unwrap(), Status::WontFix);
        assert_eq!(Status::from_str("wont-fix").unwrap(), Status::WontFix);
        assert_eq!(Status::from_str("wont_fix").unwrap(), Status::WontFix);
        assert!(Status::from_str("invalid").is_err());
    }

    #[test]
    fn test_status_display() {
        assert_eq!(Status::Open.to_string(), "open");
        assert_eq!(Status::InProgress.to_string(), "in-progress");
        assert_eq!(Status::Done.to_string(), "done");
        assert_eq!(Status::WontFix.to_string(), "wontfix");
    }
}

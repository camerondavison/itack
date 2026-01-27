//! Issue struct with YAML serialization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::core::Status;

/// An issue in the tracker.
/// Fields are ordered alphabetically for consistent YAML output.
/// Note: The title is stored in the markdown body as an H1 heading, not in YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Assignee (who has claimed the issue).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Branch name where this issue is being worked on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Creation timestamp.
    pub created: DateTime<Utc>,

    /// Issue IDs that this issue depends on (must be completed first).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<u32>,

    /// Epic/category for grouping issues.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,

    /// Unique issue ID.
    pub id: u32,

    /// Session ID (e.g., Claude Code session) working on this issue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,

    /// Current status.
    pub status: Status,
}

impl Issue {
    /// Create a new issue with the given ID.
    pub fn new(id: u32) -> Self {
        Issue {
            assignee: None,
            branch: None,
            created: Utc::now(),
            depends_on: Vec::new(),
            epic: None,
            id,
            session: None,
            status: Status::default(),
        }
    }

    /// Create a new issue with optional epic.
    pub fn with_epic(id: u32, epic: Option<String>) -> Self {
        let mut issue = Self::new(id);
        issue.epic = epic;
        issue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_issue() {
        let issue = Issue::new(1);
        assert_eq!(issue.id, 1);
        assert_eq!(issue.status, Status::Open);
        assert!(issue.assignee.is_none());
        assert!(issue.epic.is_none());
    }

    #[test]
    fn test_issue_with_epic() {
        let issue = Issue::with_epic(2, Some("MVP".to_string()));
        assert_eq!(issue.epic, Some("MVP".to_string()));
    }
}

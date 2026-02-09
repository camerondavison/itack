//! Markdown file I/O with YAML front matter.

use crate::core::Issue;
use crate::error::{ItackError, Result};

const FRONT_MATTER_DELIMITER: &str = "---";

/// Parse an issue from a markdown file with YAML front matter.
/// Returns the issue, title (from H1 heading), and body (without title heading).
pub fn parse_issue(content: &str) -> Result<(Issue, String, String)> {
    let content = content.trim_start();

    if !content.starts_with(FRONT_MATTER_DELIMITER) {
        return Err(ItackError::InvalidMarkdown(
            "Missing YAML front matter".to_string(),
        ));
    }

    let after_first = &content[FRONT_MATTER_DELIMITER.len()..];
    let Some(end_pos) = after_first.find(FRONT_MATTER_DELIMITER) else {
        return Err(ItackError::InvalidMarkdown(
            "Unclosed YAML front matter".to_string(),
        ));
    };

    let yaml_content = &after_first[..end_pos];
    let body_start = FRONT_MATTER_DELIMITER.len() + end_pos + FRONT_MATTER_DELIMITER.len();
    let body = content[body_start..].trim_start_matches('\n').to_string();

    let issue: Issue = serde_yaml::from_str(yaml_content)?;

    // Extract the title from H1 heading
    let (title, body) = extract_title_heading(&body)?;

    Ok((issue, title, body))
}

/// Extract the title from the H1 heading and return (title, remaining body).
fn extract_title_heading(body: &str) -> Result<(String, String)> {
    if let Some(rest) = body.strip_prefix("# ") {
        // Find end of line
        let newline_pos = rest.find('\n').unwrap_or(rest.len());
        let title = rest[..newline_pos].to_string();
        let remaining = rest[newline_pos..].trim_start_matches('\n').to_string();
        Ok((title, remaining))
    } else {
        Err(ItackError::InvalidMarkdown(
            "Missing title heading (# Title)".to_string(),
        ))
    }
}

/// Format an issue as markdown with YAML front matter.
/// The title is stored as an H1 heading in the body, not in YAML.
pub fn format_issue(issue: &Issue, title: &str, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(issue)?;
    let mut result = String::new();
    result.push_str(FRONT_MATTER_DELIMITER);
    result.push('\n');
    result.push_str(&yaml);
    result.push_str(FRONT_MATTER_DELIMITER);
    result.push('\n');
    // Add title as H1 heading (this is the canonical location for the title)
    result.push('\n');
    result.push_str("# ");
    result.push_str(title);
    result.push('\n');
    if !body.is_empty() {
        result.push('\n');
        result.push_str(body);
        if !body.ends_with('\n') {
            result.push('\n');
        }
    }
    Ok(result)
}

/// Read an issue from a markdown file.
/// Returns the issue, title, and body.
#[cfg(test)]
fn read_issue(path: &std::path::Path) -> Result<(Issue, String, String)> {
    let content = std::fs::read_to_string(path)?;
    parse_issue(&content)
}

/// Write an issue to a markdown file.
#[cfg(test)]
fn write_issue(path: &std::path::Path, issue: &Issue, title: &str, body: &str) -> Result<()> {
    let content = format_issue(issue, title, body)?;
    std::fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Status;
    use tempfile::TempDir;

    #[test]
    fn test_parse_issue() {
        let content = r#"---
assignee: agent-1
created: 2024-01-15T10:30:00Z
id: 1
status: open
---

# Test issue

This is the body.
"#;

        let (issue, title, body) = parse_issue(content).unwrap();
        assert_eq!(issue.id, 1);
        assert_eq!(title, "Test issue");
        assert_eq!(issue.status, Status::Open);
        assert_eq!(issue.assignee, Some("agent-1".to_string()));
        assert_eq!(body, "This is the body.\n");
    }

    #[test]
    fn test_parse_issue_no_body() {
        let content = r#"---
created: 2024-01-15T10:30:00Z
id: 1
status: open
---

# Just a title
"#;

        let (issue, title, body) = parse_issue(content).unwrap();
        assert_eq!(issue.id, 1);
        assert_eq!(title, "Just a title");
        assert_eq!(body, "");
    }

    #[test]
    fn test_format_issue() {
        let issue = Issue::new(1);
        let title = "Test issue";
        let body = "This is the body.";

        let formatted = format_issue(&issue, title, body).unwrap();
        assert!(formatted.starts_with("---\n"));
        assert!(formatted.contains("id: 1"));
        assert!(!formatted.contains("title:")); // Title should NOT be in YAML
        assert!(formatted.contains("status: open"));
        assert!(formatted.contains("\n# Test issue\n"));
        assert!(formatted.ends_with("This is the body.\n"));
    }

    #[test]
    fn test_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("1.md");

        let mut issue = Issue::new(1);
        issue.epic = Some("MVP".to_string());
        let title = "Test issue";
        let body = "Description here.";

        write_issue(&path, &issue, title, body).unwrap();
        let (loaded, loaded_title, loaded_body) = read_issue(&path).unwrap();

        assert_eq!(loaded.id, issue.id);
        assert_eq!(loaded_title, title);
        assert_eq!(loaded.epic, issue.epic);
        assert_eq!(loaded_body.trim(), body);
    }

    #[test]
    fn test_invalid_markdown() {
        assert!(parse_issue("no front matter").is_err());
        assert!(parse_issue("---\nunclosed").is_err());
    }

    #[test]
    fn test_missing_title_heading() {
        let content = r#"---
created: 2024-01-15T10:30:00Z
id: 1
status: open
---

No H1 heading here.
"#;
        assert!(parse_issue(content).is_err());
    }
}

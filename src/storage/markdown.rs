//! Markdown file I/O with YAML front matter.

use std::fs;
use std::path::Path;

use crate::core::Issue;
use crate::error::{ItackError, Result};

const FRONT_MATTER_DELIMITER: &str = "---";

/// Parse an issue from a markdown file with YAML front matter.
pub fn parse_issue(content: &str) -> Result<(Issue, String)> {
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

    // Strip the title heading from body if present (since we write it automatically)
    let body = strip_title_heading(&body, &issue.title);

    Ok((issue, body))
}

/// Strip the title heading from the body if it matches the issue title.
fn strip_title_heading(body: &str, title: &str) -> String {
    let expected_heading = format!("# {}", title);
    if let Some(rest) = body.strip_prefix(&expected_heading) {
        rest.trim_start_matches('\n').to_string()
    } else {
        body.to_string()
    }
}

/// Check if a markdown file has the title heading after frontmatter.
pub fn has_title_heading(content: &str) -> bool {
    let content = content.trim_start();
    if !content.starts_with(FRONT_MATTER_DELIMITER) {
        return false;
    }

    let after_first = &content[FRONT_MATTER_DELIMITER.len()..];
    let Some(end_pos) = after_first.find(FRONT_MATTER_DELIMITER) else {
        return false;
    };

    let body_start = FRONT_MATTER_DELIMITER.len() + end_pos + FRONT_MATTER_DELIMITER.len();
    let body = content[body_start..].trim_start_matches('\n');

    body.starts_with("# ")
}

/// Format an issue as markdown with YAML front matter.
pub fn format_issue(issue: &Issue, body: &str) -> Result<String> {
    let yaml = serde_yaml::to_string(issue)?;
    let mut result = String::new();
    result.push_str(FRONT_MATTER_DELIMITER);
    result.push('\n');
    result.push_str(&yaml);
    result.push_str(FRONT_MATTER_DELIMITER);
    result.push('\n');
    // Add title as H1 heading so it's visible in markdown viewers like glow
    result.push('\n');
    result.push_str("# ");
    result.push_str(&issue.title);
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
pub fn read_issue(path: &Path) -> Result<(Issue, String)> {
    let content = fs::read_to_string(path)?;
    parse_issue(&content)
}

/// Write an issue to a markdown file.
pub fn write_issue(path: &Path, issue: &Issue, body: &str) -> Result<()> {
    let content = format_issue(issue, body)?;
    fs::write(path, content)?;
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
title: Test issue
---

This is the body.
"#;

        let (issue, body) = parse_issue(content).unwrap();
        assert_eq!(issue.id, 1);
        assert_eq!(issue.title, "Test issue");
        assert_eq!(issue.status, Status::Open);
        assert_eq!(issue.assignee, Some("agent-1".to_string()));
        assert_eq!(body, "This is the body.\n");
    }

    #[test]
    fn test_parse_issue_strips_title_heading() {
        let content = r#"---
assignee: agent-1
created: 2024-01-15T10:30:00Z
id: 1
status: open
title: Test issue
---

# Test issue

This is the body.
"#;

        let (issue, body) = parse_issue(content).unwrap();
        assert_eq!(issue.id, 1);
        assert_eq!(issue.title, "Test issue");
        assert_eq!(body, "This is the body.\n");
    }

    #[test]
    fn test_format_issue() {
        let issue = Issue::new(1, "Test issue".to_string());
        let body = "This is the body.";

        let formatted = format_issue(&issue, body).unwrap();
        assert!(formatted.starts_with("---\n"));
        assert!(formatted.contains("id: 1"));
        assert!(formatted.contains("title: Test issue"));
        assert!(formatted.contains("status: open"));
        assert!(formatted.contains("\n# Test issue\n"));
        assert!(formatted.ends_with("This is the body.\n"));
    }

    #[test]
    fn test_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("1.md");

        let mut issue = Issue::new(1, "Test issue".to_string());
        issue.epic = Some("MVP".to_string());
        let body = "Description here.";

        write_issue(&path, &issue, body).unwrap();
        let (loaded, loaded_body) = read_issue(&path).unwrap();

        assert_eq!(loaded.id, issue.id);
        assert_eq!(loaded.title, issue.title);
        assert_eq!(loaded.epic, issue.epic);
        assert_eq!(loaded_body.trim(), body);
    }

    #[test]
    fn test_invalid_markdown() {
        assert!(parse_issue("no front matter").is_err());
        assert!(parse_issue("---\nunclosed").is_err());
    }
}

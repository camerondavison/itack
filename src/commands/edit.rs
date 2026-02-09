//! itack edit command.

use std::io::Write;
use std::process::Command;

use crate::core::{Project, commit_to_branch, find_issue_in_branch, read_file_from_branch};
use crate::error::{ItackError, Result};
use crate::storage::markdown::{format_issue, parse_issue};

/// Arguments for the edit command.
pub struct EditArgs {
    pub id: u32,
    pub body: Option<String>,
    pub message: Option<String>,
}

/// Open an issue in the editor, or update the body directly if provided.
pub fn run(args: EditArgs) -> Result<()> {
    let project = Project::discover()?;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Edit issue #{}", args.id));

    // Find the issue file in the data branch (source of truth)
    let relative_path = find_issue_in_branch(&project.repo_root, data_branch, args.id)?
        .ok_or(ItackError::IssueNotFound(args.id))?;

    // Read the latest content from the data branch
    let current_content_bytes =
        read_file_from_branch(&project.repo_root, data_branch, &relative_path)?
            .ok_or(ItackError::IssueNotFound(args.id))?;
    let current_content = String::from_utf8(current_content_bytes)
        .map_err(|e| ItackError::InvalidMarkdown(format!("Invalid UTF-8 in issue file: {}", e)))?;

    // If body is provided, update directly without editor
    let new_content = if let Some(new_body) = args.body {
        let (issue, title, _old_body) = parse_issue(&current_content)?;
        format_issue(&issue, &title, &new_body)?
    } else {
        // Editor-based workflow using a temp file
        let editor = project.config.get_editor();

        let mut temp_file = tempfile::Builder::new()
            .suffix(".md")
            .tempfile()
            .map_err(|e| ItackError::Other(format!("Failed to create temp file: {}", e)))?;

        temp_file
            .write_all(current_content.as_bytes())
            .map_err(|e| ItackError::Other(format!("Failed to write temp file: {}", e)))?;

        let temp_path = temp_file.path().to_path_buf();

        // Open editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .map_err(|e| ItackError::EditorFailed(format!("Failed to launch {}: {}", editor, e)))?;

        if !status.success() {
            return Err(ItackError::EditorFailed(format!(
                "Editor exited with status: {}",
                status
            )));
        }

        // Read the edited content
        std::fs::read_to_string(&temp_path)?
    };

    // Commit to data branch only
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        new_content.as_bytes(),
        &commit_message,
    )?;

    Ok(())
}

//! itack edit command.

use std::fs;
use std::process::Command;

use crate::core::{Project, commit_to_branch, find_issue_in_branch, read_file_from_branch};
use crate::error::{ItackError, Result};

/// Arguments for the edit command.
pub struct EditArgs {
    pub id: u32,
    pub message: Option<String>,
}

/// Open an issue in the editor.
pub fn run(args: EditArgs) -> Result<()> {
    let project = Project::discover()?;
    let editor = project.config.get_editor();

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

    let path = project.repo_root.join(&relative_path);

    // Read the latest content from the data branch and sync to working directory
    if let Some(content) = read_file_from_branch(&project.repo_root, data_branch, &relative_path)? {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &content)?;
    } else {
        return Err(ItackError::IssueNotFound(args.id));
    }

    // Open editor
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .map_err(|e| ItackError::EditorFailed(format!("Failed to launch {}: {}", editor, e)))?;

    if !status.success() {
        return Err(ItackError::EditorFailed(format!(
            "Editor exited with status: {}",
            status
        )));
    }

    // Read the edited content
    let content = fs::read(&path)?;

    // Get relative path for commit
    let relative_path = path
        .strip_prefix(&project.repo_root)
        .unwrap_or(&path)
        .to_path_buf();

    // Commit to data branch only (feature branches get updated on 'done')
    commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &commit_message,
    )?;

    // Delete from working directory (only exists in data branch until 'done')
    fs::remove_file(&path)?;

    Ok(())
}

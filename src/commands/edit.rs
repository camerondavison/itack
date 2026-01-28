//! itack edit command.

use std::fs;
use std::process::Command;

use crate::core::{
    Project, commit_to_branch, find_issue_file_in_branch, merge_branches, read_file_from_branch,
};
use crate::error::{ItackError, Result};
use crate::storage::db::load_issue;

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

    if project.config.merge_branch.is_none() {
        // Data-only mode: read from branch, edit in temp file
        let relative_path = find_issue_file_in_branch(&project.repo_root, data_branch, args.id)?;
        let content = read_file_from_branch(&project.repo_root, data_branch, &relative_path)?;

        // Write to temp file
        let temp_path = std::env::temp_dir().join(format!("itack-{}.md", args.id));
        fs::write(&temp_path, &content)?;

        // Open editor
        let status = Command::new(&editor)
            .arg(&temp_path)
            .status()
            .map_err(|e| ItackError::EditorFailed(format!("Failed to launch {}: {}", editor, e)))?;

        if !status.success() {
            let _ = fs::remove_file(&temp_path);
            return Err(ItackError::EditorFailed(format!(
                "Editor exited with status: {}",
                status
            )));
        }

        // Read edited content and commit
        let edited = fs::read(&temp_path)?;
        commit_to_branch(
            &project.repo_root,
            data_branch,
            &relative_path,
            &edited,
            &commit_message,
        )?;

        // Clean up
        let _ = fs::remove_file(&temp_path);
    } else {
        // Merged mode: edit file in place
        let issue_info = load_issue(&project.itack_dir, args.id)?;
        let path = issue_info.path;

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

        // Commit to data branch
        commit_to_branch(
            &project.repo_root,
            data_branch,
            &relative_path,
            &content,
            &commit_message,
        )?;

        // Merge into main if configured
        if let Some(ref merge_branch) = project.config.merge_branch
            && !merge_branch.is_empty()
        {
            merge_branches(&project.repo_root, data_branch, merge_branch)?;
        }
    }

    Ok(())
}

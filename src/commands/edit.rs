//! itack edit command.

use std::fs;
use std::process::Command;

use crate::core::{Project, cherry_pick_to_head, commit_to_branch};
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

    // Load issue to get its path
    let issue_info = load_issue(&project.itack_dir, args.id)?;
    let path = issue_info.path;

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

    // Commit to data branch
    let commit_oid = commit_to_branch(
        &project.repo_root,
        data_branch,
        &relative_path,
        &content,
        &commit_message,
    )?;

    // Cherry-pick onto current branch (updates working dir, index, and HEAD)
    if let Some(oid) = commit_oid {
        cherry_pick_to_head(&project.repo_root, oid, &commit_message)?;
    }

    Ok(())
}

//! itack edit command.

use std::process::Command;

use crate::core::{commit_file, Project};
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

    // Load issue to get its path (supports both old and new filename formats)
    let issue_info = load_issue(&project.itack_dir, args.id)?;

    let path = issue_info.path;
    let editor = project.config.get_editor();

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

    // Auto-commit the edited issue
    let commit_message = args
        .message
        .unwrap_or_else(|| format!("Edit issue #{}", args.id));
    commit_file(&path, &commit_message)?;

    Ok(())
}

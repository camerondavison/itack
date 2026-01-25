//! itack edit command.

use std::process::Command;

use crate::core::Project;
use crate::error::{ItackError, Result};
use crate::storage::db::load_issue;

/// Arguments for the edit command.
pub struct EditArgs {
    pub id: u32,
}

/// Open an issue in the editor.
pub fn run(args: EditArgs) -> Result<()> {
    let project = Project::discover()?;

    // Verify issue exists
    let _ = load_issue(&project.itack_dir, args.id)?;

    let path = project.issue_path(args.id);
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

    Ok(())
}

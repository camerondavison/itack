//! Git operations for itack.

use std::path::Path;
use std::process::Command;

use crate::error::{ItackError, Result};

/// Commit a file with the given message.
///
/// This uses the git CLI to add and commit the file.
/// Returns Ok(()) even if there are no changes to commit.
pub fn commit_file(path: &Path, message: &str) -> Result<()> {
    // Stage the file
    let status = Command::new("git")
        .args(["add", "--"])
        .arg(path)
        .status()
        .map_err(|e| ItackError::Other(format!("Failed to run git add: {}", e)))?;

    if !status.success() {
        return Err(ItackError::Other(format!(
            "git add failed with status: {}",
            status
        )));
    }

    // Check if there are staged changes for this file
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet", "--"])
        .arg(path)
        .status()
        .map_err(|e| ItackError::Other(format!("Failed to run git diff: {}", e)))?;

    // Exit code 0 means no differences (nothing to commit)
    if output.success() {
        return Ok(());
    }

    // Commit the file
    let status = Command::new("git")
        .args(["commit", "-m", message, "--"])
        .arg(path)
        .status()
        .map_err(|e| ItackError::Other(format!("Failed to run git commit: {}", e)))?;

    if !status.success() {
        return Err(ItackError::Other(format!(
            "git commit failed with status: {}",
            status
        )));
    }

    Ok(())
}

//! itack init command.

use std::fs;

use crate::core::{Config, Project};
use crate::error::{ItackError, Result};
use crate::storage::Metadata;

/// Initialize a new itack project in the current git repository.
pub fn run() -> Result<()> {
    let repo_root = Project::find_repo_root()?;

    if Project::is_initialized(&repo_root) {
        return Err(ItackError::AlreadyInitialized);
    }

    // Create .itack directory
    let itack_dir = repo_root.join(".itack");
    fs::create_dir_all(&itack_dir)?;

    // Create metadata.toml with random project ID
    let metadata = Metadata::new();
    metadata.save(&itack_dir.join("metadata.toml"))?;

    // Create .gitignore for .itack directory (ignore the db, keep md files)
    let gitignore_content = "# itack ignores\n*.db\n*.db-wal\n*.db-shm\n";
    fs::write(itack_dir.join(".gitignore"), gitignore_content)?;

    // Initialize global config directory
    Config::init_global()?;

    // Open database to initialize it
    let project = Project::discover()?;
    let _db = project.open_db()?;

    println!("Initialized itack project: {}", metadata.project_id);
    println!("Issues will be stored in: .itack/");

    Ok(())
}

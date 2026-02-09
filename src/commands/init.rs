//! itack init command.

use std::fs;

use crate::core::{Config, Project};
use crate::error::Result;
use crate::storage::{Database, Metadata};

/// Initialize a new itack project in the current git repository.
/// If already initialized, repairs/recreates the database.
pub fn run() -> Result<()> {
    let repo_root = Project::find_repo_root()?;

    if Project::is_initialized(&repo_root) {
        // Already initialized - repair the database
        return repair_database();
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

    // Open database to initialize it (use open_or_create for init)
    let project = Project::discover()?;
    let data_branch = project.config.data_branch.as_deref();
    let _db = Database::open_or_create(&project.db_path, Some(&project.repo_root), data_branch)?;

    println!("Initialized itack project: {}", metadata.project_id);
    println!("Issues will be stored in: .itack/");

    Ok(())
}

/// Repair/recreate the database for an existing project.
fn repair_database() -> Result<()> {
    let project = Project::discover()?;

    // Use open_or_create to ensure directory and DB exist
    let data_branch = project.config.data_branch.as_deref();
    let mut db = Database::open_or_create(&project.db_path, Some(&project.repo_root), data_branch)?;

    // Always repair state to sync with data branch
    db.repair_state()?;

    println!(
        "Repaired database for project: {}",
        project.metadata.project_id
    );

    Ok(())
}

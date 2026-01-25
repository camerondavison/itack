//! itack init command.

use std::fs;

use crate::core::{Config, Project};
use crate::error::Result;
use crate::storage::{markdown, Database, Metadata};

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
    let _db = Database::open_or_create(&project.db_path, &project.itack_dir)?;

    println!("Initialized itack project: {}", metadata.project_id);
    println!("Issues will be stored in: .itack/");

    Ok(())
}

/// Repair/recreate the database for an existing project.
fn repair_database() -> Result<()> {
    let project = Project::discover()?;

    // Migrate issue filenames to new format before repairing the database
    migrate_issue_filenames(&project)?;

    // Use open_or_create to ensure directory and DB exist
    let mut db = Database::open_or_create(&project.db_path, &project.itack_dir)?;

    // Always repair state to sync with issue files
    db.repair_state()?;

    println!(
        "Repaired database for project: {}",
        project.metadata.project_id
    );

    Ok(())
}

/// Migrate issue files from old format (N.md) to new format (YYYY-MM-DD-issue-NNN.md).
fn migrate_issue_filenames(project: &Project) -> Result<()> {
    if !project.itack_dir.exists() {
        return Ok(());
    }

    let mut files_to_rename = Vec::new();

    for entry in fs::read_dir(&project.itack_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let filename = path.file_stem().unwrap_or_default().to_string_lossy();

            // Check if it's old format (just a number) or already new format
            if filename.parse::<u32>().is_ok() {
                // Old format - needs migration
                if let Ok((issue, body)) = markdown::read_issue(&path) {
                    let new_path = project.issue_path_with_date(issue.id, &issue.created);
                    if new_path != path {
                        files_to_rename.push((path, new_path, issue, body));
                    }
                }
            }
        }
    }

    // Rename files
    for (old_path, new_path, issue, body) in files_to_rename {
        // Write to new path first, then delete old
        markdown::write_issue(&new_path, &issue, &body)?;
        fs::remove_file(&old_path)?;
        println!(
            "Migrated: {} -> {}",
            old_path.file_name().unwrap_or_default().to_string_lossy(),
            new_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    Ok(())
}

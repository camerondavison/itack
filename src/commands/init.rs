//! itack init command.

use std::fs;
use std::path::Path;

use crate::core::{Config, Project, commit_to_branch, find_issue_in_branch};
use crate::error::Result;
use crate::storage::{Database, Metadata, markdown};

/// Initialize a new itack project in the current git repository.
/// If already initialized, repairs/recreates the database.
pub fn run() -> Result<()> {
    let repo_root = Project::find_repo_root()?;

    if Project::is_initialized(&repo_root) {
        // Already initialized - repair the database
        repair_database()?;

        let project = Project::discover()?;
        let data_branch = project
            .config
            .data_branch
            .as_deref()
            .unwrap_or("data/itack");
        migrate_stray_issues(&project.repo_root, data_branch)?;
        return Ok(());
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

    let db_data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");
    migrate_stray_issues(&project.repo_root, db_data_branch)?;

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

/// Find and migrate stray .itack/*.md issue files from the working directory to the data branch.
fn migrate_stray_issues(repo_root: &Path, data_branch: &str) -> Result<()> {
    let itack_dir = repo_root.join(".itack");
    if !itack_dir.is_dir() {
        return Ok(());
    }

    let mut migrated = 0u32;
    let mut removed = 0u32;

    for entry in fs::read_dir(&itack_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Only process valid itack issue files
        let (issue, title, body) = match markdown::parse_issue(&content) {
            Ok(result) => result,
            Err(_) => continue,
        };

        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let relative_path = std::path::PathBuf::from(".itack").join(&filename);

        // Migrate to data branch if not already there
        if find_issue_in_branch(repo_root, data_branch, issue.id)?.is_none() {
            let formatted = markdown::format_issue(&issue, &title, &body)?;
            commit_to_branch(
                repo_root,
                data_branch,
                &relative_path,
                formatted.as_bytes(),
                &format!("Migrate issue #{} from working directory", issue.id),
            )?;
            migrated += 1;
        }

        // Remove from working directory
        fs::remove_file(&path)?;
        removed += 1;
    }

    if removed > 0 {
        println!(
            "Cleaned up {} stray issue file(s) from working directory ({} migrated to data branch).",
            removed, migrated
        );
    }

    Ok(())
}

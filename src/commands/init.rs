//! itack init command.

use std::fs;

use crate::core::{Config, Project, commit_to_branch};
use crate::error::Result;
use crate::storage::{Database, Metadata, markdown};

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
    let _db = Database::open_or_create(
        &project.db_path,
        &project.itack_dir,
        Some(&project.repo_root),
        data_branch,
    )?;

    println!("Initialized itack project: {}", metadata.project_id);
    println!("Issues will be stored in: .itack/");

    Ok(())
}

/// Repair/recreate the database for an existing project.
fn repair_database() -> Result<()> {
    let project = Project::discover()?;

    // Migrate issue filenames to new format before repairing the database
    migrate_issue_filenames(&project)?;

    // Migrate issues to add title headings
    migrate_title_headings(&project)?;

    // Migrate issues to data branch
    migrate_to_data_branch(&project)?;

    // Use open_or_create to ensure directory and DB exist
    let data_branch = project.config.data_branch.as_deref();
    let mut db = Database::open_or_create(
        &project.db_path,
        &project.itack_dir,
        Some(&project.repo_root),
        data_branch,
    )?;

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
                if let Ok((issue, title, body)) = markdown::read_issue(&path) {
                    let new_path = project.issue_path_with_date(issue.id, &issue.created);
                    if new_path != path {
                        files_to_rename.push((path, new_path, issue, title, body));
                    }
                }
            }
        }
    }

    // Rename files
    for (old_path, new_path, issue, title, body) in files_to_rename {
        // Write to new path first, then delete old
        markdown::write_issue(&new_path, &issue, &title, &body)?;
        fs::remove_file(&old_path)?;
        println!(
            "Migrated: {} -> {}",
            old_path.file_name().unwrap_or_default().to_string_lossy(),
            new_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    Ok(())
}

/// Migrate issue files to add title headings after frontmatter.
/// Also removes title from YAML front matter if present.
fn migrate_title_headings(project: &Project) -> Result<()> {
    if !project.itack_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&project.itack_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let content = fs::read_to_string(&path)?;

            // Check if file needs migration (has title in YAML or missing title heading)
            let needs_migration =
                content.contains("\ntitle:") || !markdown::has_title_heading(&content);

            if needs_migration {
                // Try to read with old format (title in YAML)
                if let Some((issue, title, body)) = try_read_old_format(&content) {
                    markdown::write_issue(&path, &issue, &title, &body)?;
                    println!("Migrated issue #{} (removed title from YAML)", issue.id);
                } else if let Ok((issue, title, body)) = markdown::read_issue(&path) {
                    // Already in new format, just re-write to ensure consistency
                    markdown::write_issue(&path, &issue, &title, &body)?;
                }
            }
        }
    }

    Ok(())
}

/// Migrate issues from working directory to the data branch.
fn migrate_to_data_branch(project: &Project) -> Result<()> {
    use crate::core::find_issue_in_branch;

    if !project.itack_dir.exists() {
        return Ok(());
    }

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    let mut migrated = 0;

    for entry in fs::read_dir(&project.itack_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false)
            && let Ok((issue, _, _)) = markdown::read_issue(&path)
        {
            // Check if this issue already exists in the data branch
            let exists_in_data_branch = matches!(
                find_issue_in_branch(&project.repo_root, data_branch, issue.id),
                Ok(Some(_))
            );

            if !exists_in_data_branch {
                // Read the file content and commit to data branch
                let content = fs::read(&path)?;
                let relative_path = path
                    .strip_prefix(&project.repo_root)
                    .unwrap_or(&path)
                    .to_path_buf();

                let message = format!("Migrate issue #{} to data branch", issue.id);
                commit_to_branch(
                    &project.repo_root,
                    data_branch,
                    &relative_path,
                    &content,
                    &message,
                )?;

                println!(
                    "Migrated issue #{} to data branch '{}'",
                    issue.id, data_branch
                );
                migrated += 1;
            }
        }
    }

    if migrated > 0 {
        println!(
            "Migrated {} issues to data branch '{}'",
            migrated, data_branch
        );
    }

    Ok(())
}

/// Try to read an issue file in the old format (with title in YAML).
/// Returns None if the file doesn't have the old format.
fn try_read_old_format(content: &str) -> Option<(crate::core::Issue, String, String)> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct OldIssue {
        #[serde(default)]
        assignee: Option<String>,
        created: chrono::DateTime<chrono::Utc>,
        #[serde(default)]
        epic: Option<String>,
        id: u32,
        status: crate::core::Status,
        title: String, // Old format has title in YAML
    }

    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("---")?;

    let yaml_content = &after_first[..end_pos];
    let body_start = 3 + end_pos + 3;
    let body = content[body_start..].trim_start_matches('\n');

    // Try to parse as old format
    let old_issue: OldIssue = serde_yaml::from_str(yaml_content).ok()?;

    // Convert to new Issue format
    let mut issue = crate::core::Issue::new(old_issue.id);
    issue.assignee = old_issue.assignee;
    issue.created = old_issue.created;
    issue.epic = old_issue.epic;
    issue.status = old_issue.status;

    // Extract body (strip title heading if present)
    let expected_heading = format!("# {}", old_issue.title);
    let body = if let Some(rest) = body.strip_prefix(&expected_heading) {
        rest.trim_start_matches('\n').to_string()
    } else {
        body.to_string()
    };

    Some((issue, old_issue.title, body))
}

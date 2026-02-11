//! itack doctor command - diagnose database and issue sync issues.

use std::collections::HashSet;
use std::fs;

use crate::core::{Project, Status};
use crate::error::Result;
use crate::storage::Database;
use crate::storage::db::load_all_issues_from_data_branch;
use crate::storage::markdown;

/// Expected schema version (must match SCHEMA_VERSION in db.rs).
const EXPECTED_SCHEMA_VERSION: i32 = 1;

/// Run diagnostics on the itack database and issue files.
pub fn run() -> Result<()> {
    let project = Project::discover()?;
    let mut has_issues = false;

    let data_branch = project
        .config
        .data_branch
        .as_deref()
        .unwrap_or("data/itack");

    // Check 1: Database schema version
    println!("Checking database schema version...");
    match check_schema_version(&project) {
        Ok(db_version) => {
            if db_version == EXPECTED_SCHEMA_VERSION {
                println!("  ✓ Database schema version: {} (matches CLI)", db_version);
            } else {
                println!(
                    "  ✗ Database schema version mismatch: DB has {}, CLI expects {}",
                    db_version, EXPECTED_SCHEMA_VERSION
                );
                println!("    Run 'itack init' to repair the database.");
                has_issues = true;
            }
        }
        Err(e) => {
            println!("  ✗ Could not read database schema version: {}", e);
            println!("    Run 'itack init' to repair the database.");
            has_issues = true;
        }
    }

    // Check 2: Compare issues in data branch vs database knowledge
    println!("\nChecking issue synchronization...");
    match check_issue_sync(&project, data_branch) {
        Ok(sync_result) => {
            if sync_result.is_ok() {
                println!(
                    "  ✓ Issues in sync: {} issues found in '{}'",
                    sync_result.issue_count, data_branch
                );
            } else {
                has_issues = true;
                if !sync_result.missing_claims.is_empty() {
                    println!(
                        "  ✗ In-progress issues without database claims: {:?}",
                        sync_result.missing_claims
                    );
                }
                if !sync_result.orphan_claims.is_empty() {
                    println!(
                        "  ✗ Claims in database for non-existent issues: {:?}",
                        sync_result.orphan_claims
                    );
                }
                if let Some(msg) = &sync_result.next_id_issue {
                    println!("  ✗ {}", msg);
                }
                println!("    Run 'itack init' to repair the database.");
            }
        }
        Err(e) => {
            println!("  ✗ Could not check issue synchronization: {}", e);
            has_issues = true;
        }
    }

    // Check 3: Stray issue files in working directory
    println!("\nChecking for stray issue files in working directory...");
    match find_stray_issue_files(&project) {
        Ok(stray_files) => {
            if stray_files.is_empty() {
                println!("  ✓ No stray issue files found");
            } else {
                has_issues = true;
                println!(
                    "  ✗ Found {} stray issue file(s) in .itack/:",
                    stray_files.len()
                );
                for file in &stray_files {
                    println!("    - {}", file);
                }
                println!("    Run 'itack init' to migrate them to the data branch.");
            }
        }
        Err(e) => {
            println!("  ✗ Could not check for stray files: {}", e);
            has_issues = true;
        }
    }

    // Summary
    println!();
    if has_issues {
        println!("Issues found. Run 'itack init' to repair.");
        std::process::exit(1);
    } else {
        println!("All checks passed.");
    }

    Ok(())
}

/// Find stray .itack/*.md issue files in the working directory.
fn find_stray_issue_files(project: &Project) -> Result<Vec<String>> {
    let itack_dir = project.repo_root.join(".itack");
    if !itack_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut stray_files = Vec::new();
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

        if markdown::parse_issue(&content).is_ok() {
            stray_files.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }

    stray_files.sort();
    Ok(stray_files)
}

/// Check the database schema version.
fn check_schema_version(project: &Project) -> Result<i32> {
    let data_branch = project.config.data_branch.as_deref();
    let db = Database::open(&project.db_path, Some(&project.repo_root), data_branch)?;
    db.get_schema_version()
}

/// Result of checking issue synchronization.
struct SyncCheckResult {
    issue_count: usize,
    missing_claims: Vec<u32>,
    orphan_claims: Vec<u32>,
    next_id_issue: Option<String>,
}

impl SyncCheckResult {
    fn is_ok(&self) -> bool {
        self.missing_claims.is_empty()
            && self.orphan_claims.is_empty()
            && self.next_id_issue.is_none()
    }
}

/// Check if issues in data branch match what the database knows about.
fn check_issue_sync(project: &Project, data_branch: &str) -> Result<SyncCheckResult> {
    let db = Database::open(
        &project.db_path,
        Some(&project.repo_root),
        Some(data_branch),
    )?;

    // Get all issues from data branch
    let issues = load_all_issues_from_data_branch(&project.repo_root, data_branch)?;

    let issue_ids: HashSet<u32> = issues.iter().map(|i| i.issue.id).collect();
    let max_issue_id = issues.iter().map(|i| i.issue.id).max().unwrap_or(0);

    // Get claims from database
    let claims = db.list_claims()?;
    let claimed_ids: HashSet<u32> = claims.iter().map(|(id, _, _)| *id).collect();

    // Find in-progress issues that don't have claims in the database
    let mut missing_claims: Vec<u32> = issues
        .iter()
        .filter(|i| i.issue.status == Status::InProgress && !claimed_ids.contains(&i.issue.id))
        .map(|i| i.issue.id)
        .collect();
    missing_claims.sort();

    // Find claims for issues that don't exist
    let mut orphan_claims: Vec<u32> = claimed_ids.difference(&issue_ids).copied().collect();
    orphan_claims.sort();

    // Check next_issue_id
    let next_id = db.peek_next_issue_id()?;
    let next_id_issue = if !issues.is_empty() && next_id <= max_issue_id {
        Some(format!(
            "next_issue_id ({}) is not greater than max issue ID ({})",
            next_id, max_issue_id
        ))
    } else {
        None
    };

    Ok(SyncCheckResult {
        issue_count: issues.len(),
        missing_claims,
        orphan_claims,
        next_id_issue,
    })
}

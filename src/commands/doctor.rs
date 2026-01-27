//! itack doctor command - diagnose database and issue sync issues.

use std::collections::HashSet;
use std::fs;

use crate::core::{Project, Status};
use crate::error::Result;
use crate::storage::{Database, markdown};

/// Expected schema version (must match SCHEMA_VERSION in db.rs).
const EXPECTED_SCHEMA_VERSION: i32 = 1;

/// Run diagnostics on the itack database and issue files.
pub fn run() -> Result<()> {
    let project = Project::discover()?;
    let mut has_issues = false;

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

    // Check 2: Compare issues in repo vs database knowledge
    println!("\nChecking issue synchronization...");
    match check_issue_sync(&project) {
        Ok(sync_result) => {
            if sync_result.is_ok() {
                println!(
                    "  ✓ Issues in sync: {} issues found",
                    sync_result.repo_issues.len()
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
                if let Some(msg) = sync_result.next_id_issue {
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

    // Check 3: Issues missing title heading
    println!("\nChecking issue markdown format...");
    match check_title_headings(&project) {
        Ok(missing) => {
            if missing.is_empty() {
                println!("  ✓ All issues have title headings");
            } else {
                println!("  ✗ Issues missing title heading: {:?}", missing);
                println!("    Run 'itack init' to repair.");
                has_issues = true;
            }
        }
        Err(e) => {
            println!("  ✗ Could not check issue format: {}", e);
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

/// Check for issues missing the title heading in markdown.
fn check_title_headings(project: &Project) -> Result<Vec<u32>> {
    let mut missing = Vec::new();

    if !project.itack_dir.exists() {
        return Ok(missing);
    }

    for entry in fs::read_dir(&project.itack_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            let content = fs::read_to_string(&path)?;
            if !markdown::has_title_heading(&content)
                && let Ok((issue, _, _)) = markdown::parse_issue(&content) {
                    missing.push(issue.id);
                }
        }
    }

    missing.sort();
    Ok(missing)
}

/// Check the database schema version.
fn check_schema_version(project: &Project) -> Result<i32> {
    let db = Database::open(&project.db_path, &project.itack_dir)?;
    db.get_schema_version()
}

/// Result of checking issue synchronization.
struct SyncCheckResult {
    repo_issues: HashSet<u32>,
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

/// Check if issues in repo match what the database knows about.
fn check_issue_sync(project: &Project) -> Result<SyncCheckResult> {
    let db = Database::open(&project.db_path, &project.itack_dir)?;

    // Get all issue IDs from the repo
    let mut repo_issues: HashSet<u32> = HashSet::new();
    let mut max_repo_id: u32 = 0;

    if project.itack_dir.exists() {
        for entry in fs::read_dir(&project.itack_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "md").unwrap_or(false)
                && let Ok((issue, _, _)) = markdown::read_issue(&path) {
                    repo_issues.insert(issue.id);
                    max_repo_id = max_repo_id.max(issue.id);
                }
        }
    }

    // Get claims from database
    let claims = db.list_claims()?;
    let claimed_ids: HashSet<u32> = claims.iter().map(|(id, _, _)| *id).collect();

    // Find in-progress issues that don't have claims in the database
    // (only check in-progress since done issues may have stale assignees)
    let mut missing_claims = Vec::new();
    for entry in fs::read_dir(&project.itack_dir)
        .into_iter()
        .flatten()
        .flatten()
    {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false)
            && let Ok((issue, _, _)) = markdown::read_issue(&path)
                && issue.status == Status::InProgress && !claimed_ids.contains(&issue.id) {
                    missing_claims.push(issue.id);
                }
    }
    missing_claims.sort();

    // Find claims for issues that don't exist in repo
    let mut orphan_claims: Vec<u32> = claimed_ids.difference(&repo_issues).copied().collect();
    orphan_claims.sort();

    // Check next_issue_id
    let next_id = db.peek_next_issue_id()?;
    let next_id_issue = if !repo_issues.is_empty() && next_id <= max_repo_id {
        Some(format!(
            "next_issue_id ({}) is not greater than max issue ID in repo ({})",
            next_id, max_repo_id
        ))
    } else {
        None
    };

    Ok(SyncCheckResult {
        repo_issues,
        missing_claims,
        orphan_claims,
        next_id_issue,
    })
}

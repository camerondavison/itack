//! SQLite database: schema, claims, state, create/rebuild.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::fs;
use std::path::Path;

use crate::core::Issue;
use crate::error::{ItackError, Result};
use crate::storage::markdown;

/// Current schema version.
const SCHEMA_VERSION: i32 = 1;

/// SQLite database handle for itack.
pub struct Database {
    conn: Connection,
    issues_dir: std::path::PathBuf,
}

impl Database {
    /// Open or create the database at the given path.
    ///
    /// The parent directory must already exist. Use `open_or_create` if you want
    /// to create the directory as well.
    pub fn open(db_path: &Path, issues_dir: &Path) -> Result<Self> {
        // Check parent directory exists - don't auto-create
        if let Some(parent) = db_path.parent()
            && !parent.exists()
        {
            return Err(ItackError::DatabaseNotFound(db_path.to_path_buf()));
        }

        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let mut db = Database {
            conn,
            issues_dir: issues_dir.to_path_buf(),
        };

        db.ensure_schema()?;
        Ok(db)
    }

    /// Open or create the database, creating the parent directory if needed.
    /// Use this for `init` command only.
    pub fn open_or_create(db_path: &Path, issues_dir: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        let mut db = Database {
            conn,
            issues_dir: issues_dir.to_path_buf(),
        };

        db.ensure_schema()?;
        Ok(db)
    }

    /// Ensure the schema is up to date, rebuilding if necessary.
    fn ensure_schema(&mut self) -> Result<()> {
        // Check if schema_version table exists
        let has_schema: bool = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
                [],
                |row| row.get::<_, i32>(0),
            )
            .map(|c| c > 0)?;

        if !has_schema {
            self.create_or_rebuild()?;
            return Ok(());
        }

        // Check version
        let version: i32 =
            self.conn
                .query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;

        if version != SCHEMA_VERSION {
            self.create_or_rebuild()?;
        }

        Ok(())
    }

    /// Create the schema or rebuild it from markdown files.
    pub fn create_or_rebuild(&mut self) -> Result<()> {
        // Use EXCLUSIVE transaction for rebuild
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Exclusive)?;

        // Re-check version inside transaction (double-check locking)
        let version: i32 = tx
            .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
            .unwrap_or(0);

        if version == SCHEMA_VERSION {
            // Another process already rebuilt
            tx.commit()?;
            return Ok(());
        }

        // Drop and recreate tables
        tx.execute_batch(
            r#"
            DROP TABLE IF EXISTS claims;
            DROP TABLE IF EXISTS state;
            DROP TABLE IF EXISTS schema_version;

            CREATE TABLE schema_version (
                version INTEGER NOT NULL
            );

            CREATE TABLE state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                next_issue_id INTEGER NOT NULL DEFAULT 1
            );

            CREATE TABLE claims (
                issue_id INTEGER PRIMARY KEY,
                assignee TEXT NOT NULL,
                claimed_at TEXT NOT NULL
            );

            INSERT INTO schema_version (version) VALUES (1);
            "#,
        )?;

        // Scan .itack/*.md files to rebuild state
        let mut max_id: u32 = 0;

        if self.issues_dir.exists() {
            for entry in fs::read_dir(&self.issues_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map(|e| e == "md").unwrap_or(false)
                    && let Ok((issue, _, _)) = markdown::read_issue(&path)
                {
                    max_id = max_id.max(issue.id);

                    // Rebuild claims from assignee field
                    if let Some(assignee) = &issue.assignee {
                        tx.execute(
                                "INSERT OR REPLACE INTO claims (issue_id, assignee, claimed_at) VALUES (?1, ?2, ?3)",
                                params![issue.id, assignee, issue.created.to_rfc3339()],
                            )?;
                    }
                }
            }
        }

        // Set next_issue_id to max + 1
        tx.execute(
            "INSERT INTO state (id, next_issue_id) VALUES (1, ?1)",
            params![max_id + 1],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Repair state tables (claims and next_issue_id) by rescanning issue files.
    /// Unlike create_or_rebuild, this always runs regardless of schema version.
    pub fn repair_state(&mut self) -> Result<()> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Exclusive)?;

        // Clear and rebuild state tables
        tx.execute("DELETE FROM claims", [])?;
        tx.execute("DELETE FROM state", [])?;

        // Scan .itack/*.md files to rebuild state
        let mut max_id: u32 = 0;

        if self.issues_dir.exists() {
            for entry in fs::read_dir(&self.issues_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().map(|e| e == "md").unwrap_or(false)
                    && let Ok((issue, _, _)) = markdown::read_issue(&path)
                {
                    max_id = max_id.max(issue.id);

                    // Rebuild claims from assignee field
                    if let Some(assignee) = &issue.assignee {
                        tx.execute(
                                "INSERT OR REPLACE INTO claims (issue_id, assignee, claimed_at) VALUES (?1, ?2, ?3)",
                                params![issue.id, assignee, issue.created.to_rfc3339()],
                            )?;
                    }
                }
            }
        }

        // Set next_issue_id to max + 1
        tx.execute(
            "INSERT INTO state (id, next_issue_id) VALUES (1, ?1)",
            params![max_id + 1],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Atomically get and increment the next issue ID.
    pub fn next_issue_id(&self) -> Result<u32> {
        let id: u32 = self.conn.query_row(
            "UPDATE state SET next_issue_id = next_issue_id + 1 WHERE id = 1 RETURNING next_issue_id - 1",
            [],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Get the current next_issue_id without incrementing.
    pub fn peek_next_issue_id(&self) -> Result<u32> {
        let id: u32 =
            self.conn
                .query_row("SELECT next_issue_id FROM state WHERE id = 1", [], |row| {
                    row.get(0)
                })?;
        Ok(id)
    }

    /// Attempt to claim an issue. Returns error if already claimed.
    pub fn claim(&mut self, issue_id: u32, assignee: &str) -> Result<()> {
        // Use IMMEDIATE transaction for write intent
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        // Check if already claimed
        let existing: Option<String> = tx
            .query_row(
                "SELECT assignee FROM claims WHERE issue_id = ?1",
                params![issue_id],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(existing_assignee) = existing {
            return Err(ItackError::AlreadyClaimed(issue_id, existing_assignee));
        }

        // Insert claim
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "INSERT INTO claims (issue_id, assignee, claimed_at) VALUES (?1, ?2, ?3)",
            params![issue_id, assignee, now],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Release a claim on an issue.
    pub fn release(&mut self, issue_id: u32) -> Result<()> {
        let rows = self
            .conn
            .execute("DELETE FROM claims WHERE issue_id = ?1", params![issue_id])?;

        if rows == 0 {
            return Err(ItackError::NotClaimed(issue_id));
        }

        Ok(())
    }

    /// Check if an issue is claimed and by whom.
    #[allow(dead_code)]
    pub fn get_claim(&self, issue_id: u32) -> Result<Option<(String, DateTime<Utc>)>> {
        let result: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT assignee, claimed_at FROM claims WHERE issue_id = ?1",
                params![issue_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        match result {
            Some((assignee, claimed_at_str)) => {
                let claimed_at = DateTime::parse_from_rfc3339(&claimed_at_str).map_err(|e| {
                    ItackError::Other(format!("Invalid claimed_at timestamp: {}", e))
                })?;
                Ok(Some((assignee, claimed_at.with_timezone(&Utc))))
            }
            None => Ok(None),
        }
    }

    /// Get the current schema version from the database.
    pub fn get_schema_version(&self) -> Result<i32> {
        let version: i32 =
            self.conn
                .query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;
        Ok(version)
    }

    /// Get all claims.
    pub fn list_claims(&self) -> Result<Vec<(u32, String, DateTime<Utc>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT issue_id, assignee, claimed_at FROM claims")?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut claims = Vec::new();
        for row in rows {
            let (issue_id, assignee, claimed_at_str) = row?;
            let claimed_at = DateTime::parse_from_rfc3339(&claimed_at_str)
                .map_err(|e| ItackError::Other(format!("Invalid claimed_at timestamp: {}", e)))?;
            claims.push((issue_id, assignee, claimed_at.with_timezone(&Utc)));
        }

        Ok(claims)
    }
}

/// Information about a loaded issue.
#[derive(Clone)]
pub struct IssueInfo {
    pub issue: Issue,
    pub title: String,
    pub body: String,
    pub path: std::path::PathBuf,
}

/// Load all issues from the data branch.
pub fn load_all_issues_from_data_branch(
    repo_root: &Path,
    data_branch: &str,
) -> Result<Vec<IssueInfo>> {
    use crate::core::read_file_from_branch;
    use git2::Repository;

    let mut issues = Vec::new();

    let repo = match Repository::discover(repo_root) {
        Ok(r) => r,
        Err(_) => return Ok(issues),
    };

    // Find the branch
    let branch_ref = format!("refs/heads/{}", data_branch);
    let reference = match repo.find_reference(&branch_ref) {
        Ok(r) => r,
        Err(_) => return Ok(issues), // Branch doesn't exist yet
    };

    let commit = reference.peel_to_commit()?;
    let tree = commit.tree()?;

    // Look for .itack directory
    let itack_entry = match tree.get_name(".itack") {
        Some(entry) => entry,
        None => return Ok(issues),
    };

    let itack_tree = repo.find_tree(itack_entry.id())?;

    // Iterate over all .md files in .itack/
    for entry in itack_tree.iter() {
        if let Some(name) = entry.name()
            && name.ends_with(".md")
            && !name.starts_with('.')
        {
            let relative_path = std::path::PathBuf::from(".itack").join(name);
            if let Some(content) = read_file_from_branch(repo_root, data_branch, &relative_path)?
                && let Ok(content_str) = String::from_utf8(content)
                && let Ok((issue, title, body)) = markdown::parse_issue(&content_str)
            {
                issues.push(IssueInfo {
                    issue,
                    title,
                    body,
                    path: repo_root.join(&relative_path),
                });
            }
        }
    }

    // Sort by status priority, then by ID
    issues.sort_by(|a, b| {
        let status_cmp = a
            .issue
            .status
            .sort_priority()
            .cmp(&b.issue.status.sort_priority());
        if status_cmp == std::cmp::Ordering::Equal {
            a.issue.id.cmp(&b.issue.id)
        } else {
            status_cmp
        }
    });

    Ok(issues)
}

/// Load all issues from the issues directory (working directory).
/// Note: Prefer `load_all_issues_from_data_branch` to get the latest version.
#[allow(dead_code)]
pub fn load_all_issues(issues_dir: &Path) -> Result<Vec<IssueInfo>> {
    let mut issues = Vec::new();

    if !issues_dir.exists() {
        return Ok(issues);
    }

    for entry in fs::read_dir(issues_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            match markdown::read_issue(&path) {
                Ok((issue, title, body)) => {
                    issues.push(IssueInfo {
                        issue,
                        title,
                        body,
                        path,
                    });
                }
                Err(_) => {
                    // Skip invalid files
                    continue;
                }
            }
        }
    }

    // Sort by status priority, then by ID
    issues.sort_by(|a, b| {
        let status_cmp = a
            .issue
            .status
            .sort_priority()
            .cmp(&b.issue.status.sort_priority());
        if status_cmp == std::cmp::Ordering::Equal {
            a.issue.id.cmp(&b.issue.id)
        } else {
            status_cmp
        }
    });

    Ok(issues)
}

/// Load a single issue by ID from the data branch and sync to working directory.
/// This is the preferred method for loading issues when you need the latest version.
/// The data branch is the source of truth for issue content.
pub fn load_issue_from_data_branch(
    repo_root: &Path,
    _issues_dir: &Path,
    data_branch: &str,
    id: u32,
) -> Result<IssueInfo> {
    use crate::core::{find_issue_in_branch, read_file_from_branch};

    // Find the issue file in the data branch
    let relative_path =
        find_issue_in_branch(repo_root, data_branch, id)?.ok_or(ItackError::IssueNotFound(id))?;

    // Read content from data branch
    let content = read_file_from_branch(repo_root, data_branch, &relative_path)?
        .ok_or(ItackError::IssueNotFound(id))?;

    let path = repo_root.join(&relative_path);

    // Ensure parent directory exists and write to working directory
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &content)?;

    // Parse the issue
    let content_str = String::from_utf8(content)
        .map_err(|e| ItackError::Other(format!("Invalid UTF-8: {}", e)))?;
    let (issue, title, body) = markdown::parse_issue(&content_str)?;

    Ok(IssueInfo {
        issue,
        title,
        body,
        path,
    })
}

/// Load a single issue by ID from the working directory.
/// Checks both new format (YYYY-MM-DD-issue-NNN.md) and old format (N.md).
/// Note: Prefer `load_issue_from_data_branch` to get the latest version.
#[allow(dead_code)]
pub fn load_issue(issues_dir: &Path, id: u32) -> Result<IssueInfo> {
    // Check for new format files first (pattern: *-issue-{id:03}.md)
    let suffix = format!("-issue-{:03}.md", id);
    if let Ok(entries) = fs::read_dir(issues_dir) {
        for entry in entries.flatten() {
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();
            if filename_str.ends_with(&suffix) {
                let path = entry.path();
                let (issue, title, body) = markdown::read_issue(&path)?;
                return Ok(IssueInfo {
                    issue,
                    title,
                    body,
                    path,
                });
            }
        }
    }

    // Fall back to old format
    let path = issues_dir.join(format!("{}.md", id));
    if !path.exists() {
        return Err(ItackError::IssueNotFound(id));
    }

    let (issue, title, body) = markdown::read_issue(&path)?;
    Ok(IssueInfo {
        issue,
        title,
        body,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("itack.db");
        let issues_dir = dir.path().join(".itack");
        fs::create_dir_all(&issues_dir).unwrap();
        let db = Database::open(&db_path, &issues_dir).unwrap();
        (dir, db)
    }

    #[test]
    fn test_next_issue_id() {
        let (_dir, db) = setup_test_db();

        assert_eq!(db.next_issue_id().unwrap(), 1);
        assert_eq!(db.next_issue_id().unwrap(), 2);
        assert_eq!(db.next_issue_id().unwrap(), 3);
        assert_eq!(db.peek_next_issue_id().unwrap(), 4);
    }

    #[test]
    fn test_claim_and_release() {
        let (_dir, mut db) = setup_test_db();

        // Claim should succeed
        db.claim(1, "agent-1").unwrap();
        assert_eq!(
            db.get_claim(1).unwrap().map(|(a, _)| a),
            Some("agent-1".to_string())
        );

        // Second claim should fail
        let err = db.claim(1, "agent-2").unwrap_err();
        assert!(matches!(err, ItackError::AlreadyClaimed(1, _)));

        // Release should succeed
        db.release(1).unwrap();
        assert!(db.get_claim(1).unwrap().is_none());

        // Now agent-2 can claim
        db.claim(1, "agent-2").unwrap();
        assert_eq!(
            db.get_claim(1).unwrap().map(|(a, _)| a),
            Some("agent-2".to_string())
        );
    }

    #[test]
    fn test_release_unclaimed() {
        let (_dir, mut db) = setup_test_db();

        let err = db.release(1).unwrap_err();
        assert!(matches!(err, ItackError::NotClaimed(1)));
    }
}

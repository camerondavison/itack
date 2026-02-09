//! Project context (paths, project_id).

use git2::Repository;
use std::path::{Path, PathBuf};

use crate::core::Config;
use crate::error::{ItackError, Result};
use crate::storage::{Database, Metadata};

/// Project context for itack operations.
pub struct Project {
    /// Root directory of the git repository.
    pub repo_root: PathBuf,
    /// Path to .itack directory.
    #[allow(dead_code)]
    pub itack_dir: PathBuf,
    /// Path to the SQLite database (in global config dir).
    pub db_path: PathBuf,
    /// Project metadata.
    pub metadata: Metadata,
    /// Global config.
    pub config: Config,
}

impl Project {
    /// Find and load the project context from the current directory.
    pub fn discover() -> Result<Self> {
        let repo_root = Self::find_repo_root()?;
        let itack_dir = repo_root.join(".itack");

        if !itack_dir.exists() {
            return Err(ItackError::NotInitialized);
        }

        let metadata_path = itack_dir.join("metadata.toml");
        if !metadata_path.exists() {
            return Err(ItackError::NotInitialized);
        }

        let metadata = Metadata::load(&metadata_path)?;
        let config = Config::load_global()?;

        // Database is stored in global config dir
        let db_path = Self::db_path_for_project(&metadata.project_id)?;

        Ok(Project {
            repo_root,
            itack_dir,
            db_path,
            metadata,
            config,
        })
    }

    /// Find the git repository root.
    pub fn find_repo_root() -> Result<PathBuf> {
        let current = std::env::current_dir()?;
        let repo = Repository::discover(&current).map_err(|_| ItackError::NotInGitRepo)?;

        repo.workdir()
            .map(|p| p.to_path_buf())
            .ok_or(ItackError::NotInGitRepo)
    }

    /// Get the database path for a project ID.
    fn db_path_for_project(project_id: &str) -> Result<PathBuf> {
        let global_dir = Config::global_dir()
            .ok_or_else(|| ItackError::Other("Could not determine home directory".to_string()))?;

        Ok(global_dir.join(format!("{}.db", project_id)))
    }

    /// Check if a project is initialized at the given path.
    pub fn is_initialized(repo_root: &Path) -> bool {
        let itack_dir = repo_root.join(".itack");
        let metadata_path = itack_dir.join("metadata.toml");
        itack_dir.exists() && metadata_path.exists()
    }

    /// Open the database for this project.
    pub fn open_db(&self) -> Result<Database> {
        let data_branch = self.config.data_branch.as_deref();
        Database::open(&self.db_path, Some(&self.repo_root), data_branch)
    }

    /// Get the relative path to an issue file (e.g. `.itack/2026-01-25-issue-001.md`).
    /// Requires the creation date to generate the filename.
    pub fn issue_relative_path(id: u32, created: &chrono::DateTime<chrono::Utc>) -> PathBuf {
        let date_str = created.format("%Y-%m-%d").to_string();
        PathBuf::from(".itack").join(format!("{}-issue-{:03}.md", date_str, id))
    }

    /// Get the current git branch name.
    /// Returns None if in a detached HEAD state or if there's no HEAD (unborn branch).
    pub fn current_branch(&self) -> Option<String> {
        let repo = Repository::open(&self.repo_root).ok()?;
        let head = repo.head().ok()?;
        head.shorthand().map(|s| s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_initialized() {
        use std::fs;
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        assert!(!Project::is_initialized(dir.path()));

        let itack_dir = dir.path().join(".itack");
        fs::create_dir_all(&itack_dir).unwrap();
        assert!(!Project::is_initialized(dir.path()));

        let metadata = Metadata::new();
        metadata.save(&itack_dir.join("metadata.toml")).unwrap();
        assert!(Project::is_initialized(dir.path()));
    }
}

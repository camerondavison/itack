//! Project metadata.toml parsing.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::Result;

/// Project metadata stored in .itack/metadata.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    /// Unique project identifier (petname-based).
    pub project_id: String,
}

impl Metadata {
    /// Create new metadata with a random project ID.
    pub fn new() -> Self {
        use petname::Generator;
        use rand::SeedableRng;

        let mut rng = rand::rngs::StdRng::from_entropy();
        let generator = petname::Petnames::large();
        let project_id = generator
            .generate(&mut rng, 3, "-")
            .unwrap_or_else(|| "unnamed-project".to_string());

        Metadata { project_id }
    }

    /// Load metadata from a path.
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let metadata: Metadata = toml::from_str(&content)?;
        Ok(metadata)
    }

    /// Save metadata to a path.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_metadata_new() {
        let metadata = Metadata::new();
        assert!(!metadata.project_id.is_empty());
        // Petname format: word-word-word
        assert!(metadata.project_id.contains('-'));
    }

    #[test]
    fn test_metadata_save_load() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("metadata.toml");

        let metadata = Metadata {
            project_id: "test-project-id".to_string(),
        };
        metadata.save(&path).unwrap();

        let loaded = Metadata::load(&path).unwrap();
        assert_eq!(loaded.project_id, "test-project-id");
    }
}

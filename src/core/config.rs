//! Global configuration (~/.itack/config.toml).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::Result;

/// Default value for data_branch config.
fn default_data_branch() -> Option<String> {
    Some("data/itack".to_string())
}

/// Default value for merge_branch config.
fn default_merge_branch() -> Option<String> {
    Some("main".to_string())
}

/// Global itack configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default assignee name for claims.
    #[serde(default)]
    pub default_assignee: Option<String>,

    /// Default editor command.
    #[serde(default)]
    pub editor: Option<String>,

    /// Branch where itack stores issue data (default: "data/itack").
    #[serde(default = "default_data_branch")]
    pub data_branch: Option<String>,

    /// Branch to merge data into after commits (default: "main").
    /// Set to empty string or null to disable merging (data-only mode).
    #[serde(default = "default_merge_branch")]
    pub merge_branch: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_assignee: None,
            editor: None,
            data_branch: default_data_branch(),
            merge_branch: default_merge_branch(),
        }
    }
}

impl Config {
    /// Get the global config directory path.
    ///
    /// Uses `ITACK_HOME` environment variable if set, otherwise `~/.itack/`.
    /// This allows tests to redirect database storage to a temp directory.
    pub fn global_dir() -> Option<PathBuf> {
        if let Ok(home) = std::env::var("ITACK_HOME") {
            return Some(PathBuf::from(home));
        }
        dirs::home_dir().map(|h| h.join(".itack"))
    }

    /// Get the global config file path (~/.itack/config.toml).
    pub fn global_path() -> Option<PathBuf> {
        Self::global_dir().map(|d| d.join("config.toml"))
    }

    /// Load global config, returning default if not found.
    pub fn load_global() -> Result<Self> {
        let Some(path) = Self::global_path() else {
            return Ok(Config::default());
        };

        if !path.exists() {
            return Ok(Config::default());
        }

        let content = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save global config.
    #[allow(dead_code)]
    pub fn save_global(&self) -> Result<()> {
        let Some(dir) = Self::global_dir() else {
            return Ok(());
        };

        fs::create_dir_all(&dir)?;
        let path = dir.join("config.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Initialize global config directory if it doesn't exist.
    pub fn init_global() -> Result<()> {
        let Some(dir) = Self::global_dir() else {
            return Ok(());
        };

        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(())
    }

    /// Get the editor command to use.
    pub fn get_editor(&self) -> String {
        self.editor
            .clone()
            .or_else(|| std::env::var("EDITOR").ok())
            .or_else(|| std::env::var("VISUAL").ok())
            .unwrap_or_else(|| "vi".to_string())
    }
}

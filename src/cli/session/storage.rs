//! Session storage implementations

use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for session storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base directory for storage
    pub base_dir: PathBuf,
    /// Whether to enable compression
    pub compress: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_dir: crate::cli::default_data_dir(),
            compress: false,
        }
    }
}

/// JSON file-based storage implementation
#[derive(Debug)]
pub struct JsonFileStorage {
    config: StorageConfig,
}

impl JsonFileStorage {
    /// Create a new JSON file storage
    pub fn new(config: StorageConfig) -> Result<Self> {
        Ok(Self { config })
    }
    
    /// Get storage directory
    pub fn storage_dir(&self) -> PathBuf {
        self.config.base_dir.clone()
    }
}
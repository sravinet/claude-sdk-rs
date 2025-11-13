//! Session management for Claude Interactive
//!
//! This module provides functionality for managing Claude sessions,
//! including creating, storing, retrieving, and managing session state.

pub mod manager;
pub mod storage;

use crate::core::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Unique identifier for a session
pub type SessionId = Uuid;

/// Session metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// Human-readable session name
    pub name: String,
    /// Session description (optional)
    pub description: Option<String>,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Session configuration
    pub config: SessionConfig,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

/// Configuration for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Working directory for the session
    pub working_dir: Option<PathBuf>,
    /// Environment variables for the session
    pub env_vars: HashMap<String, String>,
    /// Default timeout for operations
    pub default_timeout: Option<u64>,
    /// Whether to enable verbose logging
    pub verbose: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            env_vars: HashMap::new(),
            default_timeout: Some(60),
            verbose: false,
        }
    }
}

/// Session metadata information
pub type SessionMetadata = HashMap<String, String>;

/// Manages Claude sessions
#[derive(Debug)]
pub struct SessionManager {
    /// Directory where session data is stored
    data_dir: PathBuf,
    /// In-memory cache of loaded sessions
    sessions: HashMap<SessionId, Session>,
}

impl SessionManager {
    /// Create a new session manager for testing
    pub fn new() -> Self {
        Self {
            data_dir: std::env::temp_dir(),
            sessions: HashMap::new(),
        }
    }

    /// Create a new session manager with default storage location (blocking)
    pub fn with_default_storage() -> Result<Self> {
        let data_dir = crate::cli::default_data_dir();
        // For default storage, just create a simple manager without async file operations
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir)?;
        }
        
        Ok(Self {
            data_dir,
            sessions: HashMap::new(),
        })
    }

    /// Create a new session manager
    pub async fn from_data_dir(data_dir: PathBuf) -> Result<Self> {
        // Ensure the data directory exists
        if !data_dir.exists() {
            tokio::fs::create_dir_all(&data_dir).await?;
        }

        let mut manager = Self {
            data_dir,
            sessions: HashMap::new(),
        };

        // Load existing sessions
        manager.load_sessions().await?;

        Ok(manager)
    }

    /// Create a new session
    pub async fn create_session(
        &mut self,
        name: String,
        description: Option<String>,
    ) -> Result<Session> {
        let config = Some(SessionConfig::default());
        let id = SessionId::new_v4();
        let now = Utc::now();

        let session = Session {
            id,
            name,
            description,
            created_at: now,
            last_accessed: now,
            config: config.unwrap_or_default(),
            metadata: HashMap::new(),
        };

        // Save to disk
        self.save_session(&session).await?;

        // Add to in-memory cache
        let session_clone = session.clone();
        self.sessions.insert(id, session);

        Ok(session_clone)
    }

    /// Get a session by ID
    pub async fn get_session(&mut self, id: SessionId) -> Result<Option<&Session>> {
        if let Some(session) = self.sessions.get(&id) {
            // Update last accessed time
            let mut updated_session = session.clone();
            updated_session.last_accessed = Utc::now();
            self.save_session(&updated_session).await?;
            self.sessions.insert(id, updated_session);
            return Ok(self.sessions.get(&id));
        }

        Ok(None)
    }

    /// Get a session by name
    pub async fn get_session_by_name(&mut self, name: &str) -> Result<Option<&Session>> {
        let id = self
            .sessions
            .values()
            .find(|session| session.name == name)
            .map(|session| session.id);

        if let Some(id) = id {
            self.get_session(id).await
        } else {
            Ok(None)
        }
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<Session>> {
        Ok(self.sessions.values().cloned().collect())
    }

    /// Get the current session ID (most recently accessed)
    pub fn get_current_session_id(&self) -> Option<SessionId> {
        self.sessions
            .values()
            .max_by_key(|session| session.last_accessed)
            .map(|session| session.id)
    }

    /// Get the current session (most recently accessed)
    pub fn get_current_session(&self) -> Option<&Session> {
        self.sessions
            .values()
            .max_by_key(|session| session.last_accessed)
    }

    /// Switch to a session by ID
    pub async fn switch_to_session(&mut self, id: SessionId) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.last_accessed = Utc::now();
            // Create a clone to avoid borrow checker issues
            let session_to_save = session.clone();
            let _ = session; // Explicitly drop the mutable borrow
            self.save_session(&session_to_save).await?;
        }
        Ok(())
    }

    /// Delete a session
    pub fn delete_session(&mut self, id: SessionId) -> Result<bool> {
        if self.sessions.remove(&id).is_some() {
            // Remove from disk
            let session_file = self.session_file_path(id);
            if session_file.exists() {
                std::fs::remove_file(session_file)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Update session metadata
    pub async fn update_session_metadata(
        &mut self,
        id: SessionId,
        key: String,
        value: String,
    ) -> Result<()> {
        if let Some(session) = self.sessions.get_mut(&id) {
            session.metadata.insert(key, value);
            session.last_accessed = Utc::now();
            let session_clone = session.clone();
            self.save_session(&session_clone).await?;
        }
        Ok(())
    }

    /// Load all sessions from disk
    async fn load_sessions(&mut self) -> Result<()> {
        let sessions_dir = self.data_dir.join("sessions");
        if !sessions_dir.exists() {
            std::fs::create_dir_all(&sessions_dir)?;
            return Ok(());
        }

        let mut entries = tokio::fs::read_dir(&sessions_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(session) = self.load_session_from_file(&path).await {
                    self.sessions.insert(session.id, session);
                }
            }
        }

        Ok(())
    }

    /// Load a session from a file
    async fn load_session_from_file(&self, path: &Path) -> Result<Session> {
        let content = tokio::fs::read_to_string(path).await?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// Save a session to disk
    async fn save_session(&self, session: &Session) -> Result<()> {
        let sessions_dir = self.data_dir.join("sessions");
        if !sessions_dir.exists() {
            tokio::fs::create_dir_all(&sessions_dir).await?;
        }

        let session_file = self.session_file_path(session.id);
        let content = serde_json::to_string_pretty(session)?;
        tokio::fs::write(session_file, content).await?;

        Ok(())
    }

    /// Get the file path for a session
    fn session_file_path(&self, id: SessionId) -> PathBuf {
        self.data_dir.join("sessions").join(format!("{}.json", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (SessionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (mut manager, _temp_dir) = create_test_manager();

        let session = manager
            .create_session("test-session".to_string(), None)
            .await
            .unwrap();

        assert!(manager.sessions.contains_key(&session.id));

        let retrieved_session = manager.get_session(session.id).await.unwrap().unwrap();
        assert_eq!(retrieved_session.name, "test-session");
        assert!(retrieved_session.description.is_none());
    }

    #[tokio::test]
    async fn test_get_session_by_name() {
        let (mut manager, _temp_dir) = create_test_manager();

        let _session = manager
            .create_session("test-session".to_string(), None)
            .await
            .unwrap();

        let session = manager
            .get_session_by_name("test-session")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(session.name, "test-session");

        let not_found = manager.get_session_by_name("nonexistent").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let (mut manager, _temp_dir) = create_test_manager();

        let _session1 = manager
            .create_session("session1".to_string(), None)
            .await
            .unwrap();
        let _session2 = manager
            .create_session("session2".to_string(), None)
            .await
            .unwrap();

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);

        let names: Vec<&str> = sessions.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"session1"));
        assert!(names.contains(&"session2"));
    }

    #[tokio::test]
    async fn test_delete_session() {
        let (mut manager, _temp_dir) = create_test_manager();

        let session = manager
            .create_session("test-session".to_string(), None)
            .await
            .unwrap();
        let session_id = session.id;

        assert!(manager.sessions.contains_key(&session_id));

        let deleted = manager.delete_session(session_id).unwrap();
        assert!(deleted);
        assert!(!manager.sessions.contains_key(&session_id));

        // Try to delete again - should return false
        let deleted_again = manager.delete_session(session_id).unwrap();
        assert!(!deleted_again);
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        let session_id = {
            let mut manager = SessionManager::from_data_dir(data_dir.clone()).await.unwrap();
            let session = manager
                .create_session("persistent-session".to_string(), None)
                .await
                .unwrap();
            session.id
        };

        // Create a new manager instance - should load the session from disk
        let mut manager2 = SessionManager::from_data_dir(data_dir).await.unwrap();
        let session = manager2.get_session(session_id).await.unwrap().unwrap();
        assert_eq!(session.name, "persistent-session");
    }

    #[tokio::test]
    async fn test_update_session_metadata() {
        let (mut manager, _temp_dir) = create_test_manager();

        let session = manager
            .create_session("test-session".to_string(), None)
            .await
            .unwrap();
        let session_id = session.id;

        manager
            .update_session_metadata(session_id, "key1".to_string(), "value1".to_string())
            .await
            .unwrap();

        let session = manager.get_session(session_id).await.unwrap().unwrap();
        assert_eq!(session.metadata.get("key1"), Some(&"value1".to_string()));
    }
}

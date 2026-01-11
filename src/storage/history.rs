//! Watch history management

use crate::error::Result;
use crate::types::{HistoryEntry, Video};
use crate::utils::paths::ensure_dir;
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs;

/// History manager
pub struct History {
    path: PathBuf,
    max_entries: usize,
    entries: Vec<HistoryEntry>,
}

impl History {
    pub fn new(path: &str, max_entries: usize) -> Self {
        Self {
            path: PathBuf::from(path),
            max_entries,
            entries: Vec::new(),
        }
    }

    /// Load history from file
    pub async fn load(&mut self) -> Result<()> {
        if !self.path.exists() {
            self.entries = Vec::new();
            return Ok(());
        }

        let content = fs::read_to_string(&self.path).await?;
        self.entries = serde_json::from_str(&content).unwrap_or_default();
        Ok(())
    }

    /// Save history to file
    pub async fn save(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            ensure_dir(&parent.to_string_lossy()).await?;
        }
        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.path, content).await?;
        Ok(())
    }

    /// Add video to history
    pub async fn add(&mut self, video: &Video) -> Result<()> {
        let entry = HistoryEntry {
            video: video.clone(),
            timestamp: Utc::now().timestamp(),
        };

        // Remove any existing entry with the same ID
        self.entries.retain(|e| e.video.id != video.id);

        // Add new entry at the beginning
        self.entries.insert(0, entry);

        // Trim to max entries
        if self.entries.len() > self.max_entries {
            self.entries.truncate(self.max_entries);
        }

        self.save().await
    }

    /// Get all history entries
    pub fn get_all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Clear all history
    #[allow(dead_code)]
    pub async fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        if self.path.exists() {
            fs::remove_file(&self.path).await?;
        }
        Ok(())
    }
}

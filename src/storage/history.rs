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

    /// Load history from file.
    ///
    /// If the file exists but fails to parse, back it up with a timestamped
    /// suffix (so the user can recover it) and start with an empty history
    /// instead of silently wiping the file on next save.
    pub async fn load(&mut self) -> Result<()> {
        if !self.path.exists() {
            self.entries = Vec::new();
            return Ok(());
        }

        let content = fs::read_to_string(&self.path).await?;
        match serde_json::from_str::<Vec<HistoryEntry>>(&content) {
            Ok(entries) => {
                self.entries = entries;
            }
            Err(err) => {
                let ts = Utc::now().timestamp();
                let backup = self.path.with_extension(format!("json.corrupt-{}", ts));
                eprintln!(
                    "Warning: history file at {} is corrupt ({}). Backed up to {} and starting fresh.",
                    self.path.display(),
                    err,
                    backup.display()
                );
                let _ = fs::rename(&self.path, &backup).await;
                self.entries = Vec::new();
            }
        }
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
    pub async fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        if self.path.exists() {
            fs::remove_file(&self.path).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn unique_temp_path() -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "yt-chill-test-history-{}-{}-{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            n
        ));
        p
    }

    fn video(id: &str) -> Video {
        Video {
            id: id.into(),
            title: format!("Title {}", id),
            author: "Author".into(),
            duration: "1:00".into(),
            views: "1".into(),
            published: "now".into(),
            thumbnail: "".into(),
        }
    }

    #[tokio::test]
    async fn load_on_missing_file_is_empty() {
        let path = unique_temp_path();
        let mut h = History::new(path.to_str().unwrap(), 10);
        h.load().await.unwrap();
        assert!(h.get_all().is_empty());
    }

    #[tokio::test]
    async fn add_save_load_roundtrip_preserves_order() {
        let path = unique_temp_path();
        {
            let mut h = History::new(path.to_str().unwrap(), 10);
            h.load().await.unwrap();
            h.add(&video("a")).await.unwrap();
            h.add(&video("b")).await.unwrap();
            h.add(&video("c")).await.unwrap();
        }
        let mut h2 = History::new(path.to_str().unwrap(), 10);
        h2.load().await.unwrap();
        let ids: Vec<&str> = h2.get_all().iter().map(|e| e.video.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "b", "a"]);
        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn readding_moves_to_front_without_duplication() {
        let path = unique_temp_path();
        let mut h = History::new(path.to_str().unwrap(), 10);
        h.load().await.unwrap();
        h.add(&video("a")).await.unwrap();
        h.add(&video("b")).await.unwrap();
        h.add(&video("a")).await.unwrap();
        let ids: Vec<&str> = h.get_all().iter().map(|e| e.video.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn add_trims_to_max_entries() {
        let path = unique_temp_path();
        let mut h = History::new(path.to_str().unwrap(), 2);
        h.load().await.unwrap();
        h.add(&video("a")).await.unwrap();
        h.add(&video("b")).await.unwrap();
        h.add(&video("c")).await.unwrap();
        assert_eq!(h.get_all().len(), 2);
        let ids: Vec<&str> = h.get_all().iter().map(|e| e.video.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "b"]);
        let _ = tokio::fs::remove_file(&path).await;
    }

    #[tokio::test]
    async fn corrupt_json_is_backed_up_and_load_returns_empty() {
        let path = unique_temp_path();
        tokio::fs::write(&path, "{not json").await.unwrap();
        let mut h = History::new(path.to_str().unwrap(), 10);
        h.load().await.unwrap();
        assert!(h.get_all().is_empty());

        // Backup file (`.corrupt-<ts>`) should exist alongside.
        let parent = path.parent().unwrap();
        let stem = path.file_stem().unwrap().to_string_lossy().to_string();
        let mut found = false;
        let mut rd = tokio::fs::read_dir(parent).await.unwrap();
        while let Some(entry) = rd.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&stem) && name.contains(".corrupt-") {
                let _ = tokio::fs::remove_file(entry.path()).await;
                found = true;
            }
        }
        assert!(found, "expected corrupt backup file");
    }
}

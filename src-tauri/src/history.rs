//! Recording history — persists past recordings to JSON

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingEntry {
    pub id: String,
    pub output_path: String,
    pub duration_secs: f64,
    pub file_size_bytes: u64,
    pub has_audio: bool,
    pub resolution: String,
    pub fps: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecordingHistory {
    pub entries: Vec<RecordingEntry>,
}

impl RecordingHistory {
    fn history_path() -> PathBuf {
        let base = dirs::config_dir()
            .or_else(|| dirs::home_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("easyspecy").join("history.json")
    }

    pub fn load() -> Self {
        let path = Self::history_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::history_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn add(&mut self, entry: RecordingEntry) {
        self.entries.insert(0, entry);
        // Keep last 50 recordings
        self.entries.truncate(50);
        let _ = self.save();
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = self.save();
    }
}

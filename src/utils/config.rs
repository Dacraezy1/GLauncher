use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: String,
    pub language: String,
    pub close_on_launch: bool,
    pub show_snapshots: bool,
    pub show_beta: bool,
    pub show_alpha: bool,
    pub concurrent_downloads: u32,
    pub default_memory_min: u32,
    pub default_memory_max: u32,
    pub analytics_opt_in: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            language: "en".to_string(),
            close_on_launch: false,
            show_snapshots: false,
            show_beta: false,
            show_alpha: false,
            concurrent_downloads: 4,
            default_memory_min: 512,
            default_memory_max: 2048,
            analytics_opt_in: false,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = paths::config_file();
        if path.exists() {
            let s = std::fs::read_to_string(&path)?;
            Ok(serde_json::from_str(&s).unwrap_or_default())
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_file();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, s)?;
        Ok(())
    }
}

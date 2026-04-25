use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModSource {
    Modrinth { project_id: String, version_id: String },
    CurseForge { mod_id: u64, file_id: u64 },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub version: String,
    pub source: ModSource,
    pub enabled: bool,
    pub icon_url: Option<String>,
    pub description: Option<String>,
    pub installed_at: String,
}

impl InstalledMod {
    pub fn new_manual(filename: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: filename.trim_end_matches(".jar").to_string(),
            filename: filename.to_string(),
            version: "unknown".to_string(),
            source: ModSource::Manual,
            enabled: true,
            icon_url: None,
            description: None,
            installed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub struct ModManager {
    pub mods: Vec<InstalledMod>,
    mods_dir: PathBuf,
    meta_file: PathBuf,
}

impl ModManager {
    pub fn load(mods_dir: PathBuf) -> Result<Self> {
        let meta_file = mods_dir.parent().unwrap().join("mods_meta.json");
        std::fs::create_dir_all(&mods_dir)?;

        let mods = if meta_file.exists() {
            let s = std::fs::read_to_string(&meta_file)?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            // Scan mods dir for jars
            let mut found = Vec::new();
            for entry in std::fs::read_dir(&mods_dir)?.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".jar") || name.ends_with(".jar.disabled") {
                    let enabled = !name.ends_with(".disabled");
                    let filename = name.trim_end_matches(".disabled").to_string();
                    found.push(InstalledMod {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: filename.trim_end_matches(".jar").to_string(),
                        filename: filename.clone(),
                        version: "unknown".to_string(),
                        source: ModSource::Manual,
                        enabled,
                        icon_url: None,
                        description: None,
                        installed_at: chrono::Utc::now().to_rfc3339(),
                    });
                }
            }
            found
        };

        Ok(Self { mods, mods_dir, meta_file })
    }

    pub fn save(&self) -> Result<()> {
        let s = serde_json::to_string_pretty(&self.mods)?;
        std::fs::write(&self.meta_file, s)?;
        Ok(())
    }

    pub fn add(&mut self, mod_entry: InstalledMod) {
        self.mods.retain(|m| m.filename != mod_entry.filename);
        self.mods.push(mod_entry);
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(m) = self.mods.iter().find(|m| m.id == id) {
            let jar = self.mods_dir.join(&m.filename);
            if jar.exists() { std::fs::remove_file(&jar)?; }
            let disabled = self.mods_dir.join(format!("{}.disabled", m.filename));
            if disabled.exists() { std::fs::remove_file(&disabled)?; }
        }
        self.mods.retain(|m| m.id != id);
        Ok(())
    }

    pub fn toggle(&mut self, id: &str) -> Result<()> {
        if let Some(m) = self.mods.iter_mut().find(|m| m.id == id) {
            if m.enabled {
                // Disable: rename .jar → .jar.disabled
                let from = self.mods_dir.join(&m.filename);
                let to = self.mods_dir.join(format!("{}.disabled", m.filename));
                if from.exists() { std::fs::rename(from, to)?; }
                m.enabled = false;
            } else {
                // Enable: rename .jar.disabled → .jar
                let from = self.mods_dir.join(format!("{}.disabled", m.filename));
                let to = self.mods_dir.join(&m.filename);
                if from.exists() { std::fs::rename(from, to)?; }
                m.enabled = true;
            }
        }
        Ok(())
    }
}

use serde::{Deserialize, Serialize};
use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModLoader {
    Vanilla,
    Fabric,
    Forge,
    Quilt,
    NeoForge,
}

impl std::fmt::Display for ModLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModLoader::Vanilla => write!(f, "Vanilla"),
            ModLoader::Fabric => write!(f, "Fabric"),
            ModLoader::Forge => write!(f, "Forge"),
            ModLoader::Quilt => write!(f, "Quilt"),
            ModLoader::NeoForge => write!(f, "NeoForge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JvmSettings {
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub extra_jvm_args: Vec<String>,
    pub java_path: Option<String>,
    pub java_major_version: Option<u32>,
    // Performance presets
    pub use_g1gc: bool,
    pub use_aikar_flags: bool,
    pub use_zgc: bool,
    pub use_shenandoah: bool,
}

impl Default for JvmSettings {
    fn default() -> Self {
        Self {
            memory_min_mb: 512,
            memory_max_mb: 2048,
            extra_jvm_args: vec![],
            java_path: None,
            java_major_version: None,
            use_g1gc: true,
            use_aikar_flags: false,
            use_zgc: false,
            use_shenandoah: false,
        }
    }
}

impl JvmSettings {
    pub fn build_flags(&self) -> Vec<String> {
        let mut flags = vec![
            format!("-Xms{}m", self.memory_min_mb),
            format!("-Xmx{}m", self.memory_max_mb),
        ];

        if self.use_aikar_flags {
            // Aikar's famous GC flags
            flags.extend_from_slice(&[
                "-XX:+UseG1GC".to_string(),
                "-XX:+ParallelRefProcEnabled".to_string(),
                "-XX:MaxGCPauseMillis=200".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:+DisableExplicitGC".to_string(),
                "-XX:+AlwaysPreTouch".to_string(),
                "-XX:G1NewSizePercent=30".to_string(),
                "-XX:G1MaxNewSizePercent=40".to_string(),
                "-XX:G1HeapRegionSize=8M".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:G1HeapWastePercent=5".to_string(),
                "-XX:G1MixedGCCountTarget=4".to_string(),
                "-XX:InitiatingHeapOccupancyPercent=15".to_string(),
                "-XX:G1MixedGCLiveThresholdPercent=90".to_string(),
                "-XX:G1RSetUpdatingPauseTimePercent=5".to_string(),
                "-XX:SurvivorRatio=32".to_string(),
                "-XX:+PerfDisableSharedMem".to_string(),
                "-XX:MaxTenuringThreshold=1".to_string(),
                "-Dusing.aikars.flags=https://mcflags.emc.gs".to_string(),
                "-Daikars.new.flags=true".to_string(),
            ]);
        } else if self.use_zgc {
            flags.extend_from_slice(&[
                "-XX:+UseZGC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
            ]);
        } else if self.use_shenandoah {
            flags.extend_from_slice(&[
                "-XX:+UseShenandoahGC".to_string(),
                "-XX:ShenandoahGCMode=iu".to_string(),
            ]);
        } else if self.use_g1gc {
            flags.extend_from_slice(&[
                "-XX:+UseG1GC".to_string(),
                "-XX:+UnlockExperimentalVMOptions".to_string(),
                "-XX:G1NewSizePercent=20".to_string(),
                "-XX:G1ReservePercent=20".to_string(),
                "-XX:MaxGCPauseMillis=50".to_string(),
                "-XX:G1HeapRegionSize=32M".to_string(),
            ]);
        }

        // Always add
        flags.push("-XX:+UseStringDeduplication".to_string());
        flags.push("-Dfile.encoding=UTF-8".to_string());
        flags.push("-Dfml.ignoreInvalidMinecraftCertificates=true".to_string());
        flags.push("-Dfml.ignorePatchDiscrepancies=true".to_string());

        flags.extend_from_slice(&self.extra_jvm_args);
        flags
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub mod_loader: ModLoader,
    pub loader_version: Option<String>,
    pub jvm_settings: JvmSettings,
    pub icon: Option<String>,
    pub created_at: String,
    pub last_played: Option<String>,
    pub play_time_seconds: u64,
    pub notes: String,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub fullscreen: bool,
    pub game_resolution_auto: bool,
}

impl Instance {
    pub fn new(name: &str, minecraft_version: &str, mod_loader: ModLoader) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            minecraft_version: minecraft_version.to_string(),
            mod_loader,
            loader_version: None,
            jvm_settings: JvmSettings::default(),
            icon: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: None,
            play_time_seconds: 0,
            notes: String::new(),
            window_width: None,
            window_height: None,
            fullscreen: false,
            game_resolution_auto: true,
        }
    }

    pub fn dir(&self) -> PathBuf {
        paths::instances_dir().join(&self.id)
    }

    pub fn minecraft_dir(&self) -> PathBuf {
        self.dir().join("minecraft")
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.minecraft_dir().join("mods")
    }

    pub fn saves_dir(&self) -> PathBuf {
        self.minecraft_dir().join("saves")
    }

    pub fn screenshots_dir(&self) -> PathBuf {
        self.minecraft_dir().join("screenshots")
    }

    pub fn config_file(&self) -> PathBuf {
        self.dir().join("instance.json")
    }

    pub fn save(&self) -> Result<()> {
        std::fs::create_dir_all(self.dir())?;
        std::fs::create_dir_all(self.minecraft_dir())?;
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(self.config_file(), s)?;
        Ok(())
    }

    pub fn loader_display(&self) -> String {
        match &self.loader_version {
            Some(v) => format!("{} {}", self.mod_loader, v),
            None => self.mod_loader.to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct InstanceManager {
    pub instances: Vec<Instance>,
}

impl InstanceManager {
    pub fn load() -> Result<Self> {
        let instances_dir = paths::instances_dir();
        std::fs::create_dir_all(&instances_dir)?;

        let mut instances = Vec::new();

        for entry in std::fs::read_dir(&instances_dir)? {
            let entry = entry?;
            let config = entry.path().join("instance.json");
            if config.exists() {
                match std::fs::read_to_string(&config) {
                    Ok(s) => match serde_json::from_str::<Instance>(&s) {
                        Ok(inst) => instances.push(inst),
                        Err(e) => log::warn!("Failed to parse instance {}: {e}", config.display()),
                    },
                    Err(e) => log::warn!("Failed to read {}: {e}", config.display()),
                }
            }
        }

        // Sort by last played (most recent first)
        instances.sort_by(|a, b| {
            b.last_played.cmp(&a.last_played)
        });

        Ok(Self { instances })
    }

    pub fn add(&mut self, instance: Instance) -> Result<()> {
        instance.save()?;
        self.instances.push(instance);
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<()> {
        if let Some(inst) = self.instances.iter().find(|i| i.id == id) {
            if inst.dir().exists() {
                std::fs::remove_dir_all(inst.dir())?;
            }
        }
        self.instances.retain(|i| i.id != id);
        Ok(())
    }

    pub fn update(&mut self, instance: Instance) -> Result<()> {
        instance.save()?;
        if let Some(existing) = self.instances.iter_mut().find(|i| i.id == instance.id) {
            *existing = instance;
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&Instance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|i| i.id == id)
    }
}

use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::utils::paths;

pub const VERSION_MANIFEST_URL: &str =
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VersionType {
    Release,
    Snapshot,
    #[serde(rename = "old_beta")]
    OldBeta,
    #[serde(rename = "old_alpha")]
    OldAlpha,
}

impl std::fmt::Display for VersionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionType::Release => write!(f, "Release"),
            VersionType::Snapshot => write!(f, "Snapshot"),
            VersionType::OldBeta => write!(f, "Beta"),
            VersionType::OldAlpha => write!(f, "Alpha"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub sha1: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersions,
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatestVersions {
    pub release: String,
    pub snapshot: String,
}

// Full version metadata (from individual version URL)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMeta {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: VersionType,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub assets: Option<String>,
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    pub downloads: Option<VersionDownloads>,
    pub libraries: Vec<Library>,
    pub arguments: Option<Arguments>,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>, // old format
    #[serde(rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    #[serde(rename = "releaseTime")]
    pub release_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
    #[serde(rename = "totalSize")]
    pub total_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDownloads {
    pub client: Option<Download>,
    pub server: Option<Download>,
    #[serde(rename = "client_mappings")]
    pub client_mappings: Option<Download>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Download {
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<std::collections::HashMap<String, String>>,
    pub extract: Option<ExtractConfig>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<std::collections::HashMap<String, Artifact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub path: Option<String>,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<std::collections::HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractConfig {
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Option<Vec<serde_json::Value>>,
    pub jvm: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename = "majorVersion")]
    pub major_version: u32,
}

pub struct VersionManager {
    client: reqwest::Client,
}

impl VersionManager {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn fetch_manifest(&self) -> Result<VersionManifest> {
        let cache_path = paths::versions_dir().join("version_manifest_v2.json");

        // Try cache first (10 min TTL)
        if cache_path.exists() {
            if let Ok(meta) = std::fs::metadata(&cache_path) {
                if let Ok(modified) = meta.modified() {
                    if modified.elapsed().unwrap_or_default().as_secs() < 600 {
                        if let Ok(s) = std::fs::read_to_string(&cache_path) {
                            if let Ok(manifest) = serde_json::from_str::<VersionManifest>(&s) {
                                return Ok(manifest);
                            }
                        }
                    }
                }
            }
        }

        let resp = self.client.get(VERSION_MANIFEST_URL).send().await?;
        let text = resp.text().await?;

        std::fs::create_dir_all(cache_path.parent().unwrap())?;
        std::fs::write(&cache_path, &text)?;

        Ok(serde_json::from_str(&text)?)
    }

    pub async fn fetch_version_meta(&self, entry: &VersionEntry) -> Result<VersionMeta> {
        let cache_path = paths::versions_dir()
            .join(&entry.id)
            .join(format!("{}.json", entry.id));

        if cache_path.exists() {
            let s = std::fs::read_to_string(&cache_path)?;
            return Ok(serde_json::from_str(&s)?);
        }

        let resp = self.client.get(&entry.url).send().await?;
        let text = resp.text().await?;

        std::fs::create_dir_all(cache_path.parent().unwrap())?;
        std::fs::write(&cache_path, &text)?;

        Ok(serde_json::from_str(&text)?)
    }
}

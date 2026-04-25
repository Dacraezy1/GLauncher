use serde::{Deserialize, Serialize};
use anyhow::{Result, bail};
use std::path::PathBuf;
use crate::utils::{paths, download};

// ─── Fabric ────────────────────────────────────────────────────────────────

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricLoaderVersion {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricGameVersion {
    pub version: String,
    pub stable: bool,
}

pub async fn fetch_fabric_loader_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<Vec<FabricLoaderVersion>> {
    let url = format!("{FABRIC_META_URL}/versions/loader/{mc_version}");
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to fetch Fabric versions");
    }
    let data: Vec<serde_json::Value> = resp.json().await?;
    let versions: Vec<FabricLoaderVersion> = data
        .iter()
        .filter_map(|v| {
            let loader = v.get("loader")?;
            Some(FabricLoaderVersion {
                version: loader.get("version")?.as_str()?.to_string(),
                stable: loader.get("stable")?.as_bool().unwrap_or(false),
            })
        })
        .collect();
    Ok(versions)
}

pub async fn install_fabric(
    client: &reqwest::Client,
    mc_version: &str,
    loader_version: &str,
    instance_dir: &PathBuf,
    progress_tx: Option<tokio::sync::mpsc::Sender<download::DownloadProgress>>,
) -> Result<PathBuf> {
    let url = format!(
        "{FABRIC_META_URL}/versions/loader/{mc_version}/{loader_version}/profile/json"
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to fetch Fabric profile");
    }

    let profile_json = resp.text().await?;
    let profile_path = instance_dir.join("fabric-profile.json");
    std::fs::write(&profile_path, &profile_json)?;

    // Download Fabric libraries
    let profile: serde_json::Value = serde_json::from_str(&profile_json)?;
    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        let libs_dir = paths::libraries_dir();
        for lib in libs {
            if let (Some(name), Some(url)) = (
                lib.get("name").and_then(|n| n.as_str()),
                lib.get("url").and_then(|u| u.as_str()),
            ) {
                let path = maven_name_to_path(name);
                let dest = libs_dir.join(&path);
                if !dest.exists() {
                    let dl_url = format!("{url}{path}");
                    download::download_file(client, &dl_url, &dest, progress_tx.clone()).await
                        .unwrap_or_else(|e| log::warn!("Failed to download Fabric lib {name}: {e}"));
                }
            }
        }
    }

    Ok(profile_path)
}

// ─── Forge ─────────────────────────────────────────────────────────────────

const FORGE_META_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeVersion {
    pub version: String,
    pub mc_version: String,
    pub recommended: bool,
    pub latest: bool,
}

pub async fn fetch_forge_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<Vec<ForgeVersion>> {
    // Use Forge promotions endpoint
    let url = format!("{FORGE_META_URL}/promotions_slim.json");
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        bail!("Failed to fetch Forge promotions");
    }

    let data: serde_json::Value = resp.json().await?;
    let promos = data.get("promos").and_then(|p| p.as_object()).cloned().unwrap_or_default();

    let mut versions = Vec::new();

    let recommended_key = format!("{mc_version}-recommended");
    let latest_key = format!("{mc_version}-latest");

    if let Some(v) = promos.get(&recommended_key).and_then(|v| v.as_str()) {
        versions.push(ForgeVersion {
            version: format!("{mc_version}-{v}"),
            mc_version: mc_version.to_string(),
            recommended: true,
            latest: false,
        });
    }

    if let Some(v) = promos.get(&latest_key).and_then(|v| v.as_str()) {
        let full = format!("{mc_version}-{v}");
        if !versions.iter().any(|fv| fv.version == full) {
            versions.push(ForgeVersion {
                version: full,
                mc_version: mc_version.to_string(),
                recommended: false,
                latest: true,
            });
        }
    }

    Ok(versions)
}

pub async fn install_forge(
    client: &reqwest::Client,
    mc_version: &str,
    forge_version: &str,
    instance_dir: &PathBuf,
    java_path: &str,
    progress_tx: Option<tokio::sync::mpsc::Sender<download::DownloadProgress>>,
) -> Result<PathBuf> {
    let installer_name = format!("forge-{forge_version}-installer.jar");
    let installer_url = format!(
        "{FORGE_MAVEN}net/minecraftforge/forge/{forge_version}/{installer_name}"
    );

    let installer_path = instance_dir.join(&installer_name);
    download::download_file(client, &installer_url, &installer_path, progress_tx).await?;

    // Run installer in offline/client mode
    let output = tokio::process::Command::new(java_path)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(instance_dir.to_str().unwrap())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Forge installer failed: {stderr}");
    }

    let profile_path = instance_dir.join("forge-profile.json");
    Ok(profile_path)
}

// ─── Quilt ──────────────────────────────────────────────────────────────────

const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";

pub async fn fetch_quilt_loader_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<Vec<FabricLoaderVersion>> {
    let url = format!("{QUILT_META_URL}/versions/loader/{mc_version}");
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to fetch Quilt versions");
    }
    let data: Vec<serde_json::Value> = resp.json().await?;
    let versions: Vec<FabricLoaderVersion> = data
        .iter()
        .filter_map(|v| {
            let loader = v.get("loader")?;
            Some(FabricLoaderVersion {
                version: loader.get("version")?.as_str()?.to_string(),
                stable: !loader.get("version")?.as_str()?.contains("beta"),
            })
        })
        .collect();
    Ok(versions)
}

pub async fn install_quilt(
    client: &reqwest::Client,
    mc_version: &str,
    loader_version: &str,
    instance_dir: &PathBuf,
    progress_tx: Option<tokio::sync::mpsc::Sender<download::DownloadProgress>>,
) -> Result<PathBuf> {
    let url = format!(
        "{QUILT_META_URL}/versions/loader/{mc_version}/{loader_version}/profile/json"
    );

    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to fetch Quilt profile");
    }

    let profile_json = resp.text().await?;
    let profile_path = instance_dir.join("quilt-profile.json");
    std::fs::write(&profile_path, &profile_json)?;

    // Download Quilt libraries
    let profile: serde_json::Value = serde_json::from_str(&profile_json)?;
    if let Some(libs) = profile.get("libraries").and_then(|l| l.as_array()) {
        let libs_dir = paths::libraries_dir();
        for lib in libs {
            if let (Some(name), Some(url)) = (
                lib.get("name").and_then(|n| n.as_str()),
                lib.get("url").and_then(|u| u.as_str()),
            ) {
                let path = maven_name_to_path(name);
                let dest = libs_dir.join(&path);
                if !dest.exists() {
                    let dl_url = format!("{url}{path}");
                    download::download_file(client, &dl_url, &dest, progress_tx.clone()).await
                        .unwrap_or_else(|e| log::warn!("Failed to download Quilt lib {name}: {e}"));
                }
            }
        }
    }

    Ok(profile_path)
}

// ─── NeoForge ───────────────────────────────────────────────────────────────

const NEOFORGE_META_URL: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge";
const NEOFORGE_MAVEN: &str = "https://maven.neoforged.net/releases/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoForgeVersion {
    pub version: String,
    pub is_latest: bool,
}

pub async fn fetch_neoforge_versions(
    client: &reqwest::Client,
    mc_version: &str,
) -> Result<Vec<NeoForgeVersion>> {
    let url = format!("{NEOFORGE_META_URL}/maven-metadata.xml");
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        bail!("Failed to fetch NeoForge versions");
    }

    let text = resp.text().await?;

    // Parse version prefix from mc_version: "1.20.1" → "20.1"
    let prefix = mc_version.trim_start_matches("1.").to_string();

    let mut versions: Vec<String> = text
        .lines()
        .filter(|l| l.contains("<version>"))
        .filter_map(|l| {
            let s = l.trim();
            let s = s.trim_start_matches("<version>").trim_end_matches("</version>");
            if s.starts_with(&prefix) {
                Some(s.to_string())
            } else {
                None
            }
        })
        .collect();

    versions.reverse();
    let len = versions.len();

    Ok(versions
        .into_iter()
        .enumerate()
        .map(|(i, v)| NeoForgeVersion {
            version: v,
            is_latest: i == 0,
        })
        .take(20)
        .collect())
}

pub async fn install_neoforge(
    client: &reqwest::Client,
    mc_version: &str,
    neoforge_version: &str,
    instance_dir: &PathBuf,
    java_path: &str,
    progress_tx: Option<tokio::sync::mpsc::Sender<download::DownloadProgress>>,
) -> Result<PathBuf> {
    let installer_name = format!("neoforge-{neoforge_version}-installer.jar");
    let installer_url = format!(
        "{NEOFORGE_MAVEN}net/neoforged/neoforge/{neoforge_version}/{installer_name}"
    );

    let installer_path = instance_dir.join(&installer_name);
    download::download_file(client, &installer_url, &installer_path, progress_tx).await?;

    let output = tokio::process::Command::new(java_path)
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .arg(instance_dir.to_str().unwrap())
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("NeoForge installer failed: {stderr}");
    }

    let profile_path = instance_dir.join("neoforge-profile.json");
    Ok(profile_path)
}

// ─── Helpers ────────────────────────────────────────────────────────────────

pub fn maven_name_to_path(name: &str) -> String {
    // e.g. "net.fabricmc:fabric-loader:0.15.7" → "net/fabricmc/fabric-loader/0.15.7/fabric-loader-0.15.7.jar"
    let parts: Vec<&str> = name.splitn(3, ':').collect();
    if parts.len() < 3 {
        return format!("{}.jar", name.replace(':', '/'));
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    format!("{group}/{artifact}/{version}/{artifact}-{version}.jar")
}

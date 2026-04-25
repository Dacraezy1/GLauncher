use serde::{Deserialize, Serialize};
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};
use crate::utils::{paths, download};

// Adoptium API for Java downloads
const ADOPTIUM_API: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstall {
    pub major_version: u32,
    pub version: String,
    pub path: String,   // path to the JRE root (contains bin/java)
    pub arch: String,
    pub is_bundled: bool,
}

impl JavaInstall {
    pub fn java_bin(&self) -> PathBuf {
        Path::new(&self.path).join("bin").join("java")
    }

    pub fn is_available(&self) -> bool {
        self.java_bin().exists()
    }
}

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    release_name: String,
    binaries: Vec<AdoptiumBinary>,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
    architecture: String,
    os: String,
    image_type: String,
    jvm_impl: String,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    name: String,
    checksum: Option<String>,
    size: Option<u64>,
}

pub struct JavaManager {
    client: reqwest::Client,
}

/// All supported Java versions with their download availability
pub const SUPPORTED_JAVA_VERSIONS: &[u32] = &[8, 11, 17, 21, 22, 23, 24];
// Note: Java 26.1.2 doesn't exist — Java versioning is "major.minor.patch", latest LTS/latest is 21/22/23/24
// "26" would be a future version. We include up to 24 (current latest as of 2025)

impl JavaManager {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Detect all Java installations on the system
    pub fn detect_system_java(&self) -> Vec<JavaInstall> {
        let mut installs = Vec::new();

        // Check bundled GLauncher java first
        let java_dir = paths::java_dir();
        if java_dir.exists() {
            for entry in std::fs::read_dir(&java_dir).into_iter().flatten().flatten() {
                let bin = entry.path().join("bin").join("java");
                if bin.exists() {
                    if let Some(version_info) = Self::detect_java_version(&bin) {
                        installs.push(JavaInstall {
                            major_version: version_info.0,
                            version: version_info.1,
                            path: entry.path().to_string_lossy().to_string(),
                            arch: std::env::consts::ARCH.to_string(),
                            is_bundled: true,
                        });
                    }
                }
            }
        }

        // System java paths
        let system_paths = vec![
            "/usr/lib/jvm",
            "/usr/lib64/jvm",
            "/opt/java",
            "/opt/jdk",
        ];

        for base in system_paths {
            let base_path = Path::new(base);
            if !base_path.exists() { continue; }
            for entry in std::fs::read_dir(base_path).into_iter().flatten().flatten() {
                let bin = entry.path().join("bin").join("java");
                if bin.exists() {
                    if let Some(version_info) = Self::detect_java_version(&bin) {
                        installs.push(JavaInstall {
                            major_version: version_info.0,
                            version: version_info.1,
                            path: entry.path().to_string_lossy().to_string(),
                            arch: std::env::consts::ARCH.to_string(),
                            is_bundled: false,
                        });
                    }
                }
            }
        }

        // Check PATH java
        if let Ok(output) = std::process::Command::new("java").arg("-version").output() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if let Some(version) = parse_java_version_string(&stderr) {
                installs.push(JavaInstall {
                    major_version: version.0,
                    version: version.1,
                    path: "java".to_string(), // use PATH java
                    arch: std::env::consts::ARCH.to_string(),
                    is_bundled: false,
                });
            }
        }

        installs.sort_by_key(|j| j.major_version);
        installs.dedup_by_key(|j| j.path.clone());
        installs
    }

    fn detect_java_version(java_bin: &Path) -> Option<(u32, String)> {
        let output = std::process::Command::new(java_bin)
            .arg("-version")
            .output()
            .ok()?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{stderr}{stdout}");

        parse_java_version_string(&combined)
    }

    /// Download a specific Java version from Adoptium
    pub async fn download_java(
        &self,
        major_version: u32,
        progress_tx: Option<tokio::sync::mpsc::Sender<download::DownloadProgress>>,
    ) -> Result<JavaInstall> {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "aarch64",
            other => bail!("Unsupported architecture: {other}"),
        };

        let url = format!(
            "{ADOPTIUM_API}/assets/latest/{major_version}/hotspot\
            ?architecture={arch}&image_type=jre&os=linux&vendor=eclipse"
        );

        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            bail!("Failed to fetch Java {major_version} from Adoptium");
        }

        let releases: Vec<AdoptiumRelease> = resp.json().await?;
        let release = releases.first()
            .ok_or_else(|| anyhow::anyhow!("No Java {major_version} release found"))?;

        let binary = release.binaries.first()
            .ok_or_else(|| anyhow::anyhow!("No binary found"))?;

        let dest_dir = paths::java_dir().join(format!("java-{major_version}"));
        let archive_path = paths::java_dir().join(&binary.package.name);

        log::info!("Downloading Java {} from {}", major_version, binary.package.link);
        download::download_file(
            &self.client,
            &binary.package.link,
            &archive_path,
            progress_tx,
        ).await?;

        // Extract
        std::fs::create_dir_all(&dest_dir)?;
        self.extract_java(&archive_path, &dest_dir)?;
        std::fs::remove_file(&archive_path).ok();

        // Find actual java binary
        let java_root = find_java_root(&dest_dir)?;

        Ok(JavaInstall {
            major_version,
            version: release.release_name.clone(),
            path: java_root.to_string_lossy().to_string(),
            arch: arch.to_string(),
            is_bundled: true,
        })
    }

    fn extract_java(&self, archive: &Path, dest: &Path) -> Result<()> {
        let name = archive.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            let file = std::fs::File::open(archive)?;
            let gz = flate2::read::GzDecoder::new(file);
            let mut tar = tar::Archive::new(gz);
            tar.unpack(dest)?;
        } else if name.ends_with(".zip") {
            let file = std::fs::File::open(archive)?;
            let mut archive = zip::ZipArchive::new(file)?;
            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let out = dest.join(file.name());
                if file.name().ends_with('/') {
                    std::fs::create_dir_all(&out)?;
                } else {
                    if let Some(p) = out.parent() { std::fs::create_dir_all(p)?; }
                    let mut out_file = std::fs::File::create(&out)?;
                    std::io::copy(&mut file, &mut out_file)?;
                }
            }
        }

        Ok(())
    }

    /// Pick the best Java for a given Minecraft version
    pub fn pick_java_for_mc(&self, mc_major_java: Option<u32>, installs: &[JavaInstall]) -> Option<&JavaInstall> {
        let required = mc_major_java.unwrap_or(8);
        // Prefer exact match, then compatible higher version
        installs.iter()
            .filter(|j| j.is_available())
            .min_by_key(|j| {
                if j.major_version >= required {
                    j.major_version - required
                } else {
                    u32::MAX
                }
            })
    }
}

fn parse_java_version_string(s: &str) -> Option<(u32, String)> {
    // "openjdk version "17.0.9" 2023-10-17"
    // "java version "1.8.0_392""
    let version_line = s.lines().find(|l| l.contains("version"))?;
    let ver = version_line
        .split('"')
        .nth(1)?
        .to_string();

    let major: u32 = if ver.starts_with("1.") {
        // Java 8 and below: "1.8.x"
        ver.split('.').nth(1)?.parse().ok()?
    } else {
        // Java 9+: "17.0.9"
        ver.split('.').next()?.parse().ok()?
    };

    Some((major, ver))
}

fn find_java_root(dir: &Path) -> Result<PathBuf> {
    // After extraction there's usually a subdirectory
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let bin = entry.path().join("bin").join("java");
        if bin.exists() {
            return Ok(entry.path());
        }
    }
    // Maybe it's already the root
    let bin = dir.join("bin").join("java");
    if bin.exists() {
        return Ok(dir.to_path_buf());
    }
    bail!("Could not find java binary in extracted directory {}", dir.display());
}



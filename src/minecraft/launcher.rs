use anyhow::{Result, bail, Context};
use std::path::{Path, PathBuf};
use std::collections::HashSet;
use tokio::sync::mpsc;
use crate::minecraft::versions::{VersionMeta, Library, Rule};
use crate::minecraft::instances::Instance;
use crate::auth::accounts::{Account, AccountType};
use crate::utils::{paths, download};
use crate::java::JavaInstall;

pub enum LaunchEvent {
    Progress { message: String, percent: f64 },
    Log(String),
    Started(u32),  // PID
    Exited(i32),
    Error(String),
}

pub struct GameLauncher {
    client: reqwest::Client,
}

impl GameLauncher {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Install and then launch the game
    pub async fn install_and_launch(
        &self,
        instance: &Instance,
        account: &Account,
        java: &JavaInstall,
        event_tx: mpsc::Sender<LaunchEvent>,
    ) -> Result<()> {
        // Prepare directories
        std::fs::create_dir_all(instance.minecraft_dir())?;
        std::fs::create_dir_all(instance.mods_dir())?;

        // Load version manifest
        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Fetching version metadata...".to_string(),
            percent: 0.05,
        }).await;

        let vm = crate::minecraft::versions::VersionManager::new(self.client.clone());
        let manifest = vm.fetch_manifest().await?;

        let entry = manifest.versions
            .iter()
            .find(|v| v.id == instance.minecraft_version)
            .with_context(|| format!("Version {} not found", instance.minecraft_version))?
            .clone();

        let version_meta = vm.fetch_version_meta(&entry).await?;

        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Downloading client jar...".to_string(),
            percent: 0.15,
        }).await;

        // Download client jar
        let client_jar = self.download_client(&version_meta, &instance.minecraft_version).await?;

        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Downloading libraries...".to_string(),
            percent: 0.25,
        }).await;

        // Download libraries
        self.download_libraries(&version_meta).await?;

        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Downloading assets...".to_string(),
            percent: 0.55,
        }).await;

        // Download assets
        self.download_assets(&version_meta).await?;

        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Extracting natives...".to_string(),
            percent: 0.85,
        }).await;

        // Extract natives
        let natives_dir = instance.dir().join("natives");
        self.extract_natives(&version_meta, &natives_dir)?;

        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Launching game...".to_string(),
            percent: 0.95,
        }).await;

        // Build classpath
        let classpath = self.build_classpath(&version_meta, &client_jar)?;

        // Build game args
        let game_args = self.build_game_args(&version_meta, instance, account)?;
        let jvm_args = self.build_jvm_args(&version_meta, instance, &natives_dir, &classpath)?;

        // Launch
        let java_bin = Path::new(&java.path).join("bin").join("java");
        let mut cmd = tokio::process::Command::new(&java_bin);

        cmd.args(&jvm_args);
        cmd.arg(&version_meta.main_class);
        cmd.args(&game_args);
        cmd.current_dir(instance.minecraft_dir());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);
        let _ = event_tx.send(LaunchEvent::Started(pid)).await;
        let _ = event_tx.send(LaunchEvent::Progress {
            message: "Game running!".to_string(),
            percent: 1.0,
        }).await;

        // Stream output
        let tx2 = event_tx.clone();
        if let Some(stdout) = child.stdout.take() {
            let tx = tx2.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx.send(LaunchEvent::Log(line)).await;
                }
            });
        }
        if let Some(stderr) = child.stderr.take() {
            let tx = tx2.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx.send(LaunchEvent::Log(format!("[ERR] {line}"))).await;
                }
            });
        }

        let status = child.wait().await?;
        let code = status.code().unwrap_or(-1);
        let _ = event_tx.send(LaunchEvent::Exited(code)).await;

        Ok(())
    }

    async fn download_client(&self, meta: &VersionMeta, version_id: &str) -> Result<PathBuf> {
        let client_path = paths::versions_dir()
            .join(version_id)
            .join(format!("{version_id}.jar"));

        if client_path.exists() {
            return Ok(client_path);
        }

        let url = meta.downloads.as_ref()
            .and_then(|d| d.client.as_ref())
            .map(|c| c.url.clone())
            .with_context(|| "No client download URL found")?;

        download::download_file(&self.client, &url, &client_path, None).await?;
        Ok(client_path)
    }

    async fn download_libraries(&self, meta: &VersionMeta) -> Result<()> {
        let libs_dir = paths::libraries_dir();
        let mut tasks = vec![];

        for lib in &meta.libraries {
            if !should_include_library(lib) {
                continue;
            }

            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    if let Some(path) = &artifact.path {
                        let dest = libs_dir.join(path);
                        let url = artifact.url.clone();
                        let client = self.client.clone();

                        tasks.push(tokio::spawn(async move {
                            download::download_file_if_missing(&client, &url, &dest).await
                                .unwrap_or_else(|e| log::warn!("Lib download failed: {e}"));
                        }));
                    }
                }

                // Download linux natives
                if let Some(natives) = &lib.natives {
                    let key = natives.get("linux").or_else(|| natives.get("linux-x86_64"));
                    if let Some(classifier) = key {
                        if let Some(classifiers) = &downloads.classifiers {
                            if let Some(artifact) = classifiers.get(classifier) {
                                if let Some(path) = &artifact.path {
                                    let dest = libs_dir.join(path);
                                    let url = artifact.url.clone();
                                    let client = self.client.clone();
                                    tasks.push(tokio::spawn(async move {
                                        download::download_file_if_missing(&client, &url, &dest).await
                                            .unwrap_or_else(|e| log::warn!("Native download failed: {e}"));
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    async fn download_assets(&self, meta: &VersionMeta) -> Result<()> {
        let asset_index = match &meta.asset_index {
            Some(ai) => ai.clone(),
            None => return Ok(()),
        };

        let index_path = paths::assets_dir()
            .join("indexes")
            .join(format!("{}.json", asset_index.id));

        if !index_path.exists() {
            download::download_file(&self.client, &asset_index.url, &index_path, None).await?;
        }

        let index_text = std::fs::read_to_string(&index_path)?;
        let index: serde_json::Value = serde_json::from_str(&index_text)?;

        let objects = index.get("objects").and_then(|o| o.as_object()).cloned().unwrap_or_default();
        let objects_dir = paths::assets_dir().join("objects");

        let mut tasks = vec![];

        for (_, obj) in objects {
            let hash = obj.get("hash").and_then(|h| h.as_str()).unwrap_or("").to_string();
            if hash.len() < 2 { continue; }

            let prefix = &hash[..2];
            let dest = objects_dir.join(prefix).join(&hash);

            if dest.exists() { continue; }

            let url = format!("https://resources.download.minecraft.net/{prefix}/{hash}");
            let client = self.client.clone();

            tasks.push(tokio::spawn(async move {
                download::download_file_if_missing(&client, &url, &dest).await
                    .unwrap_or_else(|e| log::warn!("Asset download failed for {hash}: {e}"));
            }));

            // Limit concurrency
            if tasks.len() >= 16 {
                for t in tasks.drain(..) {
                    let _ = t.await;
                }
            }
        }

        for t in tasks {
            let _ = t.await;
        }

        Ok(())
    }

    fn extract_natives(&self, meta: &VersionMeta, natives_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(natives_dir)?;
        let libs_dir = paths::libraries_dir();

        for lib in &meta.libraries {
            if !should_include_library(lib) { continue; }

            if let Some(downloads) = &lib.downloads {
                if let Some(natives) = &lib.natives {
                    let key = natives.get("linux").or_else(|| natives.get("linux-x86_64"));
                    if let Some(classifier) = key {
                        if let Some(classifiers) = &downloads.classifiers {
                            if let Some(artifact) = classifiers.get(classifier) {
                                if let Some(path) = &artifact.path {
                                    let jar_path = libs_dir.join(path);
                                    if jar_path.exists() {
                                        extract_zip(&jar_path, natives_dir, &lib.extract)
                                            .unwrap_or_else(|e| log::warn!("Native extraction failed: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn build_classpath(&self, meta: &VersionMeta, client_jar: &Path) -> Result<String> {
        let libs_dir = paths::libraries_dir();
        let mut paths_set = Vec::new();

        for lib in &meta.libraries {
            if !should_include_library(lib) { continue; }

            if let Some(downloads) = &lib.downloads {
                if let Some(artifact) = &downloads.artifact {
                    if let Some(path) = &artifact.path {
                        let full = libs_dir.join(path);
                        if full.exists() {
                            paths_set.push(full.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        paths_set.push(client_jar.to_string_lossy().to_string());

        Ok(paths_set.join(":"))
    }

    fn build_game_args(&self, meta: &VersionMeta, instance: &Instance, account: &Account) -> Result<Vec<String>> {
        let mut args = Vec::new();

        let assets_dir = paths::assets_dir();
        let asset_index = meta.asset_index.as_ref().map(|ai| ai.id.clone()).unwrap_or_else(|| "legacy".to_string());
        let version_dir = paths::versions_dir().join(&instance.minecraft_version);
        let game_dir = instance.minecraft_dir();

        let replace = |s: &str| -> String {
            s.replace("${auth_player_name}", &account.username)
             .replace("${version_name}", &instance.minecraft_version)
             .replace("${game_directory}", game_dir.to_str().unwrap_or("."))
             .replace("${assets_root}", assets_dir.to_str().unwrap_or("."))
             .replace("${assets_index_name}", &asset_index)
             .replace("${auth_uuid}", &account.uuid)
             .replace("${auth_access_token}", account.access_token.as_deref().unwrap_or("0"))
             .replace("${user_type}", if account.account_type == AccountType::Microsoft { "msa" } else { "legacy" })
             .replace("${version_type}", "GLauncher")
             .replace("${user_properties}", "{}")
             .replace("${clientid}", "GLauncher")
             .replace("${auth_xuid}", account.xuid.as_deref().unwrap_or(""))
        };

        if let Some(arguments) = &meta.arguments {
            if let Some(game) = &arguments.game {
                for arg in game {
                    match arg {
                        serde_json::Value::String(s) => args.push(replace(s)),
                        serde_json::Value::Object(obj) => {
                            // conditional args (features/rules)
                            if let Some(value) = obj.get("value") {
                                match value {
                                    serde_json::Value::String(s) => args.push(replace(s)),
                                    serde_json::Value::Array(arr) => {
                                        for v in arr {
                                            if let Some(s) = v.as_str() {
                                                args.push(replace(s));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else if let Some(mc_args) = &meta.minecraft_arguments {
            // Old format (pre-1.13)
            for arg in mc_args.split_whitespace() {
                args.push(replace(arg));
            }
        }

        // Window size
        if !instance.fullscreen && !instance.game_resolution_auto {
            if let (Some(w), Some(h)) = (instance.window_width, instance.window_height) {
                args.push("--width".to_string());
                args.push(w.to_string());
                args.push("--height".to_string());
                args.push(h.to_string());
            }
        }

        Ok(args)
    }

    fn build_jvm_args(
        &self,
        meta: &VersionMeta,
        instance: &Instance,
        natives_dir: &Path,
        classpath: &str,
    ) -> Result<Vec<String>> {
        let mut args = instance.jvm_settings.build_flags();

        args.push(format!("-Djava.library.path={}", natives_dir.display()));
        args.push(format!("-Dminecraft.launcher.brand=GLauncher"));
        args.push(format!("-Dminecraft.launcher.version=1.0.0"));

        // Version-specified JVM args
        if let Some(arguments) = &meta.arguments {
            if let Some(jvm) = &arguments.jvm {
                let version_dir = paths::versions_dir().join(&instance.minecraft_version);
                let natives_str = natives_dir.to_str().unwrap_or(".");
                let assets_dir = paths::assets_dir();

                let replace = |s: &str| -> String {
                    s.replace("${natives_directory}", natives_str)
                     .replace("${launcher_name}", "GLauncher")
                     .replace("${launcher_version}", "1.0.0")
                     .replace("${classpath}", classpath)
                };

                for arg in jvm {
                    match arg {
                        serde_json::Value::String(s) => args.push(replace(s)),
                        serde_json::Value::Object(obj) => {
                            if check_rules(obj.get("rules")) {
                                if let Some(value) = obj.get("value") {
                                    match value {
                                        serde_json::Value::String(s) => args.push(replace(s)),
                                        serde_json::Value::Array(arr) => {
                                            for v in arr {
                                                if let Some(s) = v.as_str() {
                                                    args.push(replace(s));
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        } else {
            // Old-format fallback
            args.push(format!("-Djava.library.path={}", natives_dir.display()));
            args.push("-cp".to_string());
            args.push(classpath.to_string());
        }

        // Always append classpath at end if not already set by version meta
        if !args.iter().any(|a| a == "-cp" || a == "--class-path") {
            args.push("-cp".to_string());
            args.push(classpath.to_string());
        }

        Ok(args)
    }
}

fn should_include_library(lib: &Library) -> bool {
    let Some(rules) = &lib.rules else { return true; };

    for rule in rules {
        let applies_to_linux = rule.os.as_ref().map_or(true, |os| {
            os.name.as_deref() == Some("linux") || os.name.is_none()
        });

        if applies_to_linux {
            return rule.action == "allow";
        }
    }

    // Default: allow if no rule matched
    true
}

fn check_rules(rules: Option<&serde_json::Value>) -> bool {
    let Some(rules) = rules else { return true; };
    let Some(arr) = rules.as_array() else { return true; };

    for rule in arr {
        let action = rule.get("action").and_then(|a| a.as_str()).unwrap_or("allow");
        if let Some(os) = rule.get("os") {
            let name = os.get("name").and_then(|n| n.as_str());
            if name == Some("linux") || name.is_none() {
                return action == "allow";
            }
        } else {
            return action == "allow";
        }
    }

    true
}

fn extract_zip(
    jar_path: &Path,
    dest_dir: &Path,
    extract_config: &Option<crate::minecraft::versions::ExtractConfig>,
) -> Result<()> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let exclude: Vec<String> = extract_config
        .as_ref()
        .and_then(|e| e.exclude.as_ref())
        .cloned()
        .unwrap_or_default();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if exclude.iter().any(|ex| name.starts_with(ex)) {
            continue;
        }
        if name.ends_with('/') { continue; }

        let out_path = dest_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut file, &mut out)?;
    }

    Ok(())
}

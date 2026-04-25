use serde::{Deserialize, Serialize};
use anyhow::Result;

// CurseForge requires an API key. Users can provide their own.
// We use the open-source API proxy or the official API key.
const CURSEFORGE_API: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_GAME_ID: u32 = 432; // Minecraft

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeProject {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: u64,
    pub logo: Option<CurseForgeLogo>,
    pub categories: Vec<CurseForgeCategory>,
    pub links: Option<CurseForgeLinks>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub latest_files_indexes: Vec<CurseForgeFileIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeLogo {
    pub url: String,
    #[serde(rename = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeCategory {
    pub id: u64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeLinks {
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFileIndex {
    #[serde(rename = "gameVersion")]
    pub game_version: String,
    #[serde(rename = "fileId")]
    pub file_id: u64,
    pub filename: String,
    #[serde(rename = "releaseType")]
    pub release_type: u32,
    #[serde(rename = "modLoader")]
    pub mod_loader: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFile {
    pub id: u64,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    #[serde(rename = "fileLength")]
    pub file_length: u64,
    pub hashes: Vec<CurseForgeHash>,
    #[serde(rename = "gameVersions")]
    pub game_versions: Vec<String>,
    pub dependencies: Vec<CurseForgeDependency>,
    #[serde(rename = "releaseType")]
    pub release_type: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeDependency {
    #[serde(rename = "modId")]
    pub mod_id: u64,
    #[serde(rename = "relationType")]
    pub relation_type: u32,
}

pub struct CurseForgeClient {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl CurseForgeClient {
    pub fn new(client: reqwest::Client, api_key: Option<String>) -> Self {
        Self { client, api_key }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(key) = &self.api_key {
            headers.insert(
                "x-api-key",
                reqwest::header::HeaderValue::from_str(key).unwrap(),
            );
        }
        headers
    }

    pub async fn search(
        &self,
        query: &str,
        mc_version: Option<&str>,
        loader_id: Option<u32>,
        page: u32,
    ) -> Result<Vec<CurseForgeProject>> {
        if self.api_key.is_none() {
            anyhow::bail!("CurseForge API key is required. Please add your API key in Settings.");
        }

        let class_id = 6u32; // Mods

        let mut req = self.client
            .get(format!("{CURSEFORGE_API}/mods/search"))
            .headers(self.headers())
            .query(&[
                ("gameId", CURSEFORGE_GAME_ID.to_string()),
                ("classId", class_id.to_string()),
                ("searchFilter", query.to_string()),
                ("index", (page * 20).to_string()),
                ("pageSize", "20".to_string()),
                ("sortField", "2".to_string()),  // downloads
                ("sortOrder", "desc".to_string()),
            ]);

        if let Some(mc) = mc_version {
            req = req.query(&[("gameVersion", mc)]);
        }

        if let Some(loader) = loader_id {
            req = req.query(&[("modLoaderType", loader.to_string())]);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await?;
            anyhow::bail!("CurseForge search failed: {text}");
        }

        let data: serde_json::Value = resp.json().await?;
        let projects: Vec<CurseForgeProject> = serde_json::from_value(
            data.get("data").cloned().unwrap_or_default()
        )?;

        Ok(projects)
    }

    pub async fn get_mod_files(
        &self,
        mod_id: u64,
        mc_version: Option<&str>,
    ) -> Result<Vec<CurseForgeFile>> {
        let mut req = self.client
            .get(format!("{CURSEFORGE_API}/mods/{mod_id}/files"))
            .headers(self.headers())
            .query(&[("pageSize", "20")]);

        if let Some(mc) = mc_version {
            req = req.query(&[("gameVersion", mc)]);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to get files for mod {mod_id}");
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(serde_json::from_value(data.get("data").cloned().unwrap_or_default())?)
    }

    /// CurseForge sometimes withholds download URLs. This tries to construct it.
    pub fn get_download_url(file: &CurseForgeFile, _mod_id: u64) -> String {
        if let Some(url) = &file.download_url {
            return url.clone();
        }
        // Fallback construction
        let id = file.id;
        let id_str = id.to_string();
        let (a, b) = id_str.split_at(4.min(id_str.len()));
        format!(
            "https://edge.forgecdn.net/files/{a}/{b}/{}",
            file.file_name
        )
    }
}

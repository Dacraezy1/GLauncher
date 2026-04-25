use serde::{Deserialize, Serialize};
use anyhow::Result;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthProject>,
    pub total_hits: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthProject {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub followers: u64,
    pub icon_url: Option<String>,
    pub project_type: String,
    pub versions: Vec<String>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub license: Option<String>,
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    pub display_categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub changelog: Option<String>,
    pub dependencies: Vec<ModrinthDependency>,
    pub game_versions: Vec<String>,
    pub version_type: String,  // release, beta, alpha
    pub loaders: Vec<String>,
    pub featured: bool,
    pub files: Vec<ModrinthFile>,
    pub date_published: String,
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub file_name: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthFile {
    pub hashes: std::collections::HashMap<String, String>,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub limit: u32,
    pub offset: u32,
    pub mc_version: Option<String>,
    pub loader: Option<String>,
    pub project_type: Option<String>,  // "mod", "modpack", "resourcepack", "shader"
    pub categories: Vec<String>,
    pub sort_by: SortBy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortBy {
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            limit: 20,
            offset: 0,
            mc_version: None,
            loader: None,
            project_type: Some("mod".to_string()),
            categories: vec![],
            sort_by: SortBy::Relevance,
        }
    }
}

pub struct ModrinthClient {
    client: reqwest::Client,
}

impl ModrinthClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<ModrinthSearchResult> {
        let mut facets: Vec<Vec<String>> = Vec::new();

        if let Some(pt) = &query.project_type {
            facets.push(vec![format!("project_type:{pt}")]);
        }

        if let Some(mc_ver) = &query.mc_version {
            facets.push(vec![format!("versions:{mc_ver}")]);
        }

        if let Some(loader) = &query.loader {
            let loader_lower = loader.to_lowercase();
            facets.push(vec![format!("categories:{loader_lower}")]);
        }

        for cat in &query.categories {
            facets.push(vec![format!("categories:{cat}")]);
        }

        let facets_str = if !facets.is_empty() {
            let inner: Vec<String> = facets
                .iter()
                .map(|group| {
                    let items: Vec<String> = group.iter().map(|s| format!("\"{s}\"")).collect();
                    format!("[{}]", items.join(","))
                })
                .collect();
            format!("[{}]", inner.join(","))
        } else {
            String::new()
        };

        let index = match query.sort_by {
            SortBy::Relevance => "relevance",
            SortBy::Downloads => "downloads",
            SortBy::Follows => "follows",
            SortBy::Newest => "newest",
            SortBy::Updated => "updated",
        };

        let mut req = self.client
            .get(format!("{MODRINTH_API}/search"))
            .query(&[
                ("query", query.query.as_str()),
                ("limit", &query.limit.to_string()),
                ("offset", &query.offset.to_string()),
                ("index", index),
            ]);

        if !facets_str.is_empty() {
            req = req.query(&[("facets", facets_str.as_str())]);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await?;
            anyhow::bail!("Modrinth search failed: {text}");
        }

        Ok(resp.json().await?)
    }

    pub async fn get_project_versions(
        &self,
        project_id: &str,
        mc_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<ModrinthVersion>> {
        let mut req = self.client
            .get(format!("{MODRINTH_API}/project/{project_id}/version"));

        if let Some(mc) = mc_version {
            req = req.query(&[("game_versions", format!("[\"{mc}\"]"))]);
        }

        if let Some(l) = loader {
            req = req.query(&[("loaders", format!("[\"{}\"]", l.to_lowercase()))]);
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to get versions for project {project_id}");
        }

        Ok(resp.json().await?)
    }

    pub async fn get_project(&self, id_or_slug: &str) -> Result<ModrinthProject> {
        let resp = self.client
            .get(format!("{MODRINTH_API}/project/{id_or_slug}"))
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Project not found: {id_or_slug}");
        }

        Ok(resp.json().await?)
    }

    pub async fn download_mod(
        &self,
        version: &ModrinthVersion,
        dest_dir: &std::path::Path,
        progress_tx: Option<tokio::sync::mpsc::Sender<crate::utils::download::DownloadProgress>>,
    ) -> Result<std::path::PathBuf> {
        let file = version.files.iter()
            .find(|f| f.primary)
            .or_else(|| version.files.first())
            .ok_or_else(|| anyhow::anyhow!("No files found for version {}", version.id))?;

        let dest = dest_dir.join(&file.filename);
        crate::utils::download::download_file(&self.client, &file.url, &dest, progress_tx).await?;

        Ok(dest)
    }
}

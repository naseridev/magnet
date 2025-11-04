use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub owner: String,
    pub repo: String,
    pub version: String,
    pub installed_at: String,
    pub binaries: Vec<String>,
    pub install_path: String,
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub assets: Vec<Asset>,
    pub published_at: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Repository {
    pub name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub stargazers_count: u32,
    pub language: Option<String>,
    pub full_name: String,
    pub size: u32,
    #[serde(rename = "fork")]
    pub is_fork: bool,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub items: Vec<Repository>,
}

#[derive(Debug, Deserialize)]
pub struct UserSearchResult {
    pub items: Vec<User>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitResponse {
    pub rate: RateLimit,
}

#[derive(Debug, Deserialize)]
pub struct RateLimit {
    pub remaining: u32,
}

#[derive(Debug)]
pub struct DumpStats {
    pub downloaded: usize,
    pub failed: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone)]
pub struct AssetScore {
    pub asset: Asset,
    pub score: i32,
    pub reason: String,
}

impl Package {
    pub fn new(
        owner: String,
        repo: String,
        version: String,
        binaries: Vec<String>,
        install_path: String,
    ) -> Self {
        Self {
            owner,
            repo,
            version,
            installed_at: chrono::Utc::now().to_rfc3339(),
            binaries,
            install_path,
        }
    }
}

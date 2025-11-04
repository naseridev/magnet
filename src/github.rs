use anyhow::Result;
use reqwest::{Client, StatusCode};
use std::time::Duration;

use crate::types::{RateLimitResponse, Release, Repository, SearchResult, UserSearchResult};

const GITHUB_API_BASE: &str = "https://api.github.com";
const MAX_RETRIES: u32 = 3;

pub struct GitHubClient {
    client: Client,
}

impl GitHubClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn fetch_release(
        &self,
        owner: &str,
        repo: &str,
        version: Option<&str>,
    ) -> Result<Release> {
        let url = if let Some(ver) = version {
            format!(
                "{}/repos/{}/{}/releases/tags/{}",
                GITHUB_API_BASE, owner, repo, ver
            )
        } else {
            format!(
                "{}/repos/{}/{}/releases/latest",
                GITHUB_API_BASE, owner, repo
            )
        };

        let response = self.retry_request(|| self.client.get(&url).send()).await?;

        if !response.status().is_success() {
            anyhow::bail!("Release not found for {}/{}", owner, repo);
        }

        Ok(response.json().await?)
    }

    pub async fn fetch_repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let url = format!("{}/repos/{}/{}", GITHUB_API_BASE, owner, repo);
        let response = self.retry_request(|| self.client.get(&url).send()).await?;

        if !response.status().is_success() {
            anyhow::bail!("Repository not found: {}/{}", owner, repo);
        }

        Ok(response.json().await?)
    }

    pub async fn fetch_user_repos(&self, username: &str) -> Result<Vec<Repository>> {
        let mut repos = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/users/{}/repos?per_page=100&page={}",
                GITHUB_API_BASE, username, page
            );

            let response = self.retry_request(|| self.client.get(&url).send()).await?;

            if !response.status().is_success() {
                anyhow::bail!("Failed to fetch repositories: {}", response.status());
            }

            let data: Vec<Repository> = response.json().await?;

            if data.is_empty() {
                break;
            }

            repos.extend(data);
            page += 1;
        }

        self.check_rate_limit().await.ok();
        Ok(repos)
    }

    pub async fn search_repositories(&self, query: &str, limit: usize) -> Result<SearchResult> {
        let url = format!(
            "{}/search/repositories?q={}&per_page={}",
            GITHUB_API_BASE, query, limit
        );

        let response = self.retry_request(|| self.client.get(&url).send()).await?;

        if !response.status().is_success() {
            anyhow::bail!("Search failed: {}", response.status());
        }

        Ok(response.json().await?)
    }

    pub async fn search_users(&self, query: &str, limit: usize) -> Result<UserSearchResult> {
        let url = format!(
            "{}/search/users?q={}&per_page={}",
            GITHUB_API_BASE, query, limit
        );

        let response = self.retry_request(|| self.client.get(&url).send()).await?;

        if !response.status().is_success() {
            anyhow::bail!("User search failed: {}", response.status());
        }

        Ok(response.json().await?)
    }

    pub async fn check_rate_limit(&self) -> Result<()> {
        let url = format!("{}/rate_limit", GITHUB_API_BASE);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let data: RateLimitResponse = response.json().await?;
            if data.rate.remaining < 10 {
                eprintln!(
                    "WARNING: GitHub API rate limit low: {} remaining",
                    data.rate.remaining
                );
            }
        }

        Ok(())
    }

    pub async fn retry_request<F, Fut>(&self, mut request_fn: F) -> Result<reqwest::Response>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<reqwest::Response, reqwest::Error>>,
    {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            match request_fn().await {
                Ok(response) => {
                    if response.status().is_success() || response.status() == StatusCode::NOT_FOUND
                    {
                        return Ok(response);
                    }

                    if response.status() == StatusCode::FORBIDDEN
                        || response.status() == StatusCode::TOO_MANY_REQUESTS
                    {
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Request failed: {}", last_error.unwrap()))
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

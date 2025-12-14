use clap::{Arg, Command};
use regex::Regex;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use zip::ZipArchive;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;
const GITHUB_API_BASE: &str = "https://api.github.com";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("magnet")
        .version("2.0")
        .author("Nima Naseri <nerdnull@proton.me>")
        .about("Industrial strength GitHub repository scraper")
        .arg(
            Arg::new("username")
                .help("GitHub username to scrape repositories from")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("token")
                .long("token")
                .short('t')
                .help("GitHub personal access token (avoids rate limits)")
                .value_name("TOKEN")
                .env("GITHUB_TOKEN"),
        )
        .arg(
            Arg::new("language")
                .long("language")
                .short('l')
                .help("Filter by programming language")
                .value_name("LANG"),
        )
        .arg(
            Arg::new("min-stars")
                .long("min-stars")
                .short('s')
                .help("Minimum number of stars")
                .value_name("NUM")
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("only-original")
                .long("only-original")
                .short('o')
                .help("Original repositories only (no forks)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("regex")
                .long("regex")
                .short('r')
                .help("Filter repository names by regex pattern")
                .value_name("PATTERN"),
        )
        .arg(
            Arg::new("max-size")
                .long("max-size")
                .short('m')
                .help("Maximum repository size in MB")
                .value_name("MB")
                .value_parser(clap::value_parser!(u32)),
        )
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .short('p')
                .help("Parallel download count")
                .value_name("COUNT")
                .value_parser(clap::value_parser!(usize))
                .default_value("3"),
        )
        .get_matches();

    let username = matches.get_one::<String>("username").unwrap();
    let token = matches.get_one::<String>("token");
    let language_filter = matches.get_one::<String>("language");
    let min_stars = matches.get_one::<u32>("min-stars").unwrap_or(&0);
    let max_size = matches.get_one::<u32>("max-size");
    let only_original = matches.get_flag("only-original");
    let regex_pattern = matches.get_one::<String>("regex");
    let parallel_count = *matches.get_one::<usize>("parallel").unwrap();

    let regex_filter = if let Some(pattern) = regex_pattern {
        match Regex::new(pattern) {
            Ok(regex) => Some(regex),
            Err(e) => {
                eprintln!("Invalid regex: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    fs::create_dir_all(username)?;

    let start_time = Instant::now();
    let scraper = Scraper::new(token.cloned())?;

    println!("Scanning repositories for: {}", username);

    if let Some(lang) = language_filter {
        println!("Language: {}", lang);
    }

    if *min_stars > 0 {
        println!("Min stars: {}", min_stars);
    }

    if let Some(size) = max_size {
        println!("Max size: {}MB", size);
    }

    if only_original {
        println!("Original only: yes");
    }

    if let Some(pattern) = regex_pattern {
        println!("Regex: {}", pattern);
    }

    println!("Parallel: {}", parallel_count);
    if token.is_none() {
        println!("WARNING: No GitHub token provided - API rate limits apply");
    }
    println!();

    let repos = scraper.fetch_all_repos(username).await?;
    let filtered_repos = filter_repos(
        repos,
        language_filter,
        *min_stars,
        max_size,
        only_original,
        &regex_filter,
    );

    println!(
        "Found {} repositories matching criteria",
        filtered_repos.len()
    );

    if filtered_repos.is_empty() {
        println!("No repositories to download");
        return Ok(());
    }

    println!();

    let progress = Arc::new(ProgressTracker::new(filtered_repos.len()));
    let semaphore = Arc::new(Semaphore::new(parallel_count));
    let scraper = Arc::new(scraper);
    let mut tasks = Vec::new();

    for repo in filtered_repos {
        let scraper = scraper.clone();
        let username = username.clone();
        let progress = progress.clone();
        let semaphore = semaphore.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let result = scraper.download_repo(&repo, &username).await;
            progress.report_completion(repo.name.clone(), result).await;
        });

        tasks.push(task);
    }

    for task in tasks {
        task.await?;
    }

    let stats = progress.get_stats().await;
    let elapsed = start_time.elapsed();

    println!();
    println!("Results:");
    println!("Downloaded: {}", stats.downloaded);
    println!("Failed: {}", stats.failed);
    println!("Total size: {} MB", stats.total_size / 1024 / 1024);
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    if stats.downloaded > 0 {
        println!(
            "Speed: {:.1} MB/s",
            (stats.total_size as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64()
        );
    }

    Ok(())
}

#[derive(Debug)]
struct Stats {
    downloaded: usize,
    failed: usize,
    total_size: u64,
}

struct ProgressTracker {
    total: usize,
    completed: Mutex<usize>,
    downloaded: Mutex<usize>,
    failed: Mutex<usize>,
    total_size: Mutex<u64>,
}

impl ProgressTracker {
    fn new(total: usize) -> Self {
        Self {
            total,
            completed: Mutex::new(0),
            downloaded: Mutex::new(0),
            failed: Mutex::new(0),
            total_size: Mutex::new(0),
        }
    }

    async fn report_completion(&self, name: String, result: Result<u64, String>) {
        let mut completed = self.completed.lock().await;
        *completed += 1;
        let current = *completed;

        match result {
            Ok(size) => {
                let mut downloaded = self.downloaded.lock().await;
                let mut total_size = self.total_size.lock().await;
                *downloaded += 1;
                *total_size += size;
                println!("[{}/{}] {} ({} KB)", current, self.total, name, size / 1024);
            }
            Err(e) => {
                let mut failed = self.failed.lock().await;
                *failed += 1;
                println!("[{}/{}] {} FAILED: {}", current, self.total, name, e);
            }
        }
    }

    async fn get_stats(&self) -> Stats {
        Stats {
            downloaded: *self.downloaded.lock().await,
            failed: *self.failed.lock().await,
            total_size: *self.total_size.lock().await,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RepoInfo {
    name: String,
    html_url: String,
    language: Option<String>,
    #[serde(rename = "stargazers_count")]
    stars: u32,
    size: u32,
    #[serde(rename = "fork")]
    is_fork: bool,
    default_branch: String,
}

#[derive(Deserialize)]
struct RateLimitResponse {
    rate: RateLimit,
}

#[derive(Deserialize)]
struct RateLimit {
    remaining: u32,
}

struct Scraper {
    client: Client,
    token: Option<String>,
}

impl Scraper {
    fn new(token: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());
        headers.insert("User-Agent", "magnet/2.0".parse().unwrap());

        if let Some(ref token) = token {
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .default_headers(headers)
            .build()?;

        Ok(Self { client, token })
    }

    async fn fetch_all_repos(&self, username: &str) -> Result<Vec<RepoInfo>, String> {
        let mut repos = Vec::new();
        let mut page = 1;

        loop {
            let url = format!(
                "{}/users/{}/repos?per_page=100&page={}",
                GITHUB_API_BASE, username, page
            );

            let response = self.retry_request(|| self.client.get(&url).send()).await?;

            if !response.status().is_success() {
                return Err(format!("GitHub API error: {}", response.status()));
            }

            let data: Vec<RepoInfo> = response.json().await.map_err(|e| e.to_string())?;

            if data.is_empty() {
                break;
            }

            repos.extend(data);
            page += 1;
        }

        if self.token.is_none() {
            self.check_rate_limit().await.ok();
        }

        Ok(repos)
    }

    async fn check_rate_limit(&self) -> Result<(), String> {
        let url = format!("{}/rate_limit", GITHUB_API_BASE);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if response.status().is_success() {
            let data: RateLimitResponse = response.json().await.map_err(|e| e.to_string())?;
            if data.rate.remaining < 10 {
                eprintln!(
                    "WARNING: GitHub API rate limit low: {} remaining",
                    data.rate.remaining
                );
            }
        }

        Ok(())
    }

    async fn retry_request<F, Fut>(&self, mut request_fn: F) -> Result<reqwest::Response, String>
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
                        tokio::time::sleep(Duration::from_millis(
                            RETRY_DELAY_MS * 2_u64.pow(attempt),
                        ))
                        .await;
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        tokio::time::sleep(Duration::from_millis(
                            RETRY_DELAY_MS * 2_u64.pow(attempt),
                        ))
                        .await;
                    }
                }
            }
        }

        Err(last_error.unwrap().to_string())
    }

    async fn download_repo(&self, repo: &RepoInfo, username: &str) -> Result<u64, String> {
        let repo_path = Path::new(username).join(&repo.name);

        if repo_path.exists() {
            if let Ok(size) = get_dir_size(&repo_path) {
                return Ok(size);
            }
        }

        let branch = &repo.default_branch;
        let zip_url = format!("{}/archive/refs/heads/{}.zip", repo.html_url, branch);

        match self.download_and_extract(&zip_url, &repo_path).await {
            Ok(size) => Ok(size),
            Err(e) => {
                let fallback_branches = ["main", "master", "develop", "trunk"];
                for fallback in &fallback_branches {
                    if *fallback == branch {
                        continue;
                    }

                    let fallback_url =
                        format!("{}/archive/refs/heads/{}.zip", repo.html_url, fallback);

                    if let Ok(size) = self.download_and_extract(&fallback_url, &repo_path).await {
                        return Ok(size);
                    }
                }

                Err(format!("Failed to download: {}", e))
            }
        }
    }

    async fn download_and_extract(&self, url: &str, repo_path: &Path) -> Result<u64, String> {
        let response = self.retry_request(|| self.client.get(url).send()).await?;

        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }

        let bytes = response.bytes().await.map_err(|e| e.to_string())?;
        let zip_file = format!("{}.zip", repo_path.to_string_lossy());

        fs::write(&zip_file, &bytes).map_err(|e| e.to_string())?;

        let result = extract_zip(&zip_file, repo_path);
        fs::remove_file(&zip_file).ok();

        match result {
            Ok(_) => Ok(get_dir_size(repo_path).unwrap_or(0)),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn filter_repos(
    repos: Vec<RepoInfo>,
    language_filter: Option<&String>,
    min_stars: u32,
    max_size: Option<&u32>,
    only_original: bool,
    regex_filter: &Option<Regex>,
) -> Vec<RepoInfo> {
    repos
        .into_iter()
        .filter(|repo| {
            if only_original && repo.is_fork {
                return false;
            }

            if repo.stars < min_stars {
                return false;
            }

            if let Some(max_size_mb) = max_size {
                if repo.size > *max_size_mb * 1024 {
                    return false;
                }
            }

            if let Some(lang_filter) = language_filter {
                match &repo.language {
                    Some(lang) => {
                        if lang.to_lowercase() != lang_filter.to_lowercase() {
                            return false;
                        }
                    }
                    None => return false,
                }
            }

            if let Some(regex) = regex_filter {
                if !regex.is_match(&repo.name) {
                    return false;
                }
            }

            true
        })
        .collect()
}

fn extract_zip(zip_path: &str, repo_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        let components: Vec<_> = outpath.components().collect();
        let outpath = if components.len() > 1 {
            repo_path.join(components[1..].iter().collect::<PathBuf>())
        } else {
            continue;
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn get_dir_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut size = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            size += metadata.len();
        } else if metadata.is_dir() {
            size += get_dir_size(&entry.path()).unwrap_or(0);
        }
    }
    Ok(size)
}

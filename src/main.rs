use clap::{Arg, Command};
use regex::Regex;
use reqwest;
use serde_json::Value;
use std::fs;
use std::io::copy;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tokio::sync::{Mutex, Semaphore};
use zip::ZipArchive;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("magnet")
        .version("1.0")
        .author("Nima Naseri")
        .about("Industrial strength GitHub repository scraper")
        .arg(
            Arg::new("username")
                .help("GitHub username to scrape repositories from")
                .required(true)
                .index(1),
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(300))
        .user_agent("curl/8.4.0")
        .build()?;

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
    println!();

    let repos = fetch_all_repos(&client, username).await?;
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
    let mut tasks = Vec::new();

    for repo in filtered_repos {
        let client = client.clone();
        let username = username.clone();
        let progress = progress.clone();
        let semaphore = semaphore.clone();

        let task = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            let result = download_repo(&client, &repo, &username).await;
            progress.report_completion(repo.name, result).await;
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

#[derive(Debug, Clone)]
struct RepoInfo {
    name: String,
    html_url: String,
    language: Option<String>,
    stars: u32,
    size: u32,
    is_fork: bool,
}

async fn fetch_all_repos(
    client: &reqwest::Client,
    username: &str,
) -> Result<Vec<RepoInfo>, Box<dyn std::error::Error>> {
    let mut repos = Vec::new();
    let mut page = 1;

    loop {
        let url = format!(
            "https://api.github.com/users/{}/repos?per_page=100&page={}",
            username, page
        );

        let response = client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("X-GitHub-Media-Type", "github.v3")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("GitHub API error: {}", response.status()).into());
        }

        let data: Value = response.json().await?;

        if let Some(repos_array) = data.as_array() {
            if repos_array.is_empty() {
                break;
            }

            for repo in repos_array {
                repos.push(RepoInfo {
                    name: repo["name"].as_str().unwrap_or("unknown").to_string(),
                    html_url: repo["html_url"].as_str().unwrap_or("").to_string(),
                    language: repo["language"].as_str().map(|s| s.to_string()),
                    stars: repo["stargazers_count"].as_u64().unwrap_or(0) as u32,
                    size: repo["size"].as_u64().unwrap_or(0) as u32,
                    is_fork: repo["fork"].as_bool().unwrap_or(false),
                });
            }

            page += 1;
        } else {
            break;
        }
    }

    Ok(repos)
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

async fn download_repo(
    client: &reqwest::Client,
    repo: &RepoInfo,
    username: &str,
) -> Result<u64, String> {
    let repo_path = Path::new(username).join(&repo.name);

    if repo_path.exists() {
        if let Ok(size) = get_dir_size(&repo_path) {
            return Ok(size);
        }
    }

    let branches = ["main", "master", "develop", "trunk"];

    for branch in &branches {
        let zip_url = format!("{}/archive/refs/heads/{}.zip", repo.html_url, branch);

        match download_and_extract(client, &zip_url, &repo_path).await {
            Ok(size) => return Ok(size),
            Err(_) => continue,
        }
    }

    Err("All branches failed".to_string())
}

async fn download_and_extract(
    client: &reqwest::Client,
    url: &str,
    repo_path: &Path,
) -> Result<u64, Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err("Download failed".into());
    }

    let bytes = response.bytes().await?;
    let zip_file = format!("{}.zip", repo_path.to_string_lossy());

    fs::write(&zip_file, &bytes)?;

    let result = extract_zip(&zip_file, repo_path);
    fs::remove_file(&zip_file).ok();

    match result {
        Ok(_) => Ok(get_dir_size(repo_path).unwrap_or(0)),
        Err(e) => Err(e),
    }
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
            repo_path.join(components[1..].iter().collect::<std::path::PathBuf>())
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

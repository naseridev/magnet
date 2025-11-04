use anyhow::Result;
use regex::Regex;
use reqwest::Client;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

use crate::archive;
use crate::cli::Commands;
use crate::colors;
use crate::github::GitHubClient;
use crate::install;
use crate::registry::Registry;
use crate::types::{DumpStats, Package, Repository};
use crate::utils;

const VERSION: &str = "3.0.0";
const USER_AGENT: &str = "Magnet-Package-Manager";
const MAX_RETRIES: u32 = 3;

pub struct PackageManager {
    github: GitHubClient,
    verbose: bool,
}

impl PackageManager {
    pub fn new(token: Option<String>, verbose: bool) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".parse()?);
        headers.insert("User-Agent", format!("{}/{}", USER_AGENT, VERSION).parse()?);

        if let Some(token) = token {
            headers.insert("Authorization", format!("Bearer {}", token).parse()?);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .default_headers(headers)
            .build()?;

        Ok(Self {
            github: GitHubClient::new(client),
            verbose,
        })
    }

    pub async fn execute(&self, command: Commands) -> Result<()> {
        match command {
            Commands::Install {
                package,
                version,
                global,
                force,
            } => {
                let (owner, repo) = utils::parse_package(&package)?;
                self.install_package(&owner, &repo, version.as_deref(), global, force)
                    .await
            }

            Commands::Uninstall { package, global } => {
                let (owner, repo) = utils::parse_package(&package)?;
                self.uninstall_package(&owner, &repo, global).await
            }

            Commands::List { verbose } => {
                self.list_packages(false, verbose).await?;
                let global_registry = Registry::load(true)?;
                if !global_registry.is_empty() {
                    println!();
                    self.list_packages(true, verbose).await?;
                }
                Ok(())
            }

            Commands::Update { package, global } => {
                if let Some(pkg) = package {
                    let (owner, repo) = utils::parse_package(&pkg)?;
                    self.update_package(&owner, &repo, global).await
                } else {
                    self.update_all_packages(global).await
                }
            }

            Commands::Info { package } => {
                let (owner, repo) = utils::parse_package(&package)?;
                self.show_info(&owner, &repo).await
            }

            Commands::Search {
                query,
                limit,
                by_user,
            } => self.search(&query, limit, by_user).await,

            Commands::Clean { global } => self.clean_packages(global).await,

            Commands::Dump {
                username,
                language,
                min_stars,
                only_original,
                regex,
                max_size,
                parallel,
                output,
            } => {
                let output_dir = output.unwrap_or_else(|| PathBuf::from(&username));
                self.dump_repositories(
                    &username,
                    language,
                    min_stars.unwrap_or(0),
                    max_size,
                    only_original,
                    regex,
                    parallel,
                    output_dir,
                )
                .await
            }
        }
    }

    async fn install_package(
        &self,
        owner: &str,
        repo: &str,
        version: Option<&str>,
        global: bool,
        force: bool,
    ) -> Result<()> {
        let package_key = format!("{}/{}", owner, repo);
        let mut registry = Registry::load(global)?;

        if !force && registry.get(&package_key).is_some() {
            let pkg = registry.get(&package_key).unwrap();
            colors::print_warning(
                "Already",
                &format!(
                    "{} {}",
                    colors::package_name(&package_key),
                    colors::version(&format!("({})", pkg.version))
                ),
            );
            colors::print_info("Hint", "Use --force to reinstall");
            return Ok(());
        }

        colors::print_progress("Installing", &colors::package_name(&package_key));

        let release = self.github.fetch_release(owner, repo, version).await?;

        if self.verbose {
            colors::print_info(
                "Release",
                &format!("{} ({})", release.name, colors::version(&release.tag_name)),
            );
        }

        let install_dir = utils::get_install_dir(global)?;
        fs::create_dir_all(&install_dir)?;

        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;

        if self.verbose {
            colors::print_info("Platform", &format!("{} ({})", os, arch));
        }

        let asset = install::find_best_asset(&release.assets, os, arch, repo)
            .ok_or_else(|| anyhow::anyhow!("No compatible binary found for {} ({})", os, arch))?;

        if self.verbose {
            colors::print_info(
                "Asset",
                &format!(
                    "{} ({})",
                    colors::bold(&asset.asset.name),
                    colors::dim(&asset.reason)
                ),
            );
        }

        let binaries = install::download_and_install_asset(
            self.github.client(),
            &asset.asset,
            &install_dir,
            repo,
        )
        .await?;

        if binaries.is_empty() {
            anyhow::bail!("No binaries found in the downloaded asset");
        }

        let package = Package::new(
            owner.to_string(),
            repo.to_string(),
            release.tag_name.clone(),
            binaries.clone(),
            install_dir.to_string_lossy().to_string(),
        );

        registry.insert(package_key.clone(), package);
        registry.save(global)?;

        if !global {
            utils::setup_path(&install_dir)?;
        }

        println!();
        colors::print_success("Installed", &colors::package_name(&package_key));
        println!(
            "  {} {}",
            colors::dim("Version:"),
            colors::version(&release.tag_name)
        );
        println!("  {} {}", colors::dim("Binaries:"), binaries.join(", "));
        println!(
            "  {} {}",
            colors::dim("Location:"),
            colors::path(&install_dir.display().to_string())
        );

        if !global && cfg!(target_os = "windows") {
            println!();
            colors::print_info(
                "Note",
                "Restart your terminal for PATH changes to take effect",
            );
        }

        Ok(())
    }

    async fn uninstall_package(&self, owner: &str, repo: &str, global: bool) -> Result<()> {
        let package_key = format!("{}/{}", owner, repo);
        let mut registry = Registry::load(global)?;

        if let Some(package) = registry.remove(&package_key) {
            let install_path = PathBuf::from(&package.install_path);

            for binary in &package.binaries {
                let binary_path = install_path.join(binary);
                if binary_path.exists() {
                    fs::remove_file(&binary_path)?;
                }
            }

            registry.save(global)?;
            colors::print_success("Removed", &colors::package_name(&package_key));
        } else {
            colors::print_error(
                "Error",
                &format!("Package not found: {}", colors::package_name(&package_key)),
            );
            anyhow::bail!("Package not found: {}", package_key);
        }

        Ok(())
    }

    async fn list_packages(&self, global: bool, verbose: bool) -> Result<()> {
        let registry = Registry::load(global)?;

        if registry.is_empty() {
            colors::print_info(
                if global { "Global" } else { "Local" },
                "No packages installed",
            );
            return Ok(());
        }

        println!(
            "{} {}",
            colors::header(if global { "Global" } else { "Local" }),
            colors::dim(&format!("({} packages)", registry.len()))
        );
        println!();

        for (key, package) in registry.iter() {
            if verbose {
                println!(
                    "  {} {}",
                    colors::package_name(key),
                    colors::version(&package.version)
                );
                println!("    {} {}", colors::dim("Installed:"), package.installed_at);
                println!(
                    "    {} {}",
                    colors::dim("Location:"),
                    colors::path(&package.install_path)
                );
                println!(
                    "    {} {}",
                    colors::dim("Binaries:"),
                    package.binaries.join(", ")
                );
                println!();
            } else {
                println!(
                    "  {} {}",
                    colors::package_name(key),
                    colors::version(&package.version)
                );
            }
        }

        Ok(())
    }

    async fn update_package(&self, owner: &str, repo: &str, global: bool) -> Result<()> {
        let package_key = format!("{}/{}", owner, repo);
        let registry = Registry::load(global)?;

        if let Some(package) = registry.get(&package_key) {
            colors::print_progress("Checking", &colors::package_name(&package_key));

            let current_version = &package.version;
            let release = self.github.fetch_release(owner, repo, None).await?;

            if release.tag_name == *current_version {
                colors::print_info(
                    "Up-to-date",
                    &format!(
                        "{} {}",
                        colors::package_name(&package_key),
                        colors::version(&format!("({})", current_version))
                    ),
                );
                return Ok(());
            }

            colors::print_progress(
                "Updating",
                &format!(
                    "{} {} → {}",
                    colors::package_name(&package_key),
                    colors::dim(current_version),
                    colors::version(&release.tag_name)
                ),
            );
            self.install_package(owner, repo, None, global, true)
                .await?;
        } else {
            colors::print_error(
                "Error",
                &format!("Package not found: {}", colors::package_name(&package_key)),
            );
            anyhow::bail!("Package not found: {}", package_key);
        }

        Ok(())
    }

    async fn update_all_packages(&self, global: bool) -> Result<()> {
        let registry = Registry::load(global)?;

        if registry.is_empty() {
            colors::print_info("Info", "No packages to update");
            return Ok(());
        }

        colors::print_progress(
            "Checking",
            &format!("{} packages for updates", registry.len()),
        );
        println!();

        for key in registry.keys() {
            let (owner, repo) = utils::parse_package(key)?;
            if let Err(e) = self.update_package(&owner, &repo, global).await {
                colors::print_error("Failed", &format!("{}: {}", colors::package_name(key), e));
            }
            println!();
        }

        Ok(())
    }

    async fn show_info(&self, owner: &str, repo: &str) -> Result<()> {
        let pb = utils::create_spinner("Fetching repository information");
        let repository = self.github.fetch_repository(owner, repo).await?;
        pb.finish_and_clear();

        println!("{}", colors::header(&format!("{}/{}", owner, repo)));
        println!();

        if let Some(desc) = repository.description {
            println!("{} {}", colors::dim("Description:"), desc);
        }

        println!(
            "{} {}",
            colors::dim("Stars:"),
            colors::warning(&repository.stargazers_count.to_string())
        );

        if let Some(lang) = repository.language {
            println!("{} {}", colors::dim("Language:"), colors::info(&lang));
        }

        println!(
            "{} {}",
            colors::dim("URL:"),
            colors::path(&repository.html_url)
        );

        let release_result = self.github.fetch_release(owner, repo, None).await;

        if let Ok(release) = release_result {
            println!();
            println!(
                "{} {} {}",
                colors::dim("Latest:"),
                release.name,
                colors::version(&format!("({})", release.tag_name))
            );
            println!("{} {}", colors::dim("Published:"), release.published_at);

            if !release.assets.is_empty() {
                println!("{} {}", colors::dim("Assets:"), release.assets.len());
            }
        }

        Ok(())
    }

    async fn search(&self, query: &str, limit: usize, by_user: bool) -> Result<()> {
        let pb = utils::create_spinner("Searching...");

        if by_user {
            let result = self.github.search_users(query, limit).await?;
            pb.finish_and_clear();

            if result.items.is_empty() {
                colors::print_info("Result", "No users found");
                return Ok(());
            }

            println!(
                "{} {}",
                colors::header("Users"),
                colors::dim(&format!("({})", result.items.len()))
            );
            println!();

            for user in result.items {
                println!("  {}", colors::package_name(&user.login));
                println!(
                    "    {} {}",
                    colors::dim("URL:"),
                    colors::path(&user.html_url)
                );
                println!();
            }
        } else {
            let search_query = if query.contains('/') {
                query.to_string()
            } else {
                format!("user:{}", query)
            };

            let result = self
                .github
                .search_repositories(&search_query, limit)
                .await?;
            pb.finish_and_clear();

            if result.items.is_empty() {
                colors::print_info("Result", "No repositories found");
                return Ok(());
            }

            println!(
                "{} {}",
                colors::header("Repositories"),
                colors::dim(&format!("({})", result.items.len()))
            );
            println!();

            for repo in result.items {
                println!("  {}", colors::package_name(&repo.full_name));
                if let Some(desc) = repo.description {
                    println!("    {}", colors::dim(&desc));
                }
                println!(
                    "    {} {}",
                    colors::dim("Stars:"),
                    colors::warning(&repo.stargazers_count.to_string())
                );
                if let Some(lang) = repo.language {
                    println!("    {} {}", colors::dim("Language:"), colors::info(&lang));
                }
                println!();
            }
        }

        Ok(())
    }

    async fn clean_packages(&self, global: bool) -> Result<()> {
        let install_dir = utils::get_install_dir(global)?;

        if !install_dir.exists() {
            colors::print_info("Info", "Nothing to clean");
            return Ok(());
        }

        let registry = Registry::load(global)?;
        let mut removed = 0;
        let mut failed = 0;

        for entry in fs::read_dir(&install_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name().unwrap().to_string_lossy();
                let mut is_tracked = false;

                for (_, package) in registry.iter() {
                    if package.binaries.iter().any(|b| b == &filename.to_string()) {
                        is_tracked = true;
                        break;
                    }
                }

                if !is_tracked {
                    match fs::remove_file(&path) {
                        Ok(_) => {
                            colors::print_success("Removed", &filename);
                            removed += 1;
                        }
                        Err(e) => {
                            colors::print_error("Failed", &format!("{}: {}", filename, e));
                            failed += 1;
                        }
                    }
                }
            }
        }

        println!();
        colors::print_success("Cleaned", &format!("{} files", removed));

        if failed > 0 {
            colors::print_warning("Failed", &format!("{} files", failed));
        }

        Ok(())
    }

    async fn dump_repositories(
        &self,
        username: &str,
        language: Option<String>,
        min_stars: u32,
        max_size: Option<u32>,
        only_original: bool,
        regex_pattern: Option<String>,
        parallel: usize,
        output_dir: PathBuf,
    ) -> Result<()> {
        let start_time = Instant::now();

        colors::print_progress(
            "Scanning",
            &format!("repositories for {}", colors::package_name(username)),
        );

        if let Some(lang) = &language {
            println!("  {} {}", colors::dim("Language:"), colors::info(lang));
        }

        if min_stars > 0 {
            println!("  {} {}", colors::dim("Min stars:"), min_stars);
        }

        if let Some(size) = max_size {
            println!("  {} {}MB", colors::dim("Max size:"), size);
        }

        if only_original {
            println!("  {} yes", colors::dim("Original only:"));
        }

        if let Some(pattern) = &regex_pattern {
            println!("  {} {}", colors::dim("Regex:"), pattern);
        }

        println!("  {} {}", colors::dim("Parallel:"), parallel);
        println!();

        let regex_filter = if let Some(pattern) = regex_pattern {
            Some(Regex::new(&pattern)?)
        } else {
            None
        };

        let repos = self.github.fetch_user_repos(username).await?;
        let filtered_repos = filter_repositories(
            repos,
            language.as_ref(),
            min_stars,
            max_size,
            only_original,
            &regex_filter,
        );

        colors::print_info(
            "Found",
            &format!("{} repositories matching criteria", filtered_repos.len()),
        );

        if filtered_repos.is_empty() {
            colors::print_info("Info", "No repositories to download");
            return Ok(());
        }

        println!();

        fs::create_dir_all(&output_dir)?;

        let progress = Arc::new(ProgressTracker::new(filtered_repos.len()));
        let semaphore = Arc::new(Semaphore::new(parallel));
        let client = Arc::new(self.github.client().clone());
        let mut tasks = Vec::new();

        for repo in filtered_repos {
            let client = client.clone();
            let progress = progress.clone();
            let semaphore = semaphore.clone();
            let output_dir = output_dir.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = download_repository_source(client.as_ref(), &repo, &output_dir).await;
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
        println!("{}", colors::header("Results"));
        println!(
            "  {} {}",
            colors::dim("Downloaded:"),
            colors::success(&stats.downloaded.to_string())
        );
        println!(
            "  {} {}",
            colors::dim("Failed:"),
            if stats.failed > 0 {
                colors::error(&stats.failed.to_string())
            } else {
                stats.failed.to_string()
            }
        );
        println!(
            "  {} {:.2} MB",
            colors::dim("Total size:"),
            stats.total_size as f64 / 1024.0 / 1024.0
        );
        println!("  {} {:.2}s", colors::dim("Time:"), elapsed.as_secs_f64());

        if stats.downloaded > 0 {
            println!(
                "  {} {:.1} MB/s",
                colors::dim("Speed:"),
                (stats.total_size as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64()
            );
        }

        Ok(())
    }
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
                println!(
                    "{} {} {}",
                    colors::dim(&format!("[{}/{}]", current, self.total)),
                    colors::package_name(&name),
                    colors::dim(&format!("({:.2} MB)", size as f64 / 1024.0 / 1024.0))
                );
            }
            Err(e) => {
                let mut failed = self.failed.lock().await;
                *failed += 1;
                println!(
                    "{} {} {}",
                    colors::dim(&format!("[{}/{}]", current, self.total)),
                    colors::package_name(&name),
                    colors::error(&format!("FAILED: {}", e))
                );
            }
        }
    }

    async fn get_stats(&self) -> DumpStats {
        DumpStats {
            downloaded: *self.downloaded.lock().await,
            failed: *self.failed.lock().await,
            total_size: *self.total_size.lock().await,
        }
    }
}

fn filter_repositories(
    repos: Vec<Repository>,
    language_filter: Option<&String>,
    min_stars: u32,
    max_size: Option<u32>,
    only_original: bool,
    regex_filter: &Option<Regex>,
) -> Vec<Repository> {
    repos
        .into_iter()
        .filter(|repo| {
            if only_original && repo.is_fork {
                return false;
            }

            if repo.stargazers_count < min_stars {
                return false;
            }

            if let Some(max_size_mb) = max_size {
                if repo.size > max_size_mb * 1024 {
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

async fn download_repository_source(
    client: &Client,
    repo: &Repository,
    output_dir: &std::path::Path,
) -> Result<u64, String> {
    let repo_path = output_dir.join(&repo.name);

    if repo_path.exists() {
        return utils::get_directory_size(&repo_path).map_err(|e| e.to_string());
    }

    let branch = &repo.default_branch;
    let zip_url = format!("{}/archive/refs/heads/{}.zip", repo.html_url, branch);

    match download_and_extract_zip(client, &zip_url, &repo_path).await {
        Ok(size) => Ok(size),
        Err(e) => {
            let fallback_branches = ["main", "master", "develop", "trunk"];
            for fallback in &fallback_branches {
                if *fallback == branch {
                    continue;
                }

                let fallback_url = format!("{}/archive/refs/heads/{}.zip", repo.html_url, fallback);

                if let Ok(size) = download_and_extract_zip(client, &fallback_url, &repo_path).await
                {
                    return Ok(size);
                }
            }

            Err(format!("Failed to download: {}", e))
        }
    }
}

async fn download_and_extract_zip(
    client: &Client,
    url: &str,
    repo_path: &std::path::Path,
) -> Result<u64, String> {
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match client.get(url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(format!("HTTP {}", response.status()));
                }

                let bytes = response.bytes().await.map_err(|e| e.to_string())?;
                let zip_file = format!("{}.zip", repo_path.to_string_lossy());

                fs::write(&zip_file, &bytes).map_err(|e| e.to_string())?;

                let result = archive::extract_repository_zip(&zip_file, repo_path);
                fs::remove_file(&zip_file).ok();

                match result {
                    Ok(_) => {
                        return utils::get_directory_size(repo_path).map_err(|e| e.to_string());
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES - 1 {
                    tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                }
            }
        }
    }

    Err(last_error.unwrap().to_string())
}

use anyhow::Result;
use reqwest::Client;
use std::fs;
use std::path::Path;

use crate::archive;
use crate::types::{Asset, AssetScore};
use crate::utils;

const MAX_DOWNLOAD_SIZE: u64 = 2 * 1024 * 1024 * 1024;

pub fn find_best_asset(
    assets: &[Asset],
    os: &str,
    arch: &str,
    repo_name: &str,
) -> Option<AssetScore> {
    if assets.is_empty() {
        return None;
    }

    let os_patterns = match os {
        "linux" => vec!["linux", "unknown-linux", "gnu"],
        "macos" => vec!["darwin", "macos", "apple", "osx"],
        "windows" => vec!["windows", "win", "pc-windows", "msvc"],
        _ => vec![],
    };

    let arch_patterns = match arch {
        "x86_64" => vec!["x86_64", "amd64", "x64"],
        "aarch64" => vec!["aarch64", "arm64"],
        "x86" => vec!["i686", "x86", "i386"],
        _ => vec![],
    };

    let mut scored_assets: Vec<AssetScore> = assets
        .iter()
        .filter(|asset| asset.size <= MAX_DOWNLOAD_SIZE)
        .map(|asset| {
            let name = asset.name.to_lowercase();
            let mut score = 0;
            let mut reasons = Vec::new();

            for pattern in &os_patterns {
                if name.contains(pattern) {
                    score += 100;
                    reasons.push(format!("OS match: {}", pattern));
                    break;
                }
            }

            for pattern in &arch_patterns {
                if name.contains(pattern) {
                    score += 50;
                    reasons.push(format!("Arch match: {}", pattern));
                    break;
                }
            }

            if name.contains(&repo_name.to_lowercase()) {
                score += 30;
                reasons.push("Repo name match".to_string());
            }

            if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
                score += 15;
                reasons.push("Archive: tar.gz".to_string());
            } else if name.ends_with(".zip") {
                score += 12;
                reasons.push("Archive: zip".to_string());
            } else if name.ends_with(".exe") && os == "windows" {
                score += 25;
                reasons.push("Windows executable".to_string());
            } else if name.ends_with(".tar.xz") {
                score += 14;
                reasons.push("Archive: tar.xz".to_string());
            } else if name.ends_with(".tar.bz2") {
                score += 13;
                reasons.push("Archive: tar.bz2".to_string());
            }

            if name.contains("musl") && os == "linux" {
                score += 5;
                reasons.push("Musl static binary".to_string());
            }

            if name.contains("src") || name.contains("source") {
                score -= 100;
                reasons.push("Source code (excluded)".to_string());
            }

            if name.contains("checksum") || name.contains(".sha") || name.contains(".md5") {
                score -= 100;
                reasons.push("Checksum file (excluded)".to_string());
            }

            if name.contains("unsigned") {
                score -= 10;
                reasons.push("Unsigned binary".to_string());
            }

            if name.contains("debug") || name.contains(".pdb") {
                score -= 50;
                reasons.push("Debug symbols (excluded)".to_string());
            }

            AssetScore {
                asset: asset.clone(),
                score,
                reason: reasons.join(", "),
            }
        })
        .collect();

    scored_assets.sort_by(|a, b| b.score.cmp(&a.score));

    scored_assets.into_iter().find(|s| s.score > 0).or_else(|| {
        assets
            .iter()
            .find(|a| {
                let name = a.name.to_lowercase();
                a.size <= MAX_DOWNLOAD_SIZE
                    && (name.ends_with(".tar.gz")
                        || name.ends_with(".zip")
                        || name.ends_with(".exe")
                        || name.ends_with(".tgz"))
                    && !name.contains("src")
                    && !name.contains("source")
            })
            .map(|a| AssetScore {
                asset: a.clone(),
                score: 1,
                reason: "Fallback: generic archive".to_string(),
            })
    })
}

pub async fn download_and_install_asset(
    client: &Client,
    asset: &Asset,
    install_dir: &Path,
    repo_name: &str,
) -> Result<Vec<String>> {
    if asset.size > MAX_DOWNLOAD_SIZE {
        anyhow::bail!(
            "Asset too large: {} bytes (max: {} bytes)",
            asset.size,
            MAX_DOWNLOAD_SIZE
        );
    }

    let pb = utils::create_progress_bar(asset.size, format!("Downloading {}", asset.name));

    let mut response = client.get(&asset.browser_download_url).send().await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download asset: HTTP {}", response.status());
    }

    let mut downloaded: u64 = 0;
    let mut buffer = Vec::with_capacity(asset.size.min(100 * 1024 * 1024) as usize);

    while let Some(chunk) = response.chunk().await? {
        buffer.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;

        if downloaded > MAX_DOWNLOAD_SIZE {
            pb.finish_with_message("Download cancelled: size limit exceeded");
            anyhow::bail!("Download size exceeded limit");
        }

        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("Downloaded {}", asset.name));
    println!();

    let temp_file = install_dir.join(&asset.name);
    fs::write(&temp_file, &buffer)?;

    println!("Extracting binaries...");

    let binaries_result = if asset.name.ends_with(".tar.gz") || asset.name.ends_with(".tgz") {
        archive::extract_tar_gz(&temp_file, install_dir, repo_name)
    } else if asset.name.ends_with(".tar.xz") {
        anyhow::bail!("tar.xz format not yet supported")
    } else if asset.name.ends_with(".tar.bz2") {
        anyhow::bail!("tar.bz2 format not yet supported")
    } else if asset.name.ends_with(".zip") {
        archive::extract_zip_binaries(&temp_file, install_dir, repo_name)
    } else if asset.name.ends_with(".exe") {
        let binary_name = format!("{}.exe", repo_name);
        let dest = install_dir.join(&binary_name);
        fs::rename(&temp_file, &dest)?;
        Ok(vec![binary_name])
    } else {
        let binary_name = if cfg!(target_os = "windows") {
            format!("{}.exe", repo_name)
        } else {
            repo_name.to_string()
        };

        let dest = install_dir.join(&binary_name);
        fs::rename(&temp_file, &dest)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }

        Ok(vec![binary_name])
    };

    fs::remove_file(&temp_file).ok();

    binaries_result
}

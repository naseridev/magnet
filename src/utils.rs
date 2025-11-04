use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn parse_package(input: &str) -> Result<(String, String)> {
    let input = input.trim();

    if input.is_empty() {
        anyhow::bail!("Package name cannot be empty");
    }

    if input.len() > 200 {
        anyhow::bail!("Package name too long (max 200 characters)");
    }

    let parts: Vec<&str> = input.split('/').collect();

    if parts.len() != 2 {
        anyhow::bail!("Invalid package format. Use: owner/repo");
    }

    let owner = parts[0].trim();
    let repo = parts[1].trim();

    if owner.is_empty() || repo.is_empty() {
        anyhow::bail!("Owner and repo cannot be empty");
    }

    if !owner
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!("Invalid owner name: only alphanumeric, dash, and underscore allowed");
    }

    if !repo
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!("Invalid repo name: only alphanumeric, dash, underscore, and dot allowed");
    }

    Ok((owner.to_string(), repo.to_string()))
}

pub fn get_install_dir(global: bool) -> Result<PathBuf> {
    let dir = if global {
        if cfg!(target_os = "windows") {
            let program_files =
                env::var("PROGRAMFILES").unwrap_or_else(|_| "C:\\Program Files".to_string());
            PathBuf::from(program_files).join("magnet").join("bin")
        } else {
            PathBuf::from("/usr/local/bin")
        }
    } else {
        let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
        PathBuf::from(home).join(".magnet").join("bin")
    };

    fs::create_dir_all(&dir)?;

    Ok(dir)
}

pub fn get_registry_file(global: bool) -> Result<PathBuf> {
    let config_dir = if global {
        if cfg!(target_os = "windows") {
            let program_files =
                env::var("PROGRAMFILES").unwrap_or_else(|_| "C:\\Program Files".to_string());
            PathBuf::from(program_files).join("magnet")
        } else {
            PathBuf::from("/usr/local/magnet")
        }
    } else {
        let home = env::var("HOME").or_else(|_| env::var("USERPROFILE"))?;
        PathBuf::from(home).join(".magnet")
    };

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("registry.json"))
}

pub fn setup_path(bin_path: &Path) -> Result<()> {
    if !bin_path.exists() {
        anyhow::bail!("Binary path does not exist: {}", bin_path.display());
    }

    if cfg!(target_os = "windows") {
        setup_windows_path(bin_path)?;
    } else if cfg!(target_os = "macos") {
        setup_unix_path(bin_path, &[".zshrc", ".zprofile", ".bash_profile"])?;
    } else if cfg!(target_os = "linux") {
        setup_unix_path(bin_path, &[".bashrc", ".profile"])?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn setup_windows_path(bin_path: &Path) -> Result<()> {
    use std::process::Command;

    let path_str = bin_path.to_string_lossy().replace("/", "\\");

    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-Command",
            &format!(
                "$oldPath = [Environment]::GetEnvironmentVariable('Path', 'User'); \
                if ($oldPath -notlike '*{}*') {{ \
                    [Environment]::SetEnvironmentVariable('Path', $oldPath + ';{}', 'User') \
                }}",
                path_str, path_str
            ),
        ])
        .output();

    if let Ok(result) = output {
        if !result.status.success() {
            eprintln!("Warning: Failed to update PATH automatically");
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn setup_windows_path(_bin_path: &Path) -> Result<()> {
    Ok(())
}

fn setup_unix_path(bin_path: &Path, config_files: &[&str]) -> Result<()> {
    let home = env::var("HOME")?;
    let path_export = format!("\nexport PATH=\"{}:$PATH\"\n", bin_path.display());

    for config_file in config_files {
        let file_path = PathBuf::from(&home).join(config_file);

        if file_path.exists() {
            let content = fs::read_to_string(&file_path).unwrap_or_default();

            if !content.contains(&bin_path.to_string_lossy().to_string()) {
                let mut new_content = content;
                new_content.push_str(&path_export);

                if let Err(e) = fs::write(&file_path, new_content) {
                    eprintln!("Warning: Failed to update {}: {}", config_file, e);
                }
            }
        }
    }

    Ok(())
}

pub fn create_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap()
            .tick_strings(&["-", "\\", "|", "/"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn create_progress_bar(size: u64, msg: String) -> ProgressBar {
    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(msg);
    pb
}

pub fn get_directory_size(dir: &Path) -> Result<u64, std::io::Error> {
    let mut size = 0;

    if !dir.exists() {
        return Ok(0);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            size += metadata.len();
        } else if metadata.is_dir() {
            size += get_directory_size(&entry.path()).unwrap_or(0);
        }
    }
    Ok(size)
}

pub fn is_executable_file(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }

    if data.starts_with(b"\x7fELF") {
        return true;
    }

    if data.starts_with(b"MZ") {
        return true;
    }

    if data.starts_with(b"\xca\xfe\xba\xbe") || data.starts_with(b"\xce\xfa\xed\xfe") {
        return true;
    }

    false
}

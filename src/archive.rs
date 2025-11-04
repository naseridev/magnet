use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};
use tar::Archive;
use zip::ZipArchive;

use crate::utils::is_executable_file;

pub fn extract_tar_gz(
    archive_path: &Path,
    install_dir: &Path,
    repo_name: &str,
) -> Result<Vec<String>> {
    let tar_gz = fs::File::open(archive_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    let mut binaries = Vec::new();
    let mut temp_files: Vec<(String, Vec<u8>)> = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        let is_in_bin = path.starts_with("bin/")
            || path.to_string_lossy().contains("/bin/")
            || path.components().count() == 1;

        if is_in_bin {
            if let Some(filename) = path.file_name() {
                let mut buffer = Vec::new();
                copy(&mut entry, &mut buffer)?;

                if is_executable_file(&buffer) || buffer.len() > 1024 {
                    temp_files.push((filename.to_string_lossy().to_string(), buffer));
                }
            }
        }
    }

    if temp_files.is_empty() {
        let tar_gz = fs::File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();

            if !path.to_string_lossy().ends_with('/') {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if filename_str == repo_name || filename_str.starts_with(repo_name) {
                        let mut buffer = Vec::new();
                        copy(&mut entry, &mut buffer)?;

                        if is_executable_file(&buffer) || buffer.len() > 1024 {
                            temp_files.push((filename_str.to_string(), buffer));
                            break;
                        }
                    }
                }
            }
        }
    }

    for (filename, data) in temp_files {
        let dest = install_dir.join(&filename);
        fs::write(&dest, data)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }

        binaries.push(filename);
    }

    Ok(binaries)
}

pub fn extract_zip_binaries(
    zip_path: &Path,
    install_dir: &Path,
    repo_name: &str,
) -> Result<Vec<String>> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut binaries = Vec::new();
    let mut temp_files: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_path_buf(),
            None => continue,
        };

        if file.name().ends_with('/') {
            continue;
        }

        let outpath_str = outpath.to_string_lossy();
        let is_binary = outpath_str.contains("/bin/")
            || outpath_str.ends_with(".exe")
            || (outpath.components().count() == 2 && !file.name().ends_with('/'));

        if is_binary {
            if let Some(filename) = outpath.file_name() {
                let mut buffer = Vec::new();
                copy(&mut file, &mut buffer)?;

                if is_executable_file(&buffer)
                    || buffer.len() > 1024
                    || filename.to_string_lossy().ends_with(".exe")
                {
                    temp_files.push((filename.to_string_lossy().to_string(), buffer));
                }
            }
        }
    }

    if temp_files.is_empty() {
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_path_buf(),
                None => continue,
            };

            if file.name().ends_with('/') {
                continue;
            }

            if let Some(filename) = outpath.file_name() {
                let filename_str = filename.to_string_lossy();
                if filename_str == repo_name || filename_str.starts_with(repo_name) {
                    let mut buffer = Vec::new();
                    copy(&mut file, &mut buffer)?;

                    if is_executable_file(&buffer) || buffer.len() > 1024 {
                        temp_files.push((filename_str.to_string(), buffer));
                        break;
                    }
                }
            }
        }
    }

    for (filename, data) in temp_files {
        let dest = install_dir.join(&filename);
        fs::write(&dest, data)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }

        binaries.push(filename);
    }

    Ok(binaries)
}

pub fn extract_repository_zip(
    zip_path: &str,
    repo_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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

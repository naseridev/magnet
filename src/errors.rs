use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum MagnetError {
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Package already installed: {0}")]
    PackageAlreadyInstalled(String),

    #[error("Invalid package format: {0}")]
    InvalidPackageFormat(String),

    #[error("No compatible binary found for platform: {0}")]
    NoBinaryFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("GitHub API error: {0}")]
    GitHubApiError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

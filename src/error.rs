use thiserror::Error;

#[derive(Error, Debug)]
pub enum JcvmError {
    #[error("Version {0} not found")]
    VersionNotFound(String),

    #[error("Version {0} is already installed at {1}")]
    VersionAlreadyInstalled(String, String),

    #[error("Failed to download from {url}: {source}")]
    DownloadFailed {
        url: String,
        source: reqwest::Error,
    },

    #[error("Checksum verification failed for {file}")]
    ChecksumMismatch { file: String },

    #[error("Failed to extract archive: {0}")]
    ExtractionFailed(String),

    #[error("Unsupported platform: {os} {arch}")]
    UnsupportedPlatform { os: String, arch: String },

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Shell integration error: {0}")]
    ShellError(String),

    #[error("Invalid JDK structure: {0}")]
    InvalidJdkStructure(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("HTTP request error: {0}")]
    RequestError(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, JcvmError>;

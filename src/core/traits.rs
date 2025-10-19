use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Generic tool information that any versioned tool must provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Unique identifier for the tool (e.g., "java", "node", "python", "compass")
    pub id: String,

    /// Display name (e.g., "Java Development Kit", "Node.js")
    pub name: String,

    /// Short description
    pub description: String,

    /// Tool homepage URL
    pub homepage: Option<String>,

    /// Tool documentation URL
    pub docs_url: Option<String>,
}

/// Generic version representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToolVersion {
    /// Raw version string (e.g., "21.0.7", "18.17.0", "3.11.5")
    pub raw: String,

    /// Major version number
    pub major: u32,

    /// Minor version number (optional)
    pub minor: Option<u32>,

    /// Patch version number (optional)
    pub patch: Option<u32>,

    /// Additional version metadata (e.g., build number, pre-release tag)
    pub metadata: Option<String>,

    /// Whether this is an LTS version (if applicable)
    pub is_lts: bool,
}

impl ToolVersion {
    pub fn new(raw: String, major: u32, minor: Option<u32>, patch: Option<u32>) -> Self {
        Self {
            raw,
            major,
            minor,
            patch,
            metadata: None,
            is_lts: false,
        }
    }

    pub fn with_lts(mut self, is_lts: bool) -> Self {
        self.is_lts = is_lts;
        self
    }

    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl std::fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// Platform and architecture information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Mac,
    Linux,
    Windows,
}

impl Platform {
    pub fn as_str(&self) -> &str {
        match self {
            Platform::Mac => "mac",
            Platform::Linux => "linux",
            Platform::Windows => "windows",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    X64,
    Aarch64,
    X86,
    Arm,
}

impl Architecture {
    pub fn as_str(&self) -> &str {
        match self {
            Architecture::X64 => "x64",
            Architecture::Aarch64 => "aarch64",
            Architecture::X86 => "x86",
            Architecture::Arm => "arm",
        }
    }
}

impl std::fmt::Display for Architecture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Distribution information for a downloadable tool version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDistribution {
    /// Tool identifier
    pub tool_id: String,

    /// Version information
    pub version: ToolVersion,

    /// Target platform
    pub platform: Platform,

    /// Target architecture
    pub architecture: Architecture,

    /// Download URL
    pub download_url: String,

    /// Optional checksum for verification
    pub checksum: Option<String>,

    /// File size in bytes
    pub size: Option<u64>,

    /// Archive type (tar.gz, zip, dmg, exe, etc.)
    pub archive_type: ArchiveType,

    /// Additional metadata specific to the tool
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveType {
    TarGz,
    Zip,
    Dmg,
    Exe,
    Deb,
    Rpm,
    Pkg,
    Binary,
    Other(String),
}

/// Information about an installed tool version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledTool {
    /// Tool identifier
    pub tool_id: String,

    /// Version
    pub version: ToolVersion,

    /// Installation path
    pub path: PathBuf,

    /// When it was installed
    pub installed_at: chrono::DateTime<chrono::Utc>,

    /// Source/distribution name (e.g., "adoptium", "nodejs.org", "homebrew")
    pub source: String,

    /// Executable path (for tools with a primary binary)
    pub executable_path: Option<PathBuf>,
}

/// Core trait that every tool plugin must implement
#[async_trait]
pub trait ToolProvider: Send + Sync {
    /// Get tool information
    fn info(&self) -> ToolInfo;

    /// List available versions from remote sources
    async fn list_remote_versions(&self, lts_only: bool) -> Result<Vec<ToolVersion>>;

    /// Find distribution for a specific version
    async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution>;

    /// Parse version string into ToolVersion
    fn parse_version(&self, version_str: &str) -> Result<ToolVersion>;

    /// Validate if a directory contains a valid installation
    fn validate_installation(&self, path: &Path) -> Result<bool>;

    /// Get the executable path(s) for the installed tool
    #[allow(dead_code)]
    fn get_executable_paths(&self, install_path: &Path) -> Result<Vec<PathBuf>>;

    /// Get environment variables that should be set for this tool
    fn get_environment_vars(&self, install_path: &Path) -> Result<Vec<(String, String)>>;
}

/// Trait for installing tool distributions
#[async_trait]
pub trait ToolInstaller: Send + Sync {
    /// Install a tool distribution
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &Path,
    ) -> Result<InstalledTool>;

    /// Uninstall a tool version
    async fn uninstall(&self, installed: &InstalledTool) -> Result<()>;

    /// Verify installation integrity
    #[allow(dead_code)]
    async fn verify(&self, installed: &InstalledTool) -> Result<bool>;
}

/// Trait for detecting existing tool installations
#[async_trait]
pub trait ToolDetector: Send + Sync {
    /// Detect all installations of this tool on the system
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>>;

    /// Import an external installation into management
    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &Path,
    ) -> Result<InstalledTool>;
}

/// Information about a detected (but not yet managed) tool installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedInstallation {
    /// Tool identifier
    pub tool_id: String,

    /// Detected version
    pub version: ToolVersion,

    /// Installation path
    pub path: PathBuf,

    /// Source/origin of installation (e.g., "system", "homebrew", "manual")
    pub source: String,

    /// Executable path
    pub executable_path: Option<PathBuf>,
}

/// Complete tool plugin that combines all capabilities
pub trait ToolPlugin: ToolProvider + ToolInstaller + ToolDetector {
    /// Check if this plugin supports a given platform/architecture
    fn supports_platform(&self, platform: Platform, arch: Architecture) -> bool;
}

/// Plugin metadata for registration and discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin ID (matches tool_id)
    pub id: String,

    /// Human-friendly name shown in CLI output
    #[serde(default)]
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin author
    pub author: String,

    /// Supported platforms
    pub platforms: Vec<Platform>,

    /// Supported architectures
    pub architectures: Vec<Architecture>,

    /// Plugin category (language, database, tool, browser, etc.)
    pub category: PluginCategory,

    /// Whether this is a built-in or user-provided plugin
    pub builtin: bool,
}

impl PluginMetadata {
    /// Returns the display name for this plugin, falling back to the ID when unset
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.id
        } else {
            &self.name
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginCategory {
    Language,
    Runtime,
    Database,
    Tool,
    Browser,
    Editor,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_version_display() {
        let version = ToolVersion::new("21.0.7".to_string(), 21, Some(0), Some(7));
        assert_eq!(version.to_string(), "21.0.7");
    }
}

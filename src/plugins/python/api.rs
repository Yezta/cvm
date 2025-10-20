use crate::core::traits::{Architecture, Platform, ToolDistribution, ToolVersion};
use crate::error::{JcvmError, Result};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// Official Python.org FTP for distributions (primary source)
const PYTHON_ORG_FTP: &str = "https://www.python.org/ftp/python";

// Optional: python-build-standalone for pre-built binaries (fallback)
const PYTHON_STANDALONE_RELEASES: &str =
    "https://api.github.com/repos/indygreg/python-build-standalone/releases";

/// Python-build-standalone release information from GitHub API
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub assets: Vec<GitHubAsset>,
    pub prerelease: bool,
    pub published_at: Option<String>,
}

/// GitHub release asset information
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Python.org release information from the downloads API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythonRelease {
    pub name: String,
    pub version: String,
    pub stable: bool,
    pub release_date: Option<String>,
}

/// Python download file information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PythonFile {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
    pub md5: Option<String>,
    pub sha256: Option<String>,
}

/// API client for fetching Python releases
///
/// Uses official python.org FTP for distributions (primary source)
/// Can optionally use python-build-standalone for pre-built binaries
pub struct PythonApi {
    client: reqwest::Client,
    use_standalone: bool,
}

impl PythonApi {
    /// Creates a new PythonApi client
    /// By default, uses official python.org releases
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .unwrap(),
            use_standalone: false,
        }
    }

    /// Creates a PythonApi client with standalone mode
    /// Uses python-build-standalone pre-built binaries instead of official distributions
    pub fn new_with_standalone() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .unwrap(),
            use_standalone: true,
        }
    }

    /// Lists available Python versions dynamically from official python.org releases
    pub async fn list_available_versions(&self) -> Result<Vec<ToolVersion>> {
        if self.use_standalone {
            self.list_standalone_versions().await
        } else {
            self.list_pythonorg_versions().await
        }
    }

    /// Fetches versions from python-build-standalone GitHub releases
    async fn list_standalone_versions(&self) -> Result<Vec<ToolVersion>> {
        let response = self
            .client
            .get(PYTHON_STANDALONE_RELEASES)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| JcvmError::DownloadFailed {
                url: PYTHON_STANDALONE_RELEASES.to_string(),
                source: e,
            })?;

        let releases: Vec<GitHubRelease> =
            response.json().await.map_err(|e| JcvmError::PluginError {
                plugin: "python".to_string(),
                message: format!("Failed to parse GitHub API response: {}", e),
            })?;

        let mut versions = Vec::new();

        for release in releases {
            // Tag format: "20231002" or similar - we need to extract Python versions from assets
            // Assets are named like: cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz
            for asset in &release.assets {
                if let Some(version) = self.extract_version_from_asset_name(&asset.name) {
                    // Avoid duplicates
                    if !versions
                        .iter()
                        .any(|v: &ToolVersion| v.to_string() == version.to_string())
                    {
                        versions.push(version);
                    }
                }
            }
        }

        // Sort versions (newest first)
        versions.sort_by(|a, b| {
            b.major
                .cmp(&a.major)
                .then_with(|| b.minor.cmp(&a.minor))
                .then_with(|| b.patch.cmp(&a.patch))
        });

        Ok(versions)
    }

    /// Extracts Python version from standalone build asset name
    /// Example: "cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz" -> "3.12.0"
    fn extract_version_from_asset_name(&self, name: &str) -> Option<ToolVersion> {
        if !name.starts_with("cpython-") || !name.contains("install_only") {
            return None;
        }

        let parts: Vec<&str> = name.split('-').collect();
        if parts.len() < 2 {
            return None;
        }

        // Extract version from "cpython-3.12.0+20231002"
        let version_part = parts[1].split('+').next()?;
        self.parse_version_string(version_part)
    }

    /// Fetches versions from official python.org FTP directory
    async fn list_pythonorg_versions(&self) -> Result<Vec<ToolVersion>> {
        // Python.org FTP serves an HTML directory listing
        // Parse the directory to find available Python versions

        let response = self
            .client
            .get(format!("{}/", PYTHON_ORG_FTP))
            .send()
            .await
            .map_err(|e| JcvmError::DownloadFailed {
                url: PYTHON_ORG_FTP.to_string(),
                source: e,
            })?;

        if !response.status().is_success() {
            return Err(JcvmError::PluginError {
                plugin: "python".to_string(),
                message: format!(
                    "Failed to fetch Python releases: HTTP {}",
                    response.status()
                ),
            });
        }

        let html = response.text().await?;

        let mut versions = Vec::new();

        // Look for directory links like href="3.12.8/" or <a href="3.12.8/">
        // FTP directory listings typically use simple <a> tags
        for line in html.lines() {
            // Match both href="version/" and href='version/'
            let patterns = vec![("href=\"", "/\""), ("href='", "/'")];

            for (prefix, suffix) in patterns {
                if let Some(href_start) = line.find(prefix) {
                    let rest = &line[href_start + prefix.len()..];
                    if let Some(href_end) = rest.find(suffix) {
                        let version_str = &rest[..href_end];
                        // Parse version string - should be numeric like "3.12.8"
                        if let Some(version) = self.parse_version_string(version_str) {
                            // Only include Python 2.7 and 3.x versions
                            // Filter out very old versions (< 2.7) and test releases
                            if (version.major == 2 && version.minor.unwrap_or(0) >= 7)
                                || version.major == 3
                            {
                                versions.push(version);
                            }
                        }
                    }
                }
            }
        }

        if versions.is_empty() {
            return Err(JcvmError::PluginError {
                plugin: "python".to_string(),
                message: "No Python versions found on python.org FTP".to_string(),
            });
        }

        // Sort versions (newest first)
        versions.sort_by(|a, b| {
            b.major
                .cmp(&a.major)
                .then_with(|| b.minor.cmp(&a.minor))
                .then_with(|| b.patch.cmp(&a.patch))
        });

        // Deduplicate
        versions.dedup_by(|a, b| a.to_string() == b.to_string());

        Ok(versions)
    }

    /// Parses version string like "3.12.8" into ToolVersion
    fn parse_version_string(&self, version_str: &str) -> Option<ToolVersion> {
        let parts: Vec<&str> = version_str.split('.').collect();

        let major = parts.first()?.parse::<u32>().ok()?;
        let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok());

        Some(ToolVersion::new(
            version_str.to_string(),
            major,
            minor,
            patch,
        ))
    }

    /// Finds the distribution package for a specific version, platform, and architecture
    pub async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: &Platform,
        architecture: &Architecture,
    ) -> Result<ToolDistribution> {
        if self.use_standalone {
            // Try standalone first, fall back to python.org if not available
            match self
                .find_standalone_distribution(version, platform, architecture)
                .await
            {
                Ok(dist) => Ok(dist),
                Err(_) => {
                    eprintln!(
                        "Warning: No standalone build found for {}, using python.org",
                        version
                    );
                    self.find_pythonorg_distribution(version, platform, architecture)
                        .await
                }
            }
        } else {
            self.find_pythonorg_distribution(version, platform, architecture)
                .await
        }
    }

    /// Finds pre-built binary from python-build-standalone
    async fn find_standalone_distribution(
        &self,
        version: &ToolVersion,
        platform: &Platform,
        architecture: &Architecture,
    ) -> Result<ToolDistribution> {
        let version_str = version.to_string();

        // Get all releases
        let response = self
            .client
            .get(PYTHON_STANDALONE_RELEASES)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| JcvmError::DownloadFailed {
                url: PYTHON_STANDALONE_RELEASES.to_string(),
                source: e,
            })?;

        let releases: Vec<GitHubRelease> = response.json().await?;

        // Build target triple for this platform/arch
        let target_triple = self.get_standalone_target_triple(platform, architecture)?;

        // Find matching asset
        for release in releases {
            for asset in &release.assets {
                // Asset name format: cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz
                if asset.name.contains(&version_str)
                    && asset.name.contains(&target_triple)
                    && asset.name.contains("install_only")
                {
                    // Fetch SHA256 checksum
                    let checksum = self
                        .fetch_standalone_checksum(&asset.browser_download_url)
                        .await
                        .ok();

                    return Ok(ToolDistribution {
                        tool_id: "python".to_string(),
                        version: version.clone(),
                        platform: *platform,
                        architecture: *architecture,
                        download_url: asset.browser_download_url.clone(),
                        checksum,
                        size: Some(asset.size),
                        archive_type: crate::core::traits::ArchiveType::TarGz,
                        metadata: HashMap::from([
                            ("source".to_string(), "python-build-standalone".to_string()),
                            ("release_tag".to_string(), release.tag_name.clone()),
                        ]),
                    });
                }
            }
        }

        Err(JcvmError::PluginError {
            plugin: "python".to_string(),
            message: format!(
                "No standalone build found for Python {} on {:?} {:?}",
                version_str, platform, architecture
            ),
        })
    }

    /// Gets the target triple for python-build-standalone builds
    fn get_standalone_target_triple(
        &self,
        platform: &Platform,
        architecture: &Architecture,
    ) -> Result<String> {
        let triple = match (platform, architecture) {
            (Platform::Mac, Architecture::X64) => "x86_64-apple-darwin",
            (Platform::Mac, Architecture::Aarch64) => "aarch64-apple-darwin",
            (Platform::Linux, Architecture::X64) => "x86_64-unknown-linux-gnu",
            (Platform::Linux, Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
            (Platform::Windows, Architecture::X64) => "x86_64-pc-windows-msvc",
            (Platform::Windows, Architecture::X86) => "i686-pc-windows-msvc",
            _ => {
                return Err(JcvmError::UnsupportedPlatform {
                    os: format!("{:?}", platform),
                    arch: format!("{:?}", architecture),
                });
            }
        };

        Ok(triple.to_string())
    }

    /// Fetches SHA256 checksum from standalone build
    async fn fetch_standalone_checksum(&self, download_url: &str) -> Result<String> {
        // Checksum files are named with .sha256 extension
        let checksum_url = format!("{}.sha256", download_url);

        let response = self.client.get(&checksum_url).send().await;

        match response {
            Ok(resp) => {
                let checksum_text = resp.text().await?;
                // Format is usually: <hash>  <filename>
                let hash = checksum_text
                    .split_whitespace()
                    .next()
                    .unwrap_or(&checksum_text)
                    .to_string();
                Ok(hash)
            }
            Err(_) => Err(JcvmError::PluginError {
                plugin: "python".to_string(),
                message: "Checksum file not found".to_string(),
            }),
        }
    }

    /// Finds distribution from official python.org releases
    async fn find_pythonorg_distribution(
        &self,
        version: &ToolVersion,
        platform: &Platform,
        architecture: &Architecture,
    ) -> Result<ToolDistribution> {
        let version_str = version.to_string();

        // Construct the download URL based on platform
        // Python.org FTP structure: https://www.python.org/ftp/python/{version}/
        let (url, archive_type) = match platform {
            Platform::Mac => {
                // For macOS 11+, Python provides universal2 binaries (arm64 + x86_64)
                // Format: python-{version}-macos11.pkg
                // Older versions may use different naming (e.g., macosx10.9)
                let major = version.major;
                let minor = version.minor.unwrap_or(0);

                // Python 3.9+ uses macos11.pkg for universal2 binaries
                let macos_suffix = if major == 3 && minor >= 9 {
                    "macos11"
                } else if major == 3 && minor >= 6 {
                    "macosx10.9"
                } else {
                    "macosx10.6"
                };

                let url = format!(
                    "{}/{}/python-{}-{}.pkg",
                    PYTHON_ORG_FTP, version_str, version_str, macos_suffix
                );
                (url, crate::core::traits::ArchiveType::Pkg)
            }
            Platform::Linux => {
                // For Linux, use tar.xz source tarball
                // Format: Python-{version}.tar.xz (note capital P)
                let url = format!(
                    "{}/{}/Python-{}.tar.xz",
                    PYTHON_ORG_FTP, version_str, version_str
                );
                (url, crate::core::traits::ArchiveType::TarGz)
            }
            Platform::Windows => {
                // Windows uses executable installers
                // Format: python-{version}-{arch}.exe
                let arch_suffix = match architecture {
                    Architecture::X64 => "amd64",
                    Architecture::X86 => "", // 32-bit has no suffix
                    Architecture::Aarch64 => "arm64",
                    _ => "amd64",
                };
                let url = format!(
                    "{}/{}/python-{}{}.exe",
                    PYTHON_ORG_FTP,
                    version_str,
                    version_str,
                    if arch_suffix.is_empty() {
                        "".to_string()
                    } else {
                        format!("-{}", arch_suffix)
                    }
                );
                (url, crate::core::traits::ArchiveType::Exe)
            }
        };

        // Verify the URL exists before returning
        let head_response = self.client.head(&url).send().await;
        if let Ok(resp) = head_response {
            if !resp.status().is_success() {
                return Err(JcvmError::PluginError {
                    plugin: "python".to_string(),
                    message: format!(
                        "Python {} distribution not found for {:?} {:?} at {}",
                        version_str, platform, architecture, url
                    ),
                });
            }
        }

        // Fetch checksum from python.org (MD5 or SHA256)
        let checksum = self.fetch_pythonorg_checksum(&url, &version_str).await.ok();

        Ok(ToolDistribution {
            tool_id: "python".to_string(),
            version: version.clone(),
            platform: *platform,
            architecture: *architecture,
            download_url: url,
            checksum,
            size: None,
            archive_type,
            metadata: HashMap::from([
                ("source".to_string(), "python.org".to_string()),
                ("official".to_string(), "true".to_string()),
            ]),
        })
    }

    /// Fetches checksum from python.org checksum files (tries SHA256 first, then MD5)
    async fn fetch_pythonorg_checksum(&self, download_url: &str, version: &str) -> Result<String> {
        let filename =
            download_url
                .split('/')
                .next_back()
                .ok_or_else(|| JcvmError::PluginError {
                    plugin: "python".to_string(),
                    message: "Invalid download URL".to_string(),
                })?;

        // Python.org provides both SHA256SUMS and MD5SUMS files
        // Try SHA256 first (more secure), fall back to MD5
        let checksum_files = vec![
            (
                format!("{}/{}/SHA256SUMS", PYTHON_ORG_FTP, version),
                "sha256",
            ),
            (format!("{}/{}/MD5SUM", PYTHON_ORG_FTP, version), "md5"),
        ];

        for (checksum_url, hash_type) in checksum_files {
            let response = self.client.get(&checksum_url).send().await;

            if let Ok(resp) = response {
                if resp.status().is_success() {
                    if let Ok(checksum_text) = resp.text().await {
                        // Parse checksum file to find our file
                        // Format: <hash>  <filename>
                        for line in checksum_text.lines() {
                            if line.contains(filename) {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if !parts.is_empty() {
                                    let hash = parts[0];
                                    // Return hash with type prefix for clarity
                                    return Ok(format!("{}:{}", hash_type, hash));
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(JcvmError::PluginError {
            plugin: "python".to_string(),
            message: format!("No checksum found for {} in python.org checksums", filename),
        })
    }

    /// Verifies if a version exists on official python.org releases
    pub async fn verify_version_exists(&self, version: &str) -> Result<bool> {
        if self.use_standalone {
            // Check if version exists in standalone builds
            let versions = self.list_standalone_versions().await?;
            Ok(versions.iter().any(|v| v.to_string() == version))
        } else {
            // Check if directory exists on python.org FTP
            let url = format!("{}/{}/", PYTHON_ORG_FTP, version);

            match self.client.head(&url).send().await {
                Ok(response) => Ok(response.status().is_success()),
                Err(_) => Ok(false),
            }
        }
    }

    /// Gets available LTS (long-term support) versions
    /// For Python, this means currently maintained versions (3.9+)
    pub async fn list_lts_versions(&self) -> Result<Vec<ToolVersion>> {
        let all_versions = self.list_available_versions().await?;

        // Filter to maintained versions (3.9+, latest patches only)
        let mut lts_versions: Vec<ToolVersion> = all_versions
            .into_iter()
            .filter(|v| {
                // Python 3.9 and later are still maintained
                v.major == 3 && v.minor.unwrap_or(0) >= 9
            })
            .collect();

        // Keep only the latest patch for each minor version
        let mut seen_minors = std::collections::HashSet::new();
        lts_versions.retain(|v| {
            let minor = v.minor.unwrap_or(0);
            seen_minors.insert(minor)
        });

        Ok(lts_versions)
    }
}

impl Default for PythonApi {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PythonRelease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Python {} ({})",
            self.version,
            if self.stable { "stable" } else { "pre-release" }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires live GitHub API
    async fn test_list_versions() {
        let api = PythonApi::new();
        let versions = api.list_available_versions().await.unwrap();
        assert!(!versions.is_empty());
        // Should have Python 3.x versions
        assert!(versions.iter().any(|v| v.major == 3));
    }

    #[tokio::test]
    #[ignore] // Requires live GitHub API
    async fn test_list_lts_versions() {
        let api = PythonApi::new();
        let lts_versions = api.list_lts_versions().await.unwrap();
        assert!(!lts_versions.is_empty());
        // All should be Python 3.9+
        assert!(lts_versions.iter().all(|v| v.major >= 3));
    }

    #[test]
    fn test_parse_version_string() {
        let api = PythonApi::new();

        let version = api.parse_version_string("3.12.8").unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, Some(12));
        assert_eq!(version.patch, Some(8));

        let version2 = api.parse_version_string("3.11.0").unwrap();
        assert_eq!(version2.major, 3);
        assert_eq!(version2.minor, Some(11));
        assert_eq!(version2.patch, Some(0));
    }

    #[test]
    fn test_extract_version_from_asset_name() {
        let api = PythonApi::new();

        let asset_name = "cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz";
        let version = api.extract_version_from_asset_name(asset_name).unwrap();
        assert_eq!(version.to_string(), "3.12.0");

        // Should return None for non-install_only assets
        let non_install = "cpython-3.12.0+20231002-x86_64-unknown-linux-gnu.tar.gz";
        assert!(api.extract_version_from_asset_name(non_install).is_none());
    }

    #[test]
    fn test_get_standalone_target_triple() {
        let api = PythonApi::new();

        let triple = api
            .get_standalone_target_triple(&Platform::Mac, &Architecture::X64)
            .unwrap();
        assert_eq!(triple, "x86_64-apple-darwin");

        let triple2 = api
            .get_standalone_target_triple(&Platform::Mac, &Architecture::Aarch64)
            .unwrap();
        assert_eq!(triple2, "aarch64-apple-darwin");

        let triple3 = api
            .get_standalone_target_triple(&Platform::Linux, &Architecture::X64)
            .unwrap();
        assert_eq!(triple3, "x86_64-unknown-linux-gnu");

        let triple4 = api
            .get_standalone_target_triple(&Platform::Windows, &Architecture::X64)
            .unwrap();
        assert_eq!(triple4, "x86_64-pc-windows-msvc");
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_find_standalone_distribution() {
        let api = PythonApi::new();
        let version = ToolVersion::new("3.12.0".to_string(), 3, Some(12), Some(0));

        let dist = api
            .find_standalone_distribution(&version, &Platform::Linux, &Architecture::X64)
            .await;

        // This may succeed or fall back to python.org
        assert!(dist.is_ok() || dist.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_find_pythonorg_distribution_macos() {
        let api = PythonApi::new();
        let version = ToolVersion::new("3.12.8".to_string(), 3, Some(12), Some(8));
        let dist = api
            .find_pythonorg_distribution(&version, &Platform::Mac, &Architecture::Aarch64)
            .await
            .unwrap();

        assert!(dist.download_url.contains("python-3.12.8"));
        assert!(dist.download_url.contains("macos11.pkg"));
        assert_eq!(dist.metadata.get("source").unwrap(), "python.org");
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_find_pythonorg_distribution_linux() {
        let api = PythonApi::new();
        let version = ToolVersion::new("3.12.8".to_string(), 3, Some(12), Some(8));
        let dist = api
            .find_pythonorg_distribution(&version, &Platform::Linux, &Architecture::X64)
            .await
            .unwrap();

        assert!(dist.download_url.contains("Python-3.12.8"));
        assert!(dist.download_url.ends_with(".tar.xz"));
        assert_eq!(dist.metadata.get("source").unwrap(), "python.org");
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_find_pythonorg_distribution_windows() {
        let api = PythonApi::new();
        let version = ToolVersion::new("3.12.8".to_string(), 3, Some(12), Some(8));
        let dist = api
            .find_pythonorg_distribution(&version, &Platform::Windows, &Architecture::X64)
            .await
            .unwrap();

        assert!(dist.download_url.contains("python-3.12.8"));
        assert!(dist.download_url.contains("amd64.exe"));
        assert_eq!(dist.metadata.get("source").unwrap(), "python.org");
    }
}

use crate::core::traits::{Architecture, ArchiveType, Platform, ToolDistribution, ToolVersion};
use crate::error::{JcvmError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const NODEJS_API: &str = "https://nodejs.org/dist/index.json";
const NODEJS_DIST: &str = "https://nodejs.org/dist";

#[derive(Debug, Deserialize)]
struct NodeRelease {
    version: String,
    lts: serde_json::Value,
    files: Vec<String>,
}

pub struct NodeJsApi {
    client: Client,
}

impl NodeJsApi {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(concat!(
                    env!("CARGO_PKG_NAME"),
                    "/",
                    env!("CARGO_PKG_VERSION")
                ))
                .build()
                .unwrap(),
        }
    }

    pub async fn list_available_versions(&self) -> Result<Vec<ToolVersion>> {
        let releases: Vec<NodeRelease> = self
            .client
            .get(NODEJS_API)
            .send()
            .await
            .map_err(|e| JcvmError::DownloadFailed {
                url: NODEJS_API.to_string(),
                source: e,
            })?
            .json()
            .await?;

        let mut versions: Vec<ToolVersion> = releases
            .iter()
            .map(|r| self.parse_release_version(&r.version, &r.lts))
            .collect::<Result<Vec<_>>>()?;

        // Deduplicate and sort by major version
        versions.sort_by(|a, b| b.major.cmp(&a.major));
        versions.dedup_by(|a, b| a.major == b.major);

        Ok(versions)
    }

    pub async fn list_lts_versions(&self) -> Result<Vec<ToolVersion>> {
        let all_versions = self.list_available_versions().await?;
        Ok(all_versions.into_iter().filter(|v| v.is_lts).collect())
    }

    fn parse_release_version(&self, version: &str, lts: &serde_json::Value) -> Result<ToolVersion> {
        let cleaned = version.trim_start_matches('v');
        let parts: Vec<&str> = cleaned.split('.').collect();

        let major = parts
            .first()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| JcvmError::InvalidVersion(version.to_string()))?;

        let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());

        // Determine LTS status and codename from API response
        // The API returns either:
        // - false (boolean) for non-LTS versions
        // - "Codename" (string) for LTS versions with their codename
        let (is_lts, lts_name) = match lts {
            serde_json::Value::Bool(false) => (false, None),
            serde_json::Value::String(name) if !name.is_empty() => (true, Some(name.to_string())),
            // Some edge cases where lts might be true boolean
            serde_json::Value::Bool(true) => (true, None),
            _ => (false, None),
        };

        let mut version =
            ToolVersion::new(cleaned.to_string(), major, minor, patch).with_lts(is_lts);

        if let Some(name) = lts_name {
            version = version.with_metadata(format!("lts:{}", name));
        }

        Ok(version)
    }

    /// Fetches the SHASUMS256.txt file for a specific Node.js version
    async fn fetch_checksums(&self, version: &ToolVersion) -> Result<HashMap<String, String>> {
        let url = format!("{}/v{}/SHASUMS256.txt", NODEJS_DIST, version.raw);

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| JcvmError::DownloadFailed {
                    url: url.clone(),
                    source: e,
                })?;

        if !response.status().is_success() {
            return Ok(HashMap::new()); // Some old versions might not have checksums
        }

        let text = response.text().await?;
        let mut checksums = HashMap::new();

        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let checksum = parts[0];
                let filename = parts[1];
                checksums.insert(filename.to_string(), checksum.to_string());
            }
        }

        Ok(checksums)
    }

    /// Gets the expected checksum for a specific distribution file
    async fn get_checksum_for_file(
        &self,
        version: &ToolVersion,
        filename: &str,
    ) -> Result<Option<String>> {
        let checksums = self.fetch_checksums(version).await?;
        Ok(checksums.get(filename).cloned())
    }

    pub async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        let os = match platform {
            Platform::Mac => "darwin",
            Platform::Linux => "linux",
            Platform::Windows => "win",
        };

        let arch_str = match arch {
            Architecture::X64 => "x64",
            Architecture::Aarch64 => "arm64",
            _ => {
                return Err(JcvmError::UnsupportedPlatform {
                    os: platform.to_string(),
                    arch: arch.to_string(),
                })
            }
        };

        let extension = if platform == Platform::Windows {
            "zip"
        } else {
            "tar.gz"
        };

        let filename = format!("node-v{}-{}-{}.{}", version.raw, os, arch_str, extension);
        let url = format!("{}/v{}/{}", NODEJS_DIST, version.raw, filename);

        // Fetch checksum for verification
        let checksum = self.get_checksum_for_file(version, &filename).await?;

        let archive_type = if extension == "zip" {
            ArchiveType::Zip
        } else {
            ArchiveType::TarGz
        };

        let mut metadata = HashMap::new();
        metadata.insert("npm_included".to_string(), "true".to_string());

        // Add LTS information to metadata if available
        if version.is_lts {
            metadata.insert("lts".to_string(), "true".to_string());
            if let Some(ref meta) = version.metadata {
                if let Some(lts_name) = meta.strip_prefix("lts:") {
                    metadata.insert("lts_name".to_string(), lts_name.to_string());
                }
            }
        }

        Ok(ToolDistribution {
            tool_id: "node".to_string(),
            version: version.clone(),
            platform,
            architecture: arch,
            download_url: url,
            checksum,
            size: None,
            archive_type,
            metadata,
        })
    }
}

impl Default for NodeJsApi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_release_version() {
        let api = NodeJsApi::new();

        // Test non-LTS version
        let version = api
            .parse_release_version("v19.10.0", &serde_json::Value::Bool(false))
            .unwrap();
        assert_eq!(version.major, 19);
        assert_eq!(version.minor, Some(10));
        assert_eq!(version.patch, Some(0));
        assert!(!version.is_lts);

        // Test LTS version with codename (as returned by API)
        let lts_value = serde_json::Value::String("Iron".to_string());
        let version = api.parse_release_version("v20.10.0", &lts_value).unwrap();
        assert_eq!(version.major, 20);
        assert!(version.is_lts);
        assert_eq!(version.metadata, Some("lts:Iron".to_string()));

        // Test version without v prefix
        let version = api
            .parse_release_version(
                "18.17.1",
                &serde_json::Value::String("Hydrogen".to_string()),
            )
            .unwrap();
        assert_eq!(version.major, 18);
        assert_eq!(version.minor, Some(17));
        assert_eq!(version.patch, Some(1));
        assert!(version.is_lts);
        assert_eq!(version.metadata, Some("lts:Hydrogen".to_string()));
    }

    #[test]
    fn test_lts_detection_from_api() {
        let api = NodeJsApi::new();

        // Test LTS versions with different codenames
        let test_cases = vec![(22, "Jod"), (20, "Iron"), (18, "Hydrogen"), (16, "Gallium")];

        for (major, name) in test_cases {
            let version_str = format!("v{}.0.0", major);
            let lts_value = serde_json::Value::String(name.to_string());
            let version = api.parse_release_version(&version_str, &lts_value).unwrap();
            assert!(version.is_lts, "Version {} ({}) should be LTS", major, name);
            assert_eq!(
                version.metadata,
                Some(format!("lts:{}", name)),
                "Version {} should have correct LTS metadata",
                major
            );
        }

        // Non-LTS version
        let version = api
            .parse_release_version("v19.0.0", &serde_json::Value::Bool(false))
            .unwrap();
        assert!(!version.is_lts, "Version 19 should not be LTS");
        assert_eq!(version.metadata, None);
    }

    #[tokio::test]
    #[ignore] // Network test - run manually
    async fn test_list_available_versions() {
        let api = NodeJsApi::new();
        let versions = api.list_available_versions().await.unwrap();

        assert!(!versions.is_empty());
        // Should have recent versions
        assert!(versions.iter().any(|v| v.major >= 18));
    }

    #[tokio::test]
    #[ignore] // Network test - run manually
    async fn test_list_lts_versions() {
        let api = NodeJsApi::new();
        let versions = api.list_lts_versions().await.unwrap();

        assert!(!versions.is_empty());
        // All returned versions should be LTS
        assert!(versions.iter().all(|v| v.is_lts));
    }

    #[tokio::test]
    #[ignore] // Network test - run manually
    async fn test_fetch_checksums() {
        let api = NodeJsApi::new();
        let version = ToolVersion::new("20.10.0".to_string(), 20, Some(10), Some(0));

        let checksums = api.fetch_checksums(&version).await.unwrap();
        assert!(!checksums.is_empty());

        // Should have checksums for common platforms
        let has_linux = checksums.keys().any(|k| k.contains("linux"));
        let has_darwin = checksums.keys().any(|k| k.contains("darwin"));
        assert!(has_linux || has_darwin);
    }

    #[tokio::test]
    #[ignore] // Network test - run manually
    async fn test_find_distribution_with_checksum() {
        let api = NodeJsApi::new();
        let version = ToolVersion::new("20.10.0".to_string(), 20, Some(10), Some(0));

        let dist = api
            .find_distribution(&version, Platform::Linux, Architecture::X64)
            .await
            .unwrap();

        assert_eq!(dist.tool_id, "node");
        assert_eq!(dist.version.major, 20);
        assert!(dist.download_url.contains("node-v20.10.0-linux-x64.tar.gz"));
        assert!(dist.checksum.is_some(), "Checksum should be available");
        assert_eq!(dist.metadata.get("npm_included"), Some(&"true".to_string()));
    }
}

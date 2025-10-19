use crate::error::{JcvmError, Result};
use crate::models::{Architecture, JdkDistribution, Platform, Version};
use reqwest::Client;
use serde::Deserialize;

// Old Adoptium API - kept for potential future use
#[allow(dead_code)]
const ADOPTIUM_API_BASE: &str = "https://api.adoptium.net/v3";

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    available_releases: Vec<u32>,
    available_lts_releases: Vec<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    os: String,
    architecture: String,
    image_type: String,
    package: AdoptiumPackage,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: Option<String>,
    size: Option<u64>,
}

#[allow(dead_code)]
pub struct AdoptiumApi {
    client: Client,
}

#[allow(dead_code)]
impl AdoptiumApi {
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

    /// Get list of available JDK versions
    pub async fn list_available_versions(&self) -> Result<Vec<u32>> {
        let url = format!("{}/info/available_releases", ADOPTIUM_API_BASE);

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| JcvmError::DownloadFailed {
                    url: url.clone(),
                    source: e,
                })?;

        let release_info: AdoptiumRelease = response.json().await?;
        Ok(release_info.available_releases)
    }

    /// Get list of LTS versions
    pub async fn list_lts_versions(&self) -> Result<Vec<u32>> {
        let url = format!("{}/info/available_releases", ADOPTIUM_API_BASE);

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| JcvmError::DownloadFailed {
                    url: url.clone(),
                    source: e,
                })?;

        let release_info: AdoptiumRelease = response.json().await?;
        Ok(release_info.available_lts_releases)
    }

    /// Find download information for a specific version
    pub async fn find_distribution(
        &self,
        version: &Version,
        platform: Platform,
        arch: Architecture,
    ) -> Result<JdkDistribution> {
        let url = format!(
            "{}/assets/latest/{}/hotspot",
            ADOPTIUM_API_BASE, version.major
        );

        let response =
            self.client
                .get(&url)
                .send()
                .await
                .map_err(|e| JcvmError::DownloadFailed {
                    url: url.clone(),
                    source: e,
                })?;

        let assets: Vec<AdoptiumAsset> = response.json().await?;

        let asset = assets
            .into_iter()
            .find(|a| {
                a.binary.os == platform.as_str()
                    && a.binary.architecture == arch.as_str()
                    && a.binary.image_type == "jdk"
            })
            .ok_or_else(|| JcvmError::VersionNotFound(version.to_string()))?;

        Ok(JdkDistribution {
            version: version.clone(),
            platform,
            architecture: arch,
            download_url: asset.binary.package.link,
            checksum: asset.binary.package.checksum,
            size: asset.binary.package.size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_available_versions() {
        let api = AdoptiumApi::new();
        let versions = api.list_available_versions().await.unwrap();
        assert!(!versions.is_empty());
    }

    #[tokio::test]
    async fn test_list_lts_versions() {
        let api = AdoptiumApi::new();
        let lts_versions = api.list_lts_versions().await.unwrap();
        assert!(lts_versions.contains(&21));
        assert!(lts_versions.contains(&17));
    }
}

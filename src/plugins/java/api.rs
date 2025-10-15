use crate::core::traits::{Architecture, ArchiveType, Platform, ToolDistribution, ToolVersion};
use crate::error::{JcvmError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const ADOPTIUM_API_BASE: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Deserialize)]
struct AdoptiumRelease {
    available_releases: Vec<u32>,
    available_lts_releases: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct AdoptiumAsset {
    binary: AdoptiumBinary,
}

#[derive(Debug, Deserialize)]
struct AdoptiumBinary {
    os: String,
    architecture: String,
    image_type: String,
    package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: Option<String>,
    size: Option<u64>,
}

pub struct AdoptiumApi {
    client: Client,
}

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
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
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

        // Convert our types to Adoptium's naming convention
        let os_name = Self::platform_to_adoptium_os(platform);
        let arch_name = Self::arch_to_adoptium_arch(arch);

        // Find matching asset
        let asset = assets
            .into_iter()
            .find(|a| {
                a.binary.os == os_name
                    && a.binary.architecture == arch_name
                    && a.binary.image_type == "jdk"
            })
            .ok_or_else(|| JcvmError::UnsupportedPlatform {
                os: platform.to_string(),
                arch: arch.to_string(),
            })?;

        // Determine archive type from URL
        let archive_type = if asset.binary.package.link.ends_with(".tar.gz") {
            ArchiveType::TarGz
        } else if asset.binary.package.link.ends_with(".zip") {
            ArchiveType::Zip
        } else if asset.binary.package.link.ends_with(".dmg") {
            ArchiveType::Dmg
        } else if asset.binary.package.link.ends_with(".exe") {
            ArchiveType::Exe
        } else {
            ArchiveType::Other("unknown".to_string())
        };

        Ok(ToolDistribution {
            tool_id: "java".to_string(),
            version: version.clone(),
            platform,
            architecture: arch,
            download_url: asset.binary.package.link,
            checksum: asset.binary.package.checksum,
            size: asset.binary.package.size,
            archive_type,
            metadata: HashMap::new(),
        })
    }

    fn platform_to_adoptium_os(platform: Platform) -> String {
        match platform {
            Platform::Mac => "mac".to_string(),
            Platform::Linux => "linux".to_string(),
            Platform::Windows => "windows".to_string(),
        }
    }

    fn arch_to_adoptium_arch(arch: Architecture) -> String {
        match arch {
            Architecture::X64 => "x64".to_string(),
            Architecture::Aarch64 => "aarch64".to_string(),
            _ => "x64".to_string(),
        }
    }
}

impl Default for AdoptiumApi {
    fn default() -> Self {
        Self::new()
    }
}

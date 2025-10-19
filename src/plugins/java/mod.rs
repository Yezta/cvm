mod api;
mod detector;
mod installer;

use crate::core::traits::{
    Architecture, DetectedInstallation, InstalledTool, Platform, PluginCategory, PluginMetadata,
    ToolDistribution, ToolInfo, ToolPlugin, ToolProvider, ToolVersion,
};
use crate::error::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub use api::AdoptiumApi;
pub use detector::JavaDetector;
pub use installer::JavaInstaller;

/// Java/JDK plugin implementation
pub struct JavaPlugin {
    api: AdoptiumApi,
    installer: JavaInstaller,
    detector: JavaDetector,
}

impl JavaPlugin {
    pub fn new() -> Self {
        Self {
            api: AdoptiumApi::new(),
            installer: JavaInstaller::new(),
            detector: JavaDetector::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "java".to_string(),
            name: "Java Development Kit".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: "JCVM Contributors".to_string(),
            platforms: vec![Platform::Mac, Platform::Linux, Platform::Windows],
            architectures: vec![Architecture::X64, Architecture::Aarch64],
            category: PluginCategory::Language,
            builtin: true,
        }
    }
}

impl Default for JavaPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolProvider for JavaPlugin {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: "java".to_string(),
            name: "Java Development Kit".to_string(),
            description: "OpenJDK distributions from Adoptium (Eclipse Temurin)".to_string(),
            homepage: Some("https://adoptium.net".to_string()),
            docs_url: Some("https://adoptium.net/docs".to_string()),
        }
    }

    async fn list_remote_versions(&self, lts_only: bool) -> Result<Vec<ToolVersion>> {
        if lts_only {
            let versions = self.api.list_lts_versions().await?;
            Ok(versions
                .into_iter()
                .map(|major| ToolVersion::new(major.to_string(), major, None, None).with_lts(true))
                .collect())
        } else {
            let versions = self.api.list_available_versions().await?;
            Ok(versions
                .into_iter()
                .map(|major| {
                    let is_lts = matches!(major, 8 | 11 | 17 | 21);
                    ToolVersion::new(major.to_string(), major, None, None).with_lts(is_lts)
                })
                .collect())
        }
    }

    async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        self.api.find_distribution(version, platform, arch).await
    }

    fn parse_version(&self, version_str: &str) -> Result<ToolVersion> {
        // Parse versions like "21", "17.0.10", "11.0.22+7"
        let parts: Vec<&str> = version_str.split(&['.', '+'][..]).collect();

        let major = parts.first()
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| crate::error::JcvmError::InvalidVersion(version_str.to_string()))?;

        let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok());
        let build = parts.get(3).map(|p| p.to_string());

        let is_lts = matches!(major, 8 | 11 | 17 | 21);
        let mut version =
            ToolVersion::new(version_str.to_string(), major, minor, patch).with_lts(is_lts);

        if let Some(build_num) = build {
            version = version.with_metadata(build_num);
        }

        Ok(version)
    }

    fn validate_installation(&self, path: &Path) -> Result<bool> {
        // Check for macOS JDK structure (Contents/Home/bin/java)
        if path.join("Contents/Home/bin/java").exists() {
            return Ok(true);
        }

        // Check for standard structure (bin/java)
        if path.join("bin/java").exists() {
            return Ok(true);
        }

        // Check for Windows structure (bin/java.exe)
        if path.join("bin/java.exe").exists() {
            return Ok(true);
        }

        Ok(false)
    }

    fn get_executable_paths(&self, install_path: &Path) -> Result<Vec<PathBuf>> {
        let java_home = if install_path.join("Contents/Home").exists() {
            install_path.join("Contents/Home")
        } else {
            install_path.to_path_buf()
        };

        let bin_dir = java_home.join("bin");

        if cfg!(windows) {
            Ok(vec![
                bin_dir.join("java.exe"),
                bin_dir.join("javac.exe"),
                bin_dir.join("jar.exe"),
            ])
        } else {
            Ok(vec![
                bin_dir.join("java"),
                bin_dir.join("javac"),
                bin_dir.join("jar"),
            ])
        }
    }

    fn get_environment_vars(&self, install_path: &Path) -> Result<Vec<(String, String)>> {
        let java_home = if install_path.join("Contents/Home").exists() {
            install_path.join("Contents/Home")
        } else {
            install_path.to_path_buf()
        };

        Ok(vec![
            ("JAVA_HOME".to_string(), java_home.display().to_string()),
            (
                "PATH".to_string(),
                format!("{}/bin:$PATH", java_home.display()),
            ),
        ])
    }
}

#[async_trait]
impl crate::core::traits::ToolInstaller for JavaPlugin {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &Path,
    ) -> Result<InstalledTool> {
        self.installer.install(distribution, dest_dir).await
    }

    async fn uninstall(&self, installed: &InstalledTool) -> Result<()> {
        self.installer.uninstall(installed).await
    }

    async fn verify(&self, installed: &InstalledTool) -> Result<bool> {
        self.installer.verify(installed).await
    }
}

#[async_trait]
impl crate::core::traits::ToolDetector for JavaPlugin {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        self.detector.detect_installations().await
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &Path,
    ) -> Result<InstalledTool> {
        self.detector.import_installation(detected, dest_dir).await
    }
}

impl ToolPlugin for JavaPlugin {
    fn supports_platform(&self, platform: Platform, arch: Architecture) -> bool {
        matches!(
            (platform, arch),
            (Platform::Mac, Architecture::X64)
                | (Platform::Mac, Architecture::Aarch64)
                | (Platform::Linux, Architecture::X64)
                | (Platform::Linux, Architecture::Aarch64)
                | (Platform::Windows, Architecture::X64)
        )
    }
}

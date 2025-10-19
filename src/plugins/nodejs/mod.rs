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

pub use api::NodeJsApi;
pub use detector::NodeJsDetector;
pub use installer::NodeJsInstaller;

pub struct NodeJsPlugin {
    api: NodeJsApi,
    installer: NodeJsInstaller,
    detector: NodeJsDetector,
}

impl NodeJsPlugin {
    pub fn new() -> Self {
        Self {
            api: NodeJsApi::new(),
            installer: NodeJsInstaller::new(),
            detector: NodeJsDetector::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "node".to_string(),
            name: "Node.js".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: "JCVM Contributors".to_string(),
            platforms: vec![Platform::Mac, Platform::Linux, Platform::Windows],
            architectures: vec![Architecture::X64, Architecture::Aarch64],
            category: PluginCategory::Runtime,
            builtin: true,
        }
    }
}

impl Default for NodeJsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolProvider for NodeJsPlugin {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: "node".to_string(),
            name: "Node.js".to_string(),
            description: "Node.js JavaScript runtime built on Chrome's V8 engine".to_string(),
            homepage: Some("https://nodejs.org".to_string()),
            docs_url: Some("https://nodejs.org/docs".to_string()),
        }
    }

    async fn list_remote_versions(&self, lts_only: bool) -> Result<Vec<ToolVersion>> {
        if lts_only {
            self.api.list_lts_versions().await
        } else {
            self.api.list_available_versions().await
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
        let cleaned = version_str.trim_start_matches('v');

        // Handle special version aliases
        let resolved = match cleaned {
            "lts" | "lts/*" => {
                // Return latest LTS (should be resolved by caller)
                return Err(crate::error::JcvmError::InvalidVersion(
                    "LTS alias not yet resolved. Use 'jcvm list node --lts' to see LTS versions."
                        .to_string(),
                ));
            }
            "latest" | "current" => {
                // Return latest version (should be resolved by caller)
                return Err(crate::error::JcvmError::InvalidVersion(
                    "Latest alias not yet resolved. Use 'jcvm list node' to see available versions.".to_string()
                ));
            }
            _ => cleaned,
        };

        let parts: Vec<&str> = resolved.split('.').collect();

        let major = parts.first()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| crate::error::JcvmError::InvalidVersion(version_str.to_string()))?;

        let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());

        // Check if this is an LTS version
        let is_lts = matches!(major, 14 | 16 | 18 | 20 | 22);

        let mut version =
            ToolVersion::new(resolved.to_string(), major, minor, patch).with_lts(is_lts);

        // Add LTS code name if available
        if is_lts {
            let lts_name = match major {
                22 => Some("Jod"),
                20 => Some("Iron"),
                18 => Some("Hydrogen"),
                16 => Some("Gallium"),
                14 => Some("Fermium"),
                _ => None,
            };

            if let Some(name) = lts_name {
                version = version.with_metadata(format!("lts:{}", name));
            }
        }

        Ok(version)
    }

    fn validate_installation(&self, path: &Path) -> Result<bool> {
        Ok(path.join("bin/node").exists() || path.join("node.exe").exists())
    }

    fn get_executable_paths(&self, install_path: &Path) -> Result<Vec<PathBuf>> {
        if cfg!(windows) {
            Ok(vec![
                install_path.join("node.exe"),
                install_path.join("npm.cmd"),
                install_path.join("npx.cmd"),
            ])
        } else {
            Ok(vec![
                install_path.join("bin/node"),
                install_path.join("bin/npm"),
                install_path.join("bin/npx"),
            ])
        }
    }

    fn get_environment_vars(&self, install_path: &Path) -> Result<Vec<(String, String)>> {
        let bin_path = if cfg!(windows) {
            install_path.display().to_string()
        } else {
            format!("{}/bin", install_path.display())
        };

        Ok(vec![
            ("NODE_HOME".to_string(), install_path.display().to_string()),
            ("PATH".to_string(), format!("{}:$PATH", bin_path)),
        ])
    }
}

#[async_trait]
impl crate::core::traits::ToolInstaller for NodeJsPlugin {
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
impl crate::core::traits::ToolDetector for NodeJsPlugin {
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

impl ToolPlugin for NodeJsPlugin {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_standard() {
        let plugin = NodeJsPlugin::new();

        let version = plugin.parse_version("20.10.0").unwrap();
        assert_eq!(version.major, 20);
        assert_eq!(version.minor, Some(10));
        assert_eq!(version.patch, Some(0));
        assert!(version.is_lts);
        assert!(version.metadata.as_ref().unwrap().contains("Iron"));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        let plugin = NodeJsPlugin::new();

        let version = plugin.parse_version("v18.17.1").unwrap();
        assert_eq!(version.major, 18);
        assert_eq!(version.minor, Some(17));
        assert_eq!(version.patch, Some(1));
        assert!(version.is_lts);
    }

    #[test]
    fn test_parse_version_major_only() {
        let plugin = NodeJsPlugin::new();

        let version = plugin.parse_version("20").unwrap();
        assert_eq!(version.major, 20);
        assert_eq!(version.minor, None);
        assert_eq!(version.patch, None);
        assert!(version.is_lts);
    }

    #[test]
    fn test_parse_version_non_lts() {
        let plugin = NodeJsPlugin::new();

        let version = plugin.parse_version("19.0.0").unwrap();
        assert_eq!(version.major, 19);
        assert!(!version.is_lts);
        assert!(version.metadata.is_none());
    }

    #[test]
    fn test_parse_version_all_lts_versions() {
        let plugin = NodeJsPlugin::new();

        let lts_versions = vec![
            (22, "Jod"),
            (20, "Iron"),
            (18, "Hydrogen"),
            (16, "Gallium"),
            (14, "Fermium"),
        ];

        for (major, name) in lts_versions {
            let version = plugin.parse_version(&format!("{}.0.0", major)).unwrap();
            assert!(version.is_lts, "Version {} should be LTS", major);
            assert!(
                version.metadata.as_ref().unwrap().contains(name),
                "Version {} should have code name {}",
                major,
                name
            );
        }
    }

    #[test]
    fn test_validate_installation() {
        let plugin = NodeJsPlugin::new();
        let temp_dir = std::env::temp_dir();

        // Should return false for non-existent paths
        let result = plugin.validate_installation(&temp_dir).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_supports_platform() {
        let plugin = NodeJsPlugin::new();

        // Supported platforms
        assert!(plugin.supports_platform(Platform::Mac, Architecture::X64));
        assert!(plugin.supports_platform(Platform::Mac, Architecture::Aarch64));
        assert!(plugin.supports_platform(Platform::Linux, Architecture::X64));
        assert!(plugin.supports_platform(Platform::Linux, Architecture::Aarch64));
        assert!(plugin.supports_platform(Platform::Windows, Architecture::X64));

        // Unsupported combinations
        assert!(!plugin.supports_platform(Platform::Windows, Architecture::Aarch64));
        assert!(!plugin.supports_platform(Platform::Linux, Architecture::X86));
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = NodeJsPlugin::metadata();

        assert_eq!(metadata.id, "node");
        assert_eq!(metadata.name, "Node.js");
        assert!(metadata.builtin);
        assert_eq!(metadata.category, PluginCategory::Runtime);
        assert!(metadata.platforms.contains(&Platform::Mac));
        assert!(metadata.platforms.contains(&Platform::Linux));
        assert!(metadata.platforms.contains(&Platform::Windows));
    }

    #[test]
    fn test_plugin_info() {
        let plugin = NodeJsPlugin::new();
        let info = plugin.info();

        assert_eq!(info.id, "node");
        assert_eq!(info.name, "Node.js");
        assert!(info.description.contains("Node.js"));
        assert!(info.homepage.is_some());
        assert!(info.docs_url.is_some());
    }
}

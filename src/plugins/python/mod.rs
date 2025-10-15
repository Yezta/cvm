mod api;
mod detector;
mod installer;

use crate::core::traits::{
    Architecture, DetectedInstallation, InstalledTool, Platform, ToolDetector, ToolDistribution,
    ToolInfo, ToolInstaller, ToolPlugin, ToolProvider, ToolVersion,
};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use api::PythonApi;
use detector::PythonDetector;
use installer::PythonInstaller;

/// Python plugin for managing Python runtime versions
///
/// This plugin provides:
/// - Version discovery from python.org
/// - Installation management (source builds on Linux, pkg on macOS, exe on Windows)
/// - Detection of system Python, pyenv, and Homebrew installations
/// - Virtual environment awareness
///
/// Note: This plugin primarily targets CPython. PyPy, Jython, and other
/// implementations are not currently supported but could be added as separate plugins.
pub struct PythonPlugin {
    api: PythonApi,
    installer: PythonInstaller,
    detector: PythonDetector,
}

impl PythonPlugin {
    /// Creates a new Python plugin
    pub fn new(install_dir: PathBuf, cache_dir: PathBuf) -> Self {
        Self {
            api: PythonApi::new(),
            installer: PythonInstaller::new(cache_dir),
            detector: PythonDetector::new(install_dir),
        }
    }

    /// Returns plugin metadata for registration
    pub fn metadata() -> crate::core::traits::PluginMetadata {
        crate::core::traits::PluginMetadata {
            id: "python".to_string(),
            name: "Python".to_string(),
            version: "1.0.0".to_string(),
            author: "JCVM Contributors".to_string(),
            platforms: vec![Platform::Mac, Platform::Linux, Platform::Windows],
            architectures: vec![Architecture::X64, Architecture::Aarch64, Architecture::X86],
            category: crate::core::traits::PluginCategory::Language,
            builtin: true,
        }
    }
}

#[async_trait]
impl ToolProvider for PythonPlugin {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: "python".to_string(),
            name: "Python".to_string(),
            description: "Python programming language runtime (CPython)".to_string(),
            homepage: Some("https://www.python.org".to_string()),
            docs_url: Some("https://docs.python.org".to_string()),
        }
    }

    async fn list_remote_versions(&self, lts_only: bool) -> Result<Vec<ToolVersion>> {
        // Python doesn't have an "LTS" designation, but we can filter to stable releases
        let versions = self.api.list_available_versions().await?;

        if lts_only {
            // Filter to only the latest patch versions of each minor release
            Ok(versions
                .into_iter()
                .filter(|v| {
                    // Only include stable versions (3.10+, no pre-release tags)
                    v.major == 3 && v.minor.unwrap_or(0) >= 10
                })
                .collect())
        } else {
            Ok(versions)
        }
    }

    async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        self.api.find_distribution(version, &platform, &arch).await
    }

    fn parse_version(&self, version_str: &str) -> Result<ToolVersion> {
        // Parse versions like "3.12.8", "3.11.0"
        let parts: Vec<&str> = version_str.split('.').collect();

        let major = parts
            .get(0)
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| crate::error::JcvmError::InvalidVersion(version_str.to_string()))?;

        let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok());

        Ok(ToolVersion::new(
            version_str.to_string(),
            major,
            minor,
            patch,
        ))
    }

    fn validate_installation(&self, path: &PathBuf) -> Result<bool> {
        let python_exe = if cfg!(windows) {
            path.join("python.exe")
        } else {
            path.join("bin").join("python3")
        };

        Ok(python_exe.exists())
    }

    fn get_executable_paths(&self, install_path: &PathBuf) -> Result<Vec<PathBuf>> {
        if cfg!(windows) {
            Ok(vec![
                install_path.join("python.exe"),
                install_path.join("Scripts").join("pip.exe"),
            ])
        } else {
            let bin_dir = install_path.join("bin");
            Ok(vec![
                bin_dir.join("python3"),
                bin_dir.join("pip3"),
                bin_dir.join("python"),
                bin_dir.join("pip"),
            ])
        }
    }

    fn get_environment_vars(&self, install_path: &PathBuf) -> Result<Vec<(String, String)>> {
        let mut env_vars = Vec::new();

        env_vars.push((
            "PYTHON_HOME".to_string(),
            install_path.to_string_lossy().to_string(),
        ));

        if cfg!(windows) {
            env_vars.push((
                "PATH".to_string(),
                format!(
                    "{};{}\\Scripts",
                    install_path.display(),
                    install_path.display()
                ),
            ));
        } else {
            env_vars.push((
                "PATH".to_string(),
                format!("{}/bin", install_path.display()),
            ));

            // Set library paths for shared libraries
            env_vars.push((
                "LD_LIBRARY_PATH".to_string(),
                format!("{}/lib", install_path.display()),
            ));
        }

        Ok(env_vars)
    }
}

#[async_trait]
impl ToolInstaller for PythonPlugin {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &PathBuf,
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
impl ToolDetector for PythonPlugin {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        self.detector.detect_installations().await
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        self.detector.import_installation(detected, dest_dir).await
    }
}

impl ToolPlugin for PythonPlugin {
    fn supports_platform(&self, platform: Platform, _arch: Architecture) -> bool {
        matches!(
            platform,
            Platform::Mac | Platform::Linux | Platform::Windows
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir, cache_dir);

        let info = plugin.info();
        assert_eq!(info.id, "python");
        assert_eq!(info.name, "Python");
    }

    #[test]
    fn test_plugin_metadata() {
        let metadata = PythonPlugin::metadata();
        assert_eq!(metadata.id, "python");
        assert_eq!(metadata.name, "Python");
        assert_eq!(metadata.version, "1.0.0");
        assert!(matches!(
            metadata.category,
            crate::core::traits::PluginCategory::Language
        ));
        assert!(metadata.builtin);
    }

    #[tokio::test]
    #[ignore] // Requires network access to GitHub releases
    async fn test_list_versions() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir, cache_dir);

        let versions = plugin.list_remote_versions(false).await.unwrap();
        assert!(!versions.is_empty());

        // Should have Python 3.12.x and 3.11.x versions
        assert!(versions
            .iter()
            .any(|v| v.major == 3 && v.minor.unwrap_or(0) == 12));
        assert!(versions
            .iter()
            .any(|v| v.major == 3 && v.minor.unwrap_or(0) == 11));
    }

    #[test]
    fn test_parse_version() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir, cache_dir);

        let version = plugin.parse_version("3.12.8").unwrap();
        assert_eq!(version.major, 3);
        assert_eq!(version.minor, Some(12));
        assert_eq!(version.patch, Some(8));
    }

    #[test]
    fn test_get_executable_paths_unix() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir.clone(), cache_dir);

        let install_path = PathBuf::from("/opt/jcvm/python/3.12.8");
        let executables = plugin.get_executable_paths(&install_path).unwrap();

        if !cfg!(windows) {
            assert!(executables.contains(&PathBuf::from("/opt/jcvm/python/3.12.8/bin/python3")));
            assert!(executables.contains(&PathBuf::from("/opt/jcvm/python/3.12.8/bin/pip3")));
        }
    }

    #[test]
    fn test_get_environment_vars() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir, cache_dir);

        let install_path = PathBuf::from("/opt/jcvm/python/3.12.8");
        let env_vars = plugin.get_environment_vars(&install_path).unwrap();

        // Convert to HashMap for easier testing
        let env_map: std::collections::HashMap<_, _> = env_vars.into_iter().collect();

        assert_eq!(
            env_map.get("PYTHON_HOME").unwrap(),
            "/opt/jcvm/python/3.12.8"
        );
        if !cfg!(windows) {
            assert!(env_map
                .get("PATH")
                .unwrap()
                .contains("/opt/jcvm/python/3.12.8/bin"));
        }
    }

    #[test]
    fn test_supports_platform() {
        let install_dir = PathBuf::from("/tmp/jcvm/python");
        let cache_dir = PathBuf::from("/tmp/jcvm/cache");
        let plugin = PythonPlugin::new(install_dir, cache_dir);

        assert!(plugin.supports_platform(Platform::Mac, Architecture::Aarch64));
        assert!(plugin.supports_platform(Platform::Linux, Architecture::X64));
        assert!(plugin.supports_platform(Platform::Windows, Architecture::X64));
    }
}

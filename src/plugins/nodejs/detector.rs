use crate::core::traits::{DetectedInstallation, InstalledTool, ToolVersion};
use crate::error::{JcvmError, Result};
use async_trait::async_trait;
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct NodeJsDetector;

impl NodeJsDetector {
    pub fn new() -> Self {
        Self
    }

    /// Reads .nvmrc or .node-version file in the given directory
    /// These files are used by nvm and other Node version managers
    pub fn read_node_version_file(&self, directory: &std::path::Path) -> Option<String> {
        // Try .nvmrc first (most common)
        let nvmrc_path = directory.join(".nvmrc");
        if nvmrc_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&nvmrc_path) {
                let version = content.trim().to_string();
                if !version.is_empty() {
                    return Some(version);
                }
            }
        }

        // Try .node-version (alternative)
        let node_version_path = directory.join(".node-version");
        if node_version_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&node_version_path) {
                let version = content.trim().to_string();
                if !version.is_empty() {
                    return Some(version);
                }
            }
        }

        None
    }

    /// Finds installation matching a version file in the given directory
    pub async fn find_for_version_file(
        &self,
        directory: &std::path::Path,
    ) -> Result<Option<DetectedInstallation>> {
        use crate::core::traits::ToolDetector;

        if let Some(version_str) = self.read_node_version_file(directory) {
            // Detect all installations
            let installations = self.detect_installations().await?;

            // Find matching version (support both "v20" and "20" format)
            let cleaned = version_str.trim_start_matches('v');

            for installation in installations {
                let inst_version = installation.version.to_string();
                // Exact match or major version match
                if inst_version == cleaned || inst_version.starts_with(&format!("{}.", cleaned)) {
                    return Ok(Some(installation));
                }
            }
        }

        Ok(None)
    }

    fn verify_node_home(&self, path: &Path) -> Result<Option<DetectedInstallation>> {
        let node_bin = if path.join("bin/node").exists() {
            path.join("bin/node")
        } else if path.join("node.exe").exists() {
            path.join("node.exe")
        } else if path.join("node").exists() {
            path.join("node")
        } else {
            return Ok(None);
        };

        let output = match Command::new(&node_bin).arg("--version").output() {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        let version_output = String::from_utf8_lossy(&output.stdout);
        let version = self.parse_node_version(&version_output)?;

        Ok(Some(DetectedInstallation {
            tool_id: "node".to_string(),
            version,
            path: path.to_path_buf(),
            source: "detected".to_string(),
            executable_path: Some(node_bin),
        }))
    }

    fn parse_node_version(&self, output: &str) -> Result<ToolVersion> {
        let version_str = output.trim().trim_start_matches('v');

        let parts: Vec<&str> = version_str.split('.').collect();

        if let Some(major_str) = parts.get(0) {
            if let Ok(major) = major_str.parse::<u32>() {
                let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());

                // LTS versions: 14 (Fermium), 16 (Gallium), 18 (Hydrogen), 20 (Iron), 22 (upcoming)
                let is_lts = matches!(major, 14 | 16 | 18 | 20 | 22);

                return Ok(
                    ToolVersion::new(version_str.to_string(), major, minor, patch).with_lts(is_lts),
                );
            }
        }

        Err(JcvmError::InvalidVersion(output.to_string()))
    }

    fn check_common_paths(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        #[cfg(target_os = "macos")]
        let paths = vec![
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/usr/local/lib/node_modules",
            "/opt/nodejs",
        ];

        #[cfg(target_os = "linux")]
        let paths = vec![
            "/usr/bin",
            "/usr/local/bin",
            "/opt/nodejs",
            "/usr/lib/nodejs",
        ];

        #[cfg(target_os = "windows")]
        let paths = vec![
            "C:\\Program Files\\nodejs",
            "C:\\Program Files (x86)\\nodejs",
        ];

        for base_path in paths {
            let path = PathBuf::from(base_path);
            if path.exists() {
                if let Ok(Some(installation)) = self.verify_node_home(&path) {
                    detected.push(installation);
                }
            }
        }

        Ok(detected)
    }

    fn check_path(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        let which_cmd = if cfg!(windows) { "where" } else { "which" };

        if let Ok(output) = Command::new(which_cmd).arg("node").output() {
            if let Ok(node_path) = String::from_utf8(output.stdout) {
                let node_path = node_path.trim();
                if !node_path.is_empty() {
                    let path = PathBuf::from(node_path);
                    if let Some(bin_dir) = path.parent() {
                        if let Some(node_home) = bin_dir.parent() {
                            if let Ok(Some(installation)) = self.verify_node_home(node_home) {
                                detected.push(installation);
                            }
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    fn check_nvm(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        let nvm_dir = if let Ok(dir) = std::env::var("NVM_DIR") {
            PathBuf::from(dir)
        } else if let Some(home) = dirs::home_dir() {
            home.join(".nvm")
        } else {
            return Ok(detected);
        };

        let versions_dir = nvm_dir.join("versions").join("node");
        if versions_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(Some(installation)) = self.verify_node_home(&path) {
                            detected.push(installation);
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Check for project-local Node.js installations (similar to Python venv)
    fn check_local_installations(&self, search_path: &Path) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        // Common local installation directories
        let local_dirs = vec!["node", ".node", "local/node", ".local/node"];

        for dir_name in local_dirs {
            let local_path = search_path.join(dir_name);
            if local_path.exists() && local_path.is_dir() {
                if let Ok(Some(installation)) = self.verify_node_home(&local_path) {
                    detected.push(installation);
                }
            }
        }

        Ok(detected)
    }
}

impl Default for NodeJsDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::core::traits::ToolDetector for NodeJsDetector {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        detected.extend(self.check_common_paths()?);
        detected.extend(self.check_path()?);
        detected.extend(self.check_nvm()?);

        // Check current directory for local installations
        if let Ok(current_dir) = std::env::current_dir() {
            detected.extend(self.check_local_installations(&current_dir)?);
        }

        if let Ok(node_home) = std::env::var("NODE_HOME") {
            if let Ok(Some(installation)) = self.verify_node_home(&PathBuf::from(node_home)) {
                detected.push(installation);
            }
        }

        detected.sort_by(|a, b| a.path.cmp(&b.path));
        detected.dedup_by(|a, b| a.path == b.path);

        Ok(detected)
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        let version_str = detected.version.to_string();

        if dest_dir.exists() {
            if dest_dir.is_symlink() && std::fs::read_link(dest_dir)? == detected.path {
                return Err(JcvmError::VersionAlreadyInstalled(
                    version_str,
                    dest_dir.display().to_string(),
                ));
            }
        }

        println!(
            "{} Node.js {} from {}",
            "Importing".green().bold(),
            version_str.cyan(),
            detected.path.display().to_string().yellow()
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&detected.path, dest_dir)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&detected.path, dest_dir)?;
        }

        println!(
            "{} Node.js {} imported successfully",
            "✓".green().bold(),
            version_str.cyan()
        );
        println!(
            "  {} {}",
            "Source:".dimmed(),
            detected.path.display().to_string().dimmed()
        );
        println!(
            "  {} {}",
            "Linked:".dimmed(),
            dest_dir.display().to_string().dimmed()
        );

        Ok(InstalledTool {
            tool_id: "node".to_string(),
            version: detected.version.clone(),
            path: dest_dir.clone(),
            installed_at: chrono::Utc::now(),
            source: detected.source.clone(),
            executable_path: detected.executable_path.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_read_node_version_file_nvmrc() {
        let detector = NodeJsDetector::new();
        let temp_dir = TempDir::new().unwrap();

        // Create .nvmrc file
        let nvmrc_path = temp_dir.path().join(".nvmrc");
        fs::write(&nvmrc_path, "20.10.0\n").unwrap();

        let version = detector.read_node_version_file(temp_dir.path());
        assert_eq!(version, Some("20.10.0".to_string()));
    }

    #[test]
    fn test_read_node_version_file_node_version() {
        let detector = NodeJsDetector::new();
        let temp_dir = TempDir::new().unwrap();

        // Create .node-version file
        let node_version_path = temp_dir.path().join(".node-version");
        fs::write(&node_version_path, "v18.17.1\n").unwrap();

        let version = detector.read_node_version_file(temp_dir.path());
        assert_eq!(version, Some("v18.17.1".to_string()));
    }

    #[test]
    fn test_read_node_version_file_precedence() {
        let detector = NodeJsDetector::new();
        let temp_dir = TempDir::new().unwrap();

        // Create both files - .nvmrc should take precedence
        let nvmrc_path = temp_dir.path().join(".nvmrc");
        let node_version_path = temp_dir.path().join(".node-version");
        fs::write(&nvmrc_path, "20.10.0\n").unwrap();
        fs::write(&node_version_path, "18.17.1\n").unwrap();

        let version = detector.read_node_version_file(temp_dir.path());
        assert_eq!(version, Some("20.10.0".to_string()));
    }

    #[test]
    fn test_read_node_version_file_missing() {
        let detector = NodeJsDetector::new();
        let temp_dir = TempDir::new().unwrap();

        let version = detector.read_node_version_file(temp_dir.path());
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_node_version() {
        let detector = NodeJsDetector::new();

        // Standard version
        let version = detector.parse_node_version("v20.10.0").unwrap();
        assert_eq!(version.major, 20);
        assert_eq!(version.minor, Some(10));
        assert_eq!(version.patch, Some(0));
        assert!(version.is_lts);

        // Without 'v' prefix
        let version = detector.parse_node_version("18.17.1").unwrap();
        assert_eq!(version.major, 18);
        assert_eq!(version.minor, Some(17));
        assert_eq!(version.patch, Some(1));

        // Non-LTS version
        let version = detector.parse_node_version("v19.0.0").unwrap();
        assert_eq!(version.major, 19);
        assert!(!version.is_lts);
    }
}

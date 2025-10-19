use crate::core::traits::{DetectedInstallation, InstalledTool, ToolVersion};
use crate::error::{JcvmError, Result};
use async_trait::async_trait;
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct JavaDetector;

impl JavaDetector {
    pub fn new() -> Self {
        Self
    }

    /// Verify if a path contains a valid Java installation
    fn verify_java_home(&self, path: &Path) -> Result<Option<DetectedInstallation>> {
        let java_bin = if path.join("bin/java").exists() {
            path.join("bin/java")
        } else if path.join("bin/java.exe").exists() {
            path.join("bin/java.exe")
        } else {
            return Ok(None);
        };

        // Get version from java -version
        let output = match Command::new(&java_bin).arg("-version").output() {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        let version_output = String::from_utf8_lossy(&output.stderr);
        let version = self.parse_java_version(&version_output)?;

        Ok(Some(DetectedInstallation {
            tool_id: "java".to_string(),
            version,
            path: path.to_path_buf(),
            source: "detected".to_string(),
            executable_path: Some(java_bin),
        }))
    }

    /// Parse version from `java -version` output
    fn parse_java_version(&self, output: &str) -> Result<ToolVersion> {
        // Example output: 'openjdk version "21.0.1" 2023-10-17'
        // or: 'java version "1.8.0_392"'

        for line in output.lines() {
            if line.contains("version") {
                if let Some(version_str) = line.split('"').nth(1) {
                    // Handle old format like "1.8.0_392"
                    let cleaned = if version_str.starts_with("1.") {
                        version_str.split('.').nth(1).unwrap_or(version_str)
                    } else {
                        version_str
                    };

                    let parts: Vec<&str> = cleaned.split(&['.', '_', '+'][..]).collect();

                    if let Some(major_str) = parts.first() {
                        if let Ok(major) = major_str.parse::<u32>() {
                            let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                            let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());

                            let is_lts = matches!(major, 8 | 11 | 17 | 21);

                            return Ok(ToolVersion::new(
                                version_str.to_string(),
                                major,
                                minor,
                                patch,
                            )
                            .with_lts(is_lts));
                        }
                    }
                }
            }
        }

        Err(JcvmError::InvalidVersion(output.to_string()))
    }

    /// Check common installation paths
    fn check_common_paths(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        #[cfg(target_os = "macos")]
        let paths = vec![
            "/Library/Java/JavaVirtualMachines",
            "/System/Library/Java/JavaVirtualMachines",
        ];

        #[cfg(target_os = "linux")]
        let paths = vec!["/usr/lib/jvm", "/usr/local/jvm", "/opt/java", "/opt/jdk"];

        #[cfg(target_os = "windows")]
        let paths = vec![
            "C:\\Program Files\\Java",
            "C:\\Program Files (x86)\\Java",
            "C:\\Program Files\\Eclipse Adoptium",
        ];

        for base_path in paths {
            let base = PathBuf::from(base_path);
            if !base.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();

                    // On macOS, check Contents/Home
                    let java_home = if path.join("Contents/Home").exists() {
                        path.join("Contents/Home")
                    } else {
                        path
                    };

                    if let Ok(Some(installation)) = self.verify_java_home(&java_home) {
                        detected.push(installation);
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Check PATH for java installations
    fn check_path(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        if let Ok(output) = Command::new("which").arg("java").output() {
            if let Ok(java_path) = String::from_utf8(output.stdout) {
                let java_path = java_path.trim();
                if !java_path.is_empty() {
                    let path = PathBuf::from(java_path);
                    // Get parent of bin directory
                    if let Some(bin_dir) = path.parent() {
                        if let Some(java_home) = bin_dir.parent() {
                            if let Ok(Some(installation)) = self.verify_java_home(java_home) {
                                detected.push(installation);
                            }
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    /// macOS specific: Check /usr/libexec/java_home
    #[cfg(target_os = "macos")]
    fn check_macos_java_home(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        if let Ok(output) = Command::new("/usr/libexec/java_home").arg("-V").output() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stderr.lines() {
                // Parse lines like: "    21.0.1 (x86_64) "Eclipse Temurin 21.0.1" - "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home"
                if let Some(path_start) = line.rfind('"') {
                    if let Some(path_str) = line.get(path_start + 1..) {
                        let path = PathBuf::from(path_str.trim_end_matches('"'));
                        if let Ok(Some(installation)) = self.verify_java_home(&path) {
                            detected.push(installation);
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    #[cfg(not(target_os = "macos"))]
    fn check_macos_java_home(&self) -> Result<Vec<DetectedInstallation>> {
        Ok(Vec::new())
    }
}

impl Default for JavaDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::core::traits::ToolDetector for JavaDetector {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        // Check common installation paths
        detected.extend(self.check_common_paths()?);

        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            if let Ok(Some(installation)) = self.verify_java_home(&PathBuf::from(java_home)) {
                detected.push(installation);
            }
        }

        // Check PATH
        detected.extend(self.check_path()?);

        // macOS specific: Check /usr/libexec/java_home
        detected.extend(self.check_macos_java_home()?);

        // Deduplicate by path
        detected.sort_by(|a, b| a.path.cmp(&b.path));
        detected.dedup_by(|a, b| a.path == b.path);

        Ok(detected)
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &Path,
    ) -> Result<InstalledTool> {
        let version_str = detected.version.to_string();

        // Check if already managed
        if dest_dir.exists()
            && dest_dir.is_symlink()
            && std::fs::read_link(dest_dir)? == detected.path
        {
            return Err(JcvmError::VersionAlreadyInstalled(
                version_str,
                dest_dir.display().to_string(),
            ));
        }

        println!(
            "{} JDK {} from {}",
            "Importing".green().bold(),
            version_str.cyan(),
            detected.path.display().to_string().yellow()
        );

        // Create symlink to the detected installation
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&detected.path, dest_dir)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&detected.path, dest_dir)?;
        }

        println!(
            "{} JDK {} imported successfully",
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
            tool_id: "java".to_string(),
            version: detected.version.clone(),
            path: dest_dir.to_path_buf(),
            installed_at: chrono::Utc::now(),
            source: detected.source.clone(),
            executable_path: detected.executable_path.clone(),
        })
    }
}

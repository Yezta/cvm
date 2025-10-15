use crate::config::Config;
use crate::error::{JcvmError, Result};
use crate::models::{InstalledJdk, Version};
use colored::*;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct JavaDetector {
    config: Config,
}

impl JavaDetector {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Detect all Java installations on the system
    pub fn detect_all(&self) -> Result<Vec<DetectedJava>> {
        let mut detected = Vec::new();

        // Check common installation paths
        detected.extend(self.check_common_paths()?);

        // Check JAVA_HOME
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            if let Some(java) = self.verify_java_home(&PathBuf::from(java_home))? {
                detected.push(java);
            }
        }

        // Check PATH
        detected.extend(self.check_path()?);

        // macOS specific: Check /usr/libexec/java_home
        #[cfg(target_os = "macos")]
        {
            detected.extend(self.check_macos_java_home()?);
        }

        // Deduplicate by path
        detected.sort_by(|a, b| a.path.cmp(&b.path));
        detected.dedup_by(|a, b| a.path == b.path);

        Ok(detected)
    }

    /// Import detected Java installation into JCVM
    pub fn import(&self, detected: &DetectedJava) -> Result<InstalledJdk> {
        let version_str = detected.version.to_string();
        let version_dir = self.config.get_version_dir(&version_str);

        // Check if already managed by JCVM
        if version_dir.exists() {
            if version_dir.is_symlink() && std::fs::read_link(&version_dir)? == detected.path {
                return Err(JcvmError::VersionAlreadyInstalled(
                    version_str,
                    version_dir.display().to_string(),
                ));
            }
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
            std::os::unix::fs::symlink(&detected.path, &version_dir)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&detected.path, &version_dir)?;
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
            version_dir.display().to_string().dimmed()
        );

        Ok(InstalledJdk {
            version: detected.version.clone(),
            path: version_dir,
            installed_at: chrono::Utc::now(),
            distribution: detected.source.clone(),
        })
    }

    /// Check common Java installation paths
    fn check_common_paths(&self) -> Result<Vec<DetectedJava>> {
        let mut detected = Vec::new();

        let common_paths = if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/Library/Java/JavaVirtualMachines"),
                PathBuf::from("/System/Library/Java/JavaVirtualMachines"),
            ]
        } else if cfg!(target_os = "linux") {
            vec![
                PathBuf::from("/usr/lib/jvm"),
                PathBuf::from("/usr/java"),
                PathBuf::from("/opt/java"),
                PathBuf::from("/opt/jdk"),
            ]
        } else if cfg!(target_os = "windows") {
            vec![
                PathBuf::from(r"C:\Program Files\Java"),
                PathBuf::from(r"C:\Program Files (x86)\Java"),
                PathBuf::from(r"C:\Program Files\Eclipse Adoptium"),
            ]
        } else {
            vec![]
        };

        for base_path in common_paths {
            if !base_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // On macOS, the actual JDK is under Contents/Home
                        let java_home = if cfg!(target_os = "macos") {
                            path.join("Contents").join("Home")
                        } else {
                            path.clone()
                        };

                        if let Some(java) = self.verify_java_home(&java_home)? {
                            detected.push(java);
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Check for Java in PATH
    fn check_path(&self) -> Result<Vec<DetectedJava>> {
        let mut detected = Vec::new();

        if let Ok(output) = Command::new("which").arg("java").output() {
            if output.status.success() {
                let java_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !java_path.is_empty() {
                    let java_path = PathBuf::from(java_path);

                    // Resolve symlinks
                    let real_path = std::fs::canonicalize(&java_path).unwrap_or(java_path);

                    // Navigate up to find JAVA_HOME (java -> bin -> home)
                    if let Some(bin_dir) = real_path.parent() {
                        if let Some(java_home) = bin_dir.parent() {
                            if let Some(java) = self.verify_java_home(java_home)? {
                                detected.push(java);
                            }
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    /// macOS specific: Use /usr/libexec/java_home to find installations
    #[cfg(target_os = "macos")]
    fn check_macos_java_home(&self) -> Result<Vec<DetectedJava>> {
        let mut detected = Vec::new();

        // List all versions
        if let Ok(output) = Command::new("/usr/libexec/java_home").arg("-V").output() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            for line in stderr.lines() {
                // Parse lines like: "    21.0.1 (x86_64) "Java SE 21.0.1" - "/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home""
                if let Some(path_start) = line.rfind('"') {
                    if let Some(path_end) = line[..path_start].rfind('"') {
                        let path = &line[path_end + 1..path_start];
                        let java_home = PathBuf::from(path);

                        if let Some(java) = self.verify_java_home(&java_home)? {
                            detected.push(java);
                        }
                    }
                }
            }
        }

        Ok(detected)
    }

    /// Verify a path is a valid JAVA_HOME and extract version info
    fn verify_java_home(&self, path: &Path) -> Result<Option<DetectedJava>> {
        let java_bin = if cfg!(target_os = "windows") {
            path.join("bin").join("java.exe")
        } else {
            path.join("bin").join("java")
        };

        if !java_bin.exists() {
            return Ok(None);
        }

        // Get version information
        let output = Command::new(&java_bin).arg("-version").output();

        if let Ok(output) = output {
            if output.status.success() {
                let version_info = String::from_utf8_lossy(&output.stderr);

                if let Some(version) = self.parse_version_from_output(&version_info) {
                    let source = self.detect_source(path, &version_info);

                    return Ok(Some(DetectedJava {
                        path: path.to_path_buf(),
                        version,
                        source,
                        raw_version: version_info.lines().next().unwrap_or("").to_string(),
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Parse version from java -version output
    fn parse_version_from_output(&self, output: &str) -> Option<Version> {
        // Look for version pattern like: version "21.0.1" or version "1.8.0_392"
        for line in output.lines() {
            if line.contains("version") {
                if let Some(start) = line.find('"') {
                    if let Some(end) = line[start + 1..].find('"') {
                        let version_str = &line[start + 1..start + 1 + end];

                        // Handle legacy 1.8.x format
                        let version_str = if version_str.starts_with("1.8") {
                            "8"
                        } else {
                            version_str
                        };

                        // Extract major.minor.patch
                        let parts: Vec<&str> = version_str.split(['.', '_', '-']).collect();

                        if let Some(major_str) = parts.get(0) {
                            if let Ok(major) = major_str.parse::<u32>() {
                                let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                                let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());

                                return Some(Version {
                                    major,
                                    minor,
                                    patch,
                                    build: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Detect the source/distribution of a Java installation
    fn detect_source(&self, path: &Path, version_info: &str) -> String {
        let path_str = path.to_string_lossy().to_lowercase();
        let version_lower = version_info.to_lowercase();

        if version_lower.contains("temurin") || path_str.contains("temurin") {
            "adoptium".to_string()
        } else if version_lower.contains("corretto") || path_str.contains("corretto") {
            "corretto".to_string()
        } else if version_lower.contains("zulu") || path_str.contains("zulu") {
            "zulu".to_string()
        } else if version_lower.contains("graalvm") || path_str.contains("graalvm") {
            "graalvm".to_string()
        } else if version_lower.contains("oracle") || path_str.contains("oracle") {
            "oracle".to_string()
        } else if version_lower.contains("openjdk") || path_str.contains("openjdk") {
            "openjdk".to_string()
        } else {
            "system".to_string()
        }
    }
}

/// Represents a detected Java installation
#[derive(Debug, Clone)]
pub struct DetectedJava {
    pub path: PathBuf,
    pub version: Version,
    pub source: String,
    pub raw_version: String,
}

impl DetectedJava {
    pub fn display_info(&self) -> String {
        format!(
            "JDK {} ({}) at {}",
            self.version.to_string().cyan(),
            self.source.yellow(),
            self.path.display().to_string().dimmed()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let detector = JavaDetector::new(Config::default());

        let output1 = r#"openjdk version "21.0.1" 2023-10-17"#;
        let version = detector.parse_version_from_output(output1);
        assert!(version.is_some());
        assert_eq!(version.unwrap().major, 21);

        let output2 = r#"java version "1.8.0_392""#;
        let version = detector.parse_version_from_output(output2);
        assert!(version.is_some());
        assert_eq!(version.unwrap().major, 8);
    }

    #[test]
    fn test_detect_source() {
        let detector = JavaDetector::new(Config::default());

        let path = PathBuf::from("/Library/Java/JavaVirtualMachines/temurin-21.jdk");
        let source = detector.detect_source(&path, "Temurin");
        assert_eq!(source, "adoptium");

        let path = PathBuf::from("/usr/lib/jvm/java-17-openjdk");
        let source = detector.detect_source(&path, "OpenJDK");
        assert_eq!(source, "openjdk");
    }
}

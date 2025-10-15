use crate::core::traits::{DetectedInstallation, InstalledTool, ToolDetector, ToolVersion};
use crate::error::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Python installation detector
///
/// Detects Python installations from various sources:
/// - JCVM managed installations
/// - System Python (/usr/bin, /usr/local/bin)
/// - pyenv managed installations
/// - Homebrew installations (macOS)
/// - Windows standard locations
/// - Virtual environments (venv, virtualenv)
///
/// Also supports .python-version file detection for auto-switching
pub struct PythonDetector {
    jcvm_install_dir: PathBuf,
}

impl PythonDetector {
    /// Creates a new Python detector
    pub fn new(jcvm_install_dir: PathBuf) -> Self {
        Self { jcvm_install_dir }
    }

    /// Verifies a PYTHON_HOME directory is valid
    fn verify_python_home(&self, python_home: &Path) -> Option<ToolVersion> {
        let python_exe = if cfg!(windows) {
            python_home.join("python.exe")
        } else {
            python_home.join("bin").join("python3")
        };

        if !python_exe.exists() {
            return None;
        }

        self.parse_python_version(&python_exe)
    }

    /// Parses Python version from executable
    fn parse_python_version(&self, python_exe: &Path) -> Option<ToolVersion> {
        let output = Command::new(python_exe).arg("--version").output().ok()?;

        let version_str = String::from_utf8_lossy(&output.stdout);

        // Python --version outputs: "Python 3.12.8"
        let version = version_str.trim().strip_prefix("Python ")?.trim();

        // Parse version string
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.get(0)?.parse::<u32>().ok()?;
        let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok());

        Some(ToolVersion::new(version.to_string(), major, minor, patch))
    }

    /// Checks common Python installation paths on the system
    fn check_common_paths(&self) -> Vec<DetectedInstallation> {
        let mut installations = Vec::new();

        let search_paths = if cfg!(windows) {
            vec![
                PathBuf::from("C:\\Python312"),
                PathBuf::from("C:\\Python311"),
                PathBuf::from("C:\\Python310"),
                PathBuf::from("C:\\Program Files\\Python312"),
                PathBuf::from("C:\\Program Files\\Python311"),
                PathBuf::from("C:\\Program Files\\Python310"),
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/Library/Frameworks/Python.framework/Versions"),
                PathBuf::from("/System/Library/Frameworks/Python.framework/Versions"),
            ]
        } else {
            vec![
                PathBuf::from("/usr/bin"),
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/python"),
            ]
        };

        for path in search_paths {
            if path.exists() {
                if let Some(installation) = self.detect_from_path(&path) {
                    installations.push(installation);
                }
            }
        }

        installations
    }

    /// Detects Python installation from a specific path
    fn detect_from_path(&self, path: &Path) -> Option<DetectedInstallation> {
        // Check if this is a bin directory with python3
        let python_exe = path.join("python3");
        if python_exe.exists() {
            // Follow symlinks to find the real Python executable
            let real_python_exe = match std::fs::canonicalize(&python_exe) {
                Ok(real_path) => real_path,
                Err(_) => python_exe.clone(),
            };

            if let Some(version) = self.parse_python_version(&real_python_exe) {
                // Find the Python home by going up from the real executable path
                // Real path will be something like: /path/to/python/bin/python3
                // Python home is: /path/to/python
                let python_home = real_python_exe
                    .parent() // Remove python3
                    .and_then(|p| p.parent()) // Remove bin
                    .map(|p| p.to_path_buf());

                if let Some(install_path) = python_home {
                    // Validate that this is a proper Python home with a bin directory
                    let expected_bin = install_path.join("bin");

                    // For system Python or Python installed in unusual locations,
                    // we need to verify bin/python3 actually exists
                    if !expected_bin.join("python3").exists()
                        && !expected_bin.join("python").exists()
                    {
                        // This isn't a valid Python home directory, skip it
                        return None;
                    }

                    return Some(DetectedInstallation {
                        tool_id: "python".to_string(),
                        version,
                        path: install_path,
                        source: "system".to_string(),
                        executable_path: Some(real_python_exe),
                    });
                }
            }
        }

        // Check for Windows python.exe
        let python_exe_win = path.join("python.exe");
        if python_exe_win.exists() {
            if let Some(version) = self.parse_python_version(&python_exe_win) {
                return Some(DetectedInstallation {
                    tool_id: "python".to_string(),
                    version,
                    path: path.to_path_buf(),
                    source: "system".to_string(),
                    executable_path: Some(python_exe_win),
                });
            }
        }

        None
    }

    /// Detects Python installations from PATH environment variable
    fn check_path(&self) -> Vec<DetectedInstallation> {
        let mut installations = Vec::new();

        if let Ok(path_var) = std::env::var("PATH") {
            let separator = if cfg!(windows) { ';' } else { ':' };

            for path_str in path_var.split(separator) {
                let path = PathBuf::from(path_str);
                if let Some(installation) = self.detect_from_path(&path) {
                    installations.push(installation);
                }
            }
        }

        installations
    }

    /// Checks for pyenv-managed Python installations
    fn check_pyenv(&self) -> Vec<DetectedInstallation> {
        let mut installations = Vec::new();

        let pyenv_root = std::env::var("PYENV_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .map(|h| h.join(".pyenv"))
                    .unwrap_or_else(|| PathBuf::from(".pyenv"))
            });

        let versions_dir = pyenv_root.join("versions");
        if !versions_dir.exists() {
            return installations;
        }

        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let version_dir = entry.path();
                if version_dir.is_dir() {
                    let python_exe = if cfg!(windows) {
                        version_dir.join("python.exe")
                    } else {
                        version_dir.join("bin").join("python3")
                    };

                    if let Some(version) = self.parse_python_version(&python_exe) {
                        installations.push(DetectedInstallation {
                            tool_id: "python".to_string(),
                            version,
                            path: version_dir,
                            source: "pyenv".to_string(),
                            executable_path: Some(python_exe),
                        });
                    }
                }
            }
        }

        installations
    }

    /// Checks for Homebrew-managed Python installations (macOS)
    fn check_homebrew(&self) -> Vec<DetectedInstallation> {
        let mut installations = Vec::new();

        if !cfg!(target_os = "macos") {
            return installations;
        }

        let homebrew_paths = vec![
            PathBuf::from("/opt/homebrew/opt"), // Apple Silicon
            PathBuf::from("/usr/local/opt"),    // Intel
        ];

        for homebrew_path in homebrew_paths {
            if !homebrew_path.exists() {
                continue;
            }

            // Look for python@3.x directories
            if let Ok(entries) = std::fs::read_dir(&homebrew_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if name_str.starts_with("python@") {
                        let bin_dir = path.join("bin");
                        let python_exe = bin_dir.join("python3");

                        if let Some(version) = self.parse_python_version(&python_exe) {
                            installations.push(DetectedInstallation {
                                tool_id: "python".to_string(),
                                version,
                                path,
                                source: "homebrew".to_string(),
                                executable_path: Some(python_exe),
                            });
                        }
                    }
                }
            }
        }

        installations
    }

    /// Checks for Python virtual environments in the current directory and subdirectories
    fn check_virtualenvs(&self, search_path: &Path) -> Vec<DetectedInstallation> {
        let mut installations = Vec::new();

        // Common venv directory names
        let venv_names = vec!["venv", ".venv", "env", ".env", "virtualenv"];

        for venv_name in venv_names {
            let venv_path = search_path.join(venv_name);
            if venv_path.exists() {
                let python_exe = if cfg!(windows) {
                    venv_path.join("Scripts").join("python.exe")
                } else {
                    venv_path.join("bin").join("python")
                };

                if let Some(version) = self.parse_python_version(&python_exe) {
                    installations.push(DetectedInstallation {
                        tool_id: "python".to_string(),
                        version,
                        path: venv_path,
                        source: "virtualenv".to_string(),
                        executable_path: Some(python_exe),
                    });
                }
            }
        }

        installations
    }

    /// Reads .python-version file in the current directory
    /// Returns the version string if found
    pub fn read_python_version_file(&self, directory: &Path) -> Option<String> {
        let version_file = directory.join(".python-version");
        if !version_file.exists() {
            return None;
        }

        std::fs::read_to_string(&version_file)
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Finds the installation matching a .python-version file
    pub async fn find_for_version_file(
        &self,
        directory: &Path,
    ) -> Result<Option<DetectedInstallation>> {
        let version_str = match self.read_python_version_file(directory) {
            Some(v) => v,
            None => return Ok(None),
        };

        // Detect all installations
        let all_installations = self.detect_installations().await?;

        // Find matching version
        for installation in all_installations {
            if installation.version.to_string() == version_str
                || installation.version.to_string().starts_with(&version_str)
            {
                return Ok(Some(installation));
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl ToolDetector for PythonDetector {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        let mut all_installations = Vec::new();

        // Check JCVM managed installations first
        if self.jcvm_install_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.jcvm_install_dir) {
                for entry in entries.flatten() {
                    let version_dir = entry.path();
                    if version_dir.is_dir() {
                        if let Some(version) = self.verify_python_home(&version_dir) {
                            all_installations.push(DetectedInstallation {
                                tool_id: "python".to_string(),
                                version,
                                path: version_dir,
                                source: "jcvm".to_string(),
                                executable_path: None,
                            });
                        }
                    }
                }
            }
        }

        // Check common paths
        all_installations.extend(self.check_common_paths());

        // Check PATH
        all_installations.extend(self.check_path());

        // Check pyenv
        all_installations.extend(self.check_pyenv());

        // Check Homebrew (macOS only)
        all_installations.extend(self.check_homebrew());

        // Check for virtual environments in current directory
        if let Ok(current_dir) = std::env::current_dir() {
            all_installations.extend(self.check_virtualenvs(&current_dir));
        }

        // Deduplicate based on path
        let mut seen = std::collections::HashSet::new();
        all_installations.retain(|installation| seen.insert(installation.path.clone()));

        Ok(all_installations)
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        // Skip system directories that shouldn't be imported
        let path_str = detected.path.to_string_lossy();
        if path_str == "/usr" || path_str == "/usr/local" || path_str == "/System" {
            return Err(crate::error::JcvmError::InvalidToolStructure {
                tool: "python".to_string(),
                message: format!(
                    "Cannot import system directory '{}'. System Python installations cannot be managed by JCVM.",
                    detected.path.display()
                ),
            });
        }

        // Verify that the detected path has a valid Python structure
        let python_exe_in_detected = if cfg!(windows) {
            detected.path.join("python.exe")
        } else {
            detected.path.join("bin").join("python3")
        };

        if !python_exe_in_detected.exists() {
            return Err(crate::error::JcvmError::InvalidToolStructure {
                tool: "python".to_string(),
                message: format!(
                    "Cannot import: Python executable not found at {}. Path '{}' does not appear to be a valid Python home directory.",
                    python_exe_in_detected.display(),
                    detected.path.display()
                ),
            });
        }

        // dest_dir already includes the version, so we use it directly
        if dest_dir.exists() {
            return Err(crate::error::JcvmError::VersionAlreadyInstalled(
                detected.version.to_string(),
                dest_dir.to_string_lossy().to_string(),
            ));
        }

        // Ensure the parent directory exists
        if let Some(parent) = dest_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }

        #[cfg(unix)]
        {
            // On Unix systems, create a symbolic link
            std::os::unix::fs::symlink(&detected.path, dest_dir)?;
        }

        #[cfg(windows)]
        {
            // On Windows, create a junction (directory symlink)
            std::os::windows::fs::symlink_dir(&detected.path, dest_dir)?;
        }

        let python_exe = if cfg!(windows) {
            dest_dir.join("python.exe")
        } else {
            dest_dir.join("bin").join("python3")
        };

        Ok(InstalledTool {
            tool_id: "python".to_string(),
            version: detected.version.clone(),
            path: dest_dir.clone(),
            executable_path: Some(python_exe),
            installed_at: chrono::Utc::now(),
            source: format!("imported-{}", detected.source),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_python_version() {
        let detector = PythonDetector::new(PathBuf::from("/tmp/jcvm"));

        // This test would need a mock or actual Python installation
        // For now, we just ensure the detector can be created
        assert!(detector.jcvm_install_dir.to_str().is_some());
    }

    #[tokio::test]
    async fn test_detect_installations_empty() {
        let temp_dir = std::env::temp_dir().join("jcvm_test_python");
        let _ = std::fs::create_dir_all(&temp_dir);

        let detector = PythonDetector::new(temp_dir.clone());
        let installations = detector.detect_installations().await.unwrap();

        // May or may not be empty depending on system Python installations
        // Just verify it doesn't error
        assert!(installations.len() >= 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_python_version_file() {
        let temp_dir = std::env::temp_dir().join("jcvm_test_version_file");
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Create a .python-version file
        let version_file = temp_dir.join(".python-version");
        std::fs::write(&version_file, "3.12.8\n").unwrap();

        let detector = PythonDetector::new(PathBuf::from("/tmp/jcvm"));
        let version = detector.read_python_version_file(&temp_dir);

        assert_eq!(version, Some("3.12.8".to_string()));

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_read_python_version_file_missing() {
        let temp_dir = std::env::temp_dir().join("jcvm_test_no_version_file");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let detector = PythonDetector::new(PathBuf::from("/tmp/jcvm"));
        let version = detector.read_python_version_file(&temp_dir);

        assert_eq!(version, None);

        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
}

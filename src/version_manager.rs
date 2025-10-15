use crate::config::Config;
use crate::error::{JcvmError, Result};
use crate::models::Version;
use std::path::PathBuf;

pub struct VersionManager {
    config: Config,
}

impl VersionManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Set the current active version
    pub fn set_current(&self, version: &str) -> Result<PathBuf> {
        let version_dir = self.config.get_version_dir(version);

        if !version_dir.exists() {
            return Err(JcvmError::VersionNotFound(version.to_string()));
        }

        // Check for macOS JDK structure (Contents/Home)
        let java_home = if version_dir.join("Contents/Home/bin/java").exists() {
            version_dir.join("Contents/Home")
        } else if version_dir.join("bin/java").exists() {
            version_dir.clone()
        } else {
            return Err(JcvmError::InvalidJdkStructure(format!(
                "Could not find java executable in {}",
                version_dir.display()
            )));
        };

        let current_link = self.config.current_version_symlink();

        // Remove existing symlink if present
        if current_link.exists() || current_link.is_symlink() {
            std::fs::remove_file(&current_link)?;
        }

        // Create new symlink pointing to the actual JAVA_HOME
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&java_home, &current_link)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&java_home, &current_link)?;
        }

        Ok(java_home)
    }

    /// Get the current active version
    pub fn get_current(&self) -> Result<Option<String>> {
        let current_link = self.config.current_version_symlink();

        if !current_link.exists() {
            return Ok(None);
        }

        let target = std::fs::read_link(&current_link)?;

        // Extract version number from path
        // Path could be either:
        // - .../versions/21/bin -> version is "21"
        // - .../versions/25/Contents/Home -> version is "25"
        let version = target
            .components()
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .find_map(|component| {
                if let Some(s) = component.as_os_str().to_str() {
                    // Skip "Home", "Contents", "bin" - look for version number
                    if s != "Home" && s != "Contents" && s != "bin" && s != "versions" {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        Ok(version)
    }

    /// Set an alias for a version
    pub fn set_alias(&self, alias: &str, version: &str) -> Result<()> {
        let version_dir = self.config.get_version_dir(version);

        if !version_dir.exists() {
            return Err(JcvmError::VersionNotFound(version.to_string()));
        }

        // Check for macOS JDK structure (Contents/Home)
        let java_home = if version_dir.join("Contents/Home/bin/java").exists() {
            version_dir.join("Contents/Home")
        } else if version_dir.join("bin/java").exists() {
            version_dir.clone()
        } else {
            return Err(JcvmError::InvalidJdkStructure(format!(
                "Could not find java executable in {}",
                version_dir.display()
            )));
        };

        let alias_path = self.config.get_alias_path(alias);

        // Remove existing alias if present
        if alias_path.exists() || alias_path.is_symlink() {
            std::fs::remove_file(&alias_path)?;
        }

        // Create new symlink pointing to the actual JAVA_HOME
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&java_home, &alias_path)?;
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&java_home, &alias_path)?;
        }

        Ok(())
    }

    /// Get the version for an alias
    pub fn get_alias(&self, alias: &str) -> Result<Option<String>> {
        let alias_path = self.config.get_alias_path(alias);

        if !alias_path.exists() {
            return Ok(None);
        }

        let target = std::fs::read_link(&alias_path)?;

        // Extract version number from path (same logic as get_current)
        let version = target
            .components()
            .collect::<Vec<_>>()
            .iter()
            .rev()
            .find_map(|component| {
                if let Some(s) = component.as_os_str().to_str() {
                    // Skip "Home", "Contents", "bin" - look for version number
                    if s != "Home" && s != "Contents" && s != "bin" && s != "versions" {
                        Some(s.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

        Ok(version)
    }

    /// Read version from .java-version file
    pub fn read_local_version() -> Result<Option<Version>> {
        let java_version_file = std::path::Path::new(".java-version");

        if !java_version_file.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(java_version_file)?;
        let version_str = contents.trim();

        if version_str.is_empty() {
            return Ok(None);
        }

        let version = version_str.parse::<Version>()?;
        Ok(Some(version))
    }

    /// Write version to .java-version file
    pub fn write_local_version(version: &Version) -> Result<()> {
        std::fs::write(".java-version", version.to_string())?;
        Ok(())
    }

    /// Get default version
    pub fn get_default(&self) -> Result<Option<String>> {
        self.get_alias("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_write_local_version() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let version = Version::new(21);
        VersionManager::write_local_version(&version).unwrap();

        let read_version = VersionManager::read_local_version().unwrap();
        assert_eq!(read_version, Some(version));

        std::env::set_current_dir(original_dir).unwrap();
    }
}

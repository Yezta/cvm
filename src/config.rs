use crate::error::{JcvmError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub jcvm_dir: PathBuf,
    
    #[serde(skip)]
    pub versions_dir: PathBuf,
    
    #[serde(skip)]
    pub alias_dir: PathBuf,
    
    #[serde(skip)]
    pub cache_dir: PathBuf,
    
    #[serde(skip)]
    pub config_file: PathBuf,

    /// Default JDK distribution (currently only "adoptium" is supported)
    pub default_distribution: String,

    /// Whether to verify checksums when downloading
    pub verify_checksums: bool,

    /// Cache download files
    pub cache_downloads: bool,

    /// Auto-cleanup old cache files (days)
    pub cache_retention_days: u32,

    /// Show LTS indicator in version lists
    pub show_lts_indicator: bool,

    /// Parallel downloads
    pub parallel_downloads: bool,
}

impl Default for Config {
    fn default() -> Self {
        let jcvm_dir = Self::default_jcvm_dir();
        
        Self {
            versions_dir: jcvm_dir.join("versions"),
            alias_dir: jcvm_dir.join("alias"),
            cache_dir: jcvm_dir.join("cache"),
            config_file: jcvm_dir.join("config.toml"),
            jcvm_dir,
            default_distribution: "adoptium".to_string(),
            verify_checksums: true,
            cache_downloads: true,
            cache_retention_days: 30,
            show_lts_indicator: true,
            parallel_downloads: true,
        }
    }
}

impl Config {
    fn default_jcvm_dir() -> PathBuf {
        // First check JCVM_DIR environment variable
        if let Ok(dir) = std::env::var("JCVM_DIR") {
            return PathBuf::from(shellexpand::tilde(&dir).to_string());
        }

        // Then use platform-specific directory
        if let Some(proj_dirs) = ProjectDirs::from("", "", "jcvm") {
            return proj_dirs.data_dir().to_path_buf();
        }

        // Fallback to ~/.jcvm
        PathBuf::from(shellexpand::tilde("~/.jcvm").to_string())
    }

    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Create directories if they don't exist
        std::fs::create_dir_all(&config.jcvm_dir)?;
        std::fs::create_dir_all(&config.versions_dir)?;
        std::fs::create_dir_all(&config.alias_dir)?;
        std::fs::create_dir_all(&config.cache_dir)?;

        // Load config file if it exists
        if config.config_file.exists() {
            let contents = std::fs::read_to_string(&config.config_file)?;
            let file_config: Config = toml::from_str(&contents)?;
            
            // Merge file config with defaults (only certain fields)
            config.default_distribution = file_config.default_distribution;
            config.verify_checksums = file_config.verify_checksums;
            config.cache_downloads = file_config.cache_downloads;
            config.cache_retention_days = file_config.cache_retention_days;
            config.show_lts_indicator = file_config.show_lts_indicator;
            config.parallel_downloads = file_config.parallel_downloads;
        } else {
            // Create default config file
            config.save()?;
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| JcvmError::ConfigError(e.to_string()))?;
        
        std::fs::write(&self.config_file, contents)?;
        Ok(())
    }

    pub fn get_version_dir(&self, version: &str) -> PathBuf {
        self.versions_dir.join(version)
    }

    pub fn get_alias_path(&self, alias: &str) -> PathBuf {
        self.alias_dir.join(alias)
    }

    pub fn current_version_symlink(&self) -> PathBuf {
        self.alias_dir.join("current")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.default_distribution, "adoptium");
        assert!(config.verify_checksums);
        assert!(config.cache_downloads);
    }
}

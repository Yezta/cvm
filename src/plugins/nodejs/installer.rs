use crate::core::traits::{InstalledTool, ToolDistribution};
use crate::download::Downloader;
use crate::error::{JcvmError, Result};
use async_trait::async_trait;
use colored::*;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;

pub struct NodeJsInstaller {
    downloader: Downloader,
}

impl NodeJsInstaller {
    pub fn new() -> Self {
        Self {
            downloader: Downloader::new(),
        }
    }

    fn extract_archive(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Extracting...");

        if archive_path
            .to_str()
            .map(|s| s.ends_with(".tar.gz"))
            .unwrap_or(false)
        {
            self.extract_tar_gz(archive_path, dest_dir)?;
        } else if archive_path.extension().and_then(|s| s.to_str()) == Some("zip") {
            self.extract_zip(archive_path, dest_dir)?;
        } else {
            return Err(JcvmError::ExtractionFailed(format!(
                "Unsupported archive format: {:?}",
                archive_path.extension()
            )));
        }

        pb.finish_with_message("Extraction complete");
        Ok(())
    }

    fn extract_tar_gz(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        let temp_dir = dest_dir.parent().unwrap().join(format!(
            ".tmp_{}",
            dest_dir.file_name().unwrap().to_str().unwrap()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        archive.unpack(&temp_dir)?;

        let entries: Vec<_> = std::fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1 && entries[0].path().is_dir() {
            std::fs::rename(&entries[0].path(), dest_dir)?;
            std::fs::remove_dir_all(&temp_dir)?;
        } else {
            std::fs::rename(&temp_dir, dest_dir)?;
        }

        Ok(())
    }

    fn extract_zip(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let temp_dir = dest_dir.parent().unwrap().join(format!(
            ".tmp_{}",
            dest_dir.file_name().unwrap().to_str().unwrap()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        archive.extract(&temp_dir)?;

        let entries: Vec<_> = std::fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1 && entries[0].path().is_dir() {
            std::fs::rename(&entries[0].path(), dest_dir)?;
            std::fs::remove_dir_all(&temp_dir)?;
        } else {
            std::fs::rename(&temp_dir, dest_dir)?;
        }

        Ok(())
    }
}

impl Default for NodeJsInstaller {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::core::traits::ToolInstaller for NodeJsInstaller {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        let version_str = distribution.version.to_string();

        if dest_dir.exists() {
            return Err(JcvmError::VersionAlreadyInstalled(
                version_str,
                dest_dir.display().to_string(),
            ));
        }

        println!(
            "{} Node.js {} for {}-{}",
            "Installing".green().bold(),
            version_str.cyan(),
            distribution.platform.to_string().yellow(),
            distribution.architecture.to_string().yellow()
        );

        let url_path = distribution
            .download_url
            .split('/')
            .last()
            .unwrap_or("node.tar.gz");

        let cache_dir = if let Ok(jcvm_dir) = std::env::var("JCVM_DIR") {
            PathBuf::from(jcvm_dir).join("cache")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".jcvm")
                .join("cache")
        };

        std::fs::create_dir_all(&cache_dir)?;
        let cache_file = cache_dir.join(url_path);

        // Download if not cached
        if !cache_file.exists() {
            self.downloader
                .download_with_progress(&distribution.download_url, &cache_file)
                .await?;
        } else {
            println!("{}", "Using cached download".yellow());
        }

        // Verify checksum if available
        if let Some(ref checksum) = distribution.checksum {
            println!("{}", "Verifying checksum...".yellow());
            let is_valid = Downloader::verify_checksum(&cache_file, checksum).await?;

            if !is_valid {
                std::fs::remove_file(&cache_file)?;
                return Err(JcvmError::ChecksumMismatch {
                    file: cache_file.display().to_string(),
                });
            }
            println!("{} {}", "✓".green().bold(), "Checksum verified".green());
        }

        println!("{}", "Extracting archive...".yellow());
        self.extract_archive(&cache_file, dest_dir)?;

        // Verify installation
        let executable_path = if cfg!(windows) {
            dest_dir.join("node.exe")
        } else {
            dest_dir.join("bin/node")
        };

        if !executable_path.exists() {
            return Err(JcvmError::InvalidToolStructure {
                tool: "node".to_string(),
                message: format!(
                    "Node.js executable not found at {}. Installation may be corrupted.",
                    executable_path.display()
                ),
            });
        }

        // Verify npm is included
        let npm_path = if cfg!(windows) {
            dest_dir.join("npm.cmd")
        } else {
            dest_dir.join("bin/npm")
        };

        if npm_path.exists() {
            println!("{} npm included", "✓".green().bold());
        }

        println!(
            "{} Node.js {} installed to {}",
            "✓".green().bold(),
            version_str.cyan(),
            dest_dir.display().to_string().dimmed()
        );

        Ok(InstalledTool {
            tool_id: "node".to_string(),
            version: distribution.version.clone(),
            path: dest_dir.clone(),
            installed_at: chrono::Utc::now(),
            source: "nodejs.org".to_string(),
            executable_path: Some(executable_path),
        })
    }

    async fn uninstall(&self, installed: &InstalledTool) -> Result<()> {
        if !installed.path.exists() {
            return Err(JcvmError::VersionNotFound(installed.version.to_string()));
        }

        println!(
            "{} {} {}",
            "Uninstalling".red().bold(),
            installed.tool_id.cyan(),
            installed.version.to_string().yellow()
        );

        std::fs::remove_dir_all(&installed.path)?;

        println!(
            "{} {} {} uninstalled successfully",
            "✓".green().bold(),
            installed.tool_id.cyan(),
            installed.version.to_string().yellow()
        );

        Ok(())
    }

    async fn verify(&self, installed: &InstalledTool) -> Result<bool> {
        if !installed.path.exists() {
            return Ok(false);
        }

        // Check node executable
        if let Some(exec_path) = &installed.executable_path {
            if !exec_path.exists() {
                return Ok(false);
            }

            // Try to run node --version to verify it works
            if let Ok(output) = std::process::Command::new(exec_path)
                .arg("--version")
                .output()
            {
                if !output.status.success() {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        } else {
            // Fallback check
            let node_path = if installed.path.join("bin/node").exists() {
                installed.path.join("bin/node")
            } else if installed.path.join("node.exe").exists() {
                installed.path.join("node.exe")
            } else {
                return Ok(false);
            };

            if !node_path.exists() {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tar_gz_detection() {
        let _installer = NodeJsInstaller::new();
        let path = Path::new("node-v20.10.0-linux-x64.tar.gz");
        assert!(path
            .to_str()
            .map(|s| s.ends_with(".tar.gz"))
            .unwrap_or(false));
    }

    #[test]
    fn test_extract_zip_detection() {
        let _installer = NodeJsInstaller::new();
        let path = Path::new("node-v20.10.0-win-x64.zip");
        assert_eq!(path.extension().and_then(|s| s.to_str()), Some("zip"));
    }
}

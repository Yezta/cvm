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

pub struct JavaInstaller {
    downloader: Downloader,
}

impl JavaInstaller {
    pub fn new() -> Self {
        Self {
            downloader: Downloader::new(),
        }
    }

    /// Extract archive based on file type
    fn extract_archive(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message("Extracting...");

        // Determine archive type from extension
        if archive_path.extension().and_then(|s| s.to_str()) == Some("gz")
            || archive_path
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

        // Extract to temporary directory first
        let temp_dir = dest_dir.parent().unwrap().join(format!(
            ".tmp_{}",
            dest_dir.file_name().unwrap().to_str().unwrap()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        archive.unpack(&temp_dir)?;

        // Find the root directory (usually has a version-specific name)
        let entries: Vec<_> = std::fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1 && entries[0].path().is_dir() {
            // Single root directory, move its contents
            std::fs::rename(entries[0].path(), dest_dir)?;
            std::fs::remove_dir_all(&temp_dir)?;
        } else {
            // Multiple files/dirs, move the temp dir itself
            std::fs::rename(&temp_dir, dest_dir)?;
        }

        Ok(())
    }

    fn extract_zip(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        // Extract to temporary directory first
        let temp_dir = dest_dir.parent().unwrap().join(format!(
            ".tmp_{}",
            dest_dir.file_name().unwrap().to_str().unwrap()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        archive.extract(&temp_dir)?;

        // Find the root directory
        let entries: Vec<_> = std::fs::read_dir(&temp_dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.len() == 1 && entries[0].path().is_dir() {
            // Single root directory, move its contents
            std::fs::rename(entries[0].path(), dest_dir)?;
            std::fs::remove_dir_all(&temp_dir)?;
        } else {
            // Multiple files/dirs, move the temp dir itself
            std::fs::rename(&temp_dir, dest_dir)?;
        }

        Ok(())
    }
}

impl Default for JavaInstaller {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl crate::core::traits::ToolInstaller for JavaInstaller {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &Path,
    ) -> Result<InstalledTool> {
        let version_str = distribution.version.to_string();

        // Check if already installed
        if dest_dir.exists() {
            return Err(JcvmError::VersionAlreadyInstalled(
                version_str,
                dest_dir.display().to_string(),
            ));
        }

        println!(
            "{} JDK {} for {}-{}",
            "Installing".green().bold(),
            version_str.cyan(),
            distribution.platform.to_string().yellow(),
            distribution.architecture.to_string().yellow()
        );

        // Determine cache file name
        let url_path = distribution
            .download_url
            .split('/')
            .next_back()
            .unwrap_or("jdk.tar.gz");

        // Create a cache directory based on JCVM_DIR or default
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

            // Verify checksum if available
            if let Some(checksum) = &distribution.checksum {
                println!("{}", "Verifying checksum...".yellow());
                if !Downloader::verify_checksum(&cache_file, checksum).await? {
                    std::fs::remove_file(&cache_file)?;
                    return Err(JcvmError::ChecksumMismatch {
                        file: cache_file.display().to_string(),
                    });
                }
                println!("{}", "✓ Checksum verified".green());
            }
        } else {
            println!("{}", "Using cached download".yellow());
        }

        // Extract archive
        println!("{}", "Extracting archive...".yellow());
        self.extract_archive(&cache_file, dest_dir)?;

        println!(
            "{} JDK {} installed to {}",
            "✓".green().bold(),
            version_str.cyan(),
            dest_dir.display().to_string().dimmed()
        );

        // Determine executable path
        let executable_path = if dest_dir.join("Contents/Home/bin/java").exists() {
            Some(dest_dir.join("Contents/Home/bin/java"))
        } else if dest_dir.join("bin/java").exists() {
            Some(dest_dir.join("bin/java"))
        } else if dest_dir.join("bin/java.exe").exists() {
            Some(dest_dir.join("bin/java.exe"))
        } else {
            None
        };

        Ok(InstalledTool {
            tool_id: "java".to_string(),
            version: distribution.version.clone(),
            path: dest_dir.to_path_buf(),
            installed_at: chrono::Utc::now(),
            source: "adoptium".to_string(),
            executable_path,
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

        // Check if the executable exists
        if let Some(exec_path) = &installed.executable_path {
            return Ok(exec_path.exists());
        }

        // Fallback: check common Java paths
        Ok(installed.path.join("Contents/Home/bin/java").exists()
            || installed.path.join("bin/java").exists()
            || installed.path.join("bin/java.exe").exists())
    }
}

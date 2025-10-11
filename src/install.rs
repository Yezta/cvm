use crate::config::Config;
use crate::download::Downloader;
use crate::error::{JcvmError, Result};
use crate::models::{InstalledJdk, JdkDistribution, Version};
use colored::*;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;

pub struct Installer {
    config: Config,
    downloader: Downloader,
}

impl Installer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            downloader: Downloader::new(),
        }
    }

    /// Install a JDK version
    pub async fn install(&self, distribution: &JdkDistribution) -> Result<InstalledJdk> {
        let version_str = distribution.version.to_string();
        let install_dir = self.config.get_version_dir(&version_str);

        // Check if already installed
        if install_dir.exists() {
            return Err(JcvmError::VersionAlreadyInstalled(
                version_str,
                install_dir.display().to_string(),
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
        let url_path = distribution.download_url.split('/').last().unwrap_or("jdk.tar.gz");
        let cache_file = self.config.cache_dir.join(url_path);

        // Download if not cached
        if !cache_file.exists() || !self.config.cache_downloads {
            self.downloader
                .download_with_progress(&distribution.download_url, &cache_file)
                .await?;

            // Verify checksum if available
            if let Some(checksum) = &distribution.checksum {
                if self.config.verify_checksums {
                    println!("{}", "Verifying checksum...".yellow());
                    if !Downloader::verify_checksum(&cache_file, checksum).await? {
                        std::fs::remove_file(&cache_file)?;
                        return Err(JcvmError::ChecksumMismatch {
                            file: cache_file.display().to_string(),
                        });
                    }
                    println!("{}", "✓ Checksum verified".green());
                }
            }
        } else {
            println!("{}", "Using cached download".yellow());
        }

        // Extract archive
        println!("{}", "Extracting archive...".yellow());
        self.extract_archive(&cache_file, &install_dir)?;

        // Clean up if not caching
        if !self.config.cache_downloads {
            std::fs::remove_file(&cache_file)?;
        }

        println!(
            "{} JDK {} installed to {}",
            "✓".green().bold(),
            version_str.cyan(),
            install_dir.display().to_string().dimmed()
        );

        Ok(InstalledJdk {
            version: distribution.version.clone(),
            path: install_dir,
            installed_at: chrono::Utc::now(),
            distribution: "adoptium".to_string(),
        })
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

        std::fs::create_dir_all(dest_dir)?;

        let file_name = archive_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            self.extract_tar_gz(archive_path, dest_dir)?;
        } else if file_name.ends_with(".zip") {
            self.extract_zip(archive_path, dest_dir)?;
        } else {
            return Err(JcvmError::ExtractionFailed(
                "Unsupported archive format".to_string(),
            ));
        }

        pb.finish_with_message("Extraction complete");
        Ok(())
    }

    /// Extract tar.gz archive
    fn extract_tar_gz(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        // Find the root directory in the archive
        let entries = archive.entries()?;
        let _root_dir: Option<String> = None;

        for entry in entries {
            let entry = entry?;
            let path = entry.path()?;
            if let Some(first_component) = path.components().next() {
                if let Some(_component_str) = first_component.as_os_str().to_str() {
                    break;
                }
            }
        }

        // Re-open for extraction
        let tar_gz = File::open(archive_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        // Extract, stripping the root component
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            
            // Strip the first component (the root directory)
            let stripped_path: PathBuf = path
                .components()
                .skip(1)
                .collect();

            if !stripped_path.as_os_str().is_empty() {
                let dest_path = dest_dir.join(&stripped_path);
                entry.unpack(&dest_path)?;
            }
        }

        Ok(())
    }

    /// Extract zip archive
    fn extract_zip(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        let file = File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| JcvmError::ExtractionFailed(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| JcvmError::ExtractionFailed(e.to_string()))?;
            
            let outpath = match file.enclosed_name() {
                Some(path) => dest_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    /// Uninstall a JDK version
    pub fn uninstall(&self, version: &str) -> Result<()> {
        let version_dir = self.config.get_version_dir(version);

        if !version_dir.exists() {
            return Err(JcvmError::VersionNotFound(version.to_string()));
        }

        // Remove the directory
        std::fs::remove_dir_all(&version_dir)?;

        // Remove any aliases pointing to this version
        for entry in std::fs::read_dir(&self.config.alias_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_symlink() {
                if let Ok(target) = std::fs::read_link(&path) {
                    if target == version_dir {
                        std::fs::remove_file(&path)?;
                    }
                }
            }
        }

        println!(
            "{} JDK {} uninstalled",
            "✓".green().bold(),
            version.cyan()
        );

        Ok(())
    }

    /// List all installed JDKs
    pub fn list_installed(&self) -> Result<Vec<InstalledJdk>> {
        let mut installed = Vec::new();

        if !self.config.versions_dir.exists() {
            return Ok(installed);
        }

        for entry in std::fs::read_dir(&self.config.versions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(version_str) = path.file_name().and_then(|n| n.to_str()) {
                    if let Ok(version) = version_str.parse::<Version>() {
                        // Get metadata if available
                        let metadata = std::fs::metadata(&path)?;
                        let installed_at = metadata
                            .created()
                            .or_else(|_| metadata.modified())
                            .map(|t| chrono::DateTime::<chrono::Utc>::from(t))
                            .unwrap_or_else(|_| chrono::Utc::now());

                        installed.push(InstalledJdk {
                            version,
                            path: path.clone(),
                            installed_at,
                            distribution: "adoptium".to_string(),
                        });
                    }
                }
            }
        }

        // Sort by version (newest first)
        installed.sort_by(|a, b| b.version.major.cmp(&a.version.major));

        Ok(installed)
    }
}

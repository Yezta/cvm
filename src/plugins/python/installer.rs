use crate::core::traits::{ArchiveType, InstalledTool, Platform, ToolDistribution, ToolInstaller};
use crate::download::Downloader;
use crate::error::{JcvmError, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Python installer implementation
///
/// Handles downloading and installing Python distributions.
/// Supports:
/// - Pre-built binaries from python-build-standalone (fastest, recommended)
/// - macOS: .pkg installer packages
/// - Linux: Source compilation from .tar.xz (fallback)
/// - Windows: .exe installer
///
/// Includes checksum verification for security
pub struct PythonInstaller {
    cache_dir: PathBuf,
}

impl PythonInstaller {
    /// Creates a new Python installer
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Extracts a tar.gz archive (for standalone builds)
    async fn extract_tarball(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        println!("Extracting Python archive...");

        // Use tar command for extraction
        let status = Command::new("tar")
            .arg("xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(dest_dir)
            .arg("--strip-components=1") // Remove top-level directory
            .status()
            .map_err(|e| {
                JcvmError::ExtractionFailed(format!("Failed to extract archive: {}", e))
            })?;

        if !status.success() {
            return Err(JcvmError::ExtractionFailed(
                "Archive extraction failed".to_string(),
            ));
        }

        println!("✓ Extraction complete");
        Ok(())
    }

    /// Extracts a tar.xz archive and compiles Python from source (Linux fallback)
    async fn build_from_source(&self, archive_path: &Path, dest_dir: &Path) -> Result<()> {
        println!("Extracting Python source archive...");
        println!("⚠️  Building from source - this may take 10-20 minutes...");

        // Create temporary build directory
        let temp_build_dir = dest_dir
            .parent()
            .ok_or_else(|| JcvmError::InvalidToolStructure {
                tool: "python".to_string(),
                message: "Invalid destination directory".to_string(),
            })?
            .join("python-build-temp");

        std::fs::create_dir_all(&temp_build_dir)?;

        // Extract tar.xz
        let status = Command::new("tar")
            .arg("xJf") // J for xz compression
            .arg(archive_path)
            .arg("-C")
            .arg(&temp_build_dir)
            .status()
            .map_err(|e| {
                JcvmError::ExtractionFailed(format!("Failed to extract archive: {}", e))
            })?;

        if !status.success() {
            return Err(JcvmError::ExtractionFailed(
                "Archive extraction failed".to_string(),
            ));
        }

        // Find the extracted directory (Python-X.Y.Z)
        let source_dir = std::fs::read_dir(&temp_build_dir)?
            .next()
            .ok_or_else(|| JcvmError::ExtractionFailed("No source directory found".to_string()))??
            .path();

        println!("Configuring Python build...");
        println!("Build directory: {:?}", source_dir);

        // Configure with optimizations
        let configure_status = Command::new("./configure")
            .arg(format!("--prefix={}", dest_dir.display()))
            .arg("--enable-optimizations")
            .arg("--with-ensurepip=install") // Include pip
            .arg("--enable-shared") // Build shared library
            .current_dir(&source_dir)
            .status()
            .map_err(|e| JcvmError::ExtractionFailed(format!("Configure failed: {}", e)))?;

        if !configure_status.success() {
            let _ = std::fs::remove_dir_all(&temp_build_dir);
            return Err(JcvmError::ExtractionFailed(
                "Python configure failed. Ensure you have build dependencies installed (gcc, make, zlib-dev, etc.)".to_string(),
            ));
        }

        // Build with parallel compilation
        println!("Building Python (this may take a while)...");
        let num_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let make_status = Command::new("make")
            .arg("-j")
            .arg(num_cores.to_string())
            .current_dir(&source_dir)
            .status()
            .map_err(|e| JcvmError::ExtractionFailed(format!("Make failed: {}", e)))?;

        if !make_status.success() {
            let _ = std::fs::remove_dir_all(&temp_build_dir);
            return Err(JcvmError::ExtractionFailed(
                "Python build failed".to_string(),
            ));
        }

        // Install
        println!("Installing Python...");
        let install_status = Command::new("make")
            .arg("install")
            .current_dir(&source_dir)
            .status()
            .map_err(|e| JcvmError::ExtractionFailed(format!("Make install failed: {}", e)))?;

        if !install_status.success() {
            let _ = std::fs::remove_dir_all(&temp_build_dir);
            return Err(JcvmError::ExtractionFailed(
                "Python installation failed".to_string(),
            ));
        }

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_build_dir);
        println!("✓ Build and installation complete");

        Ok(())
    }

    /// Installs Python using the macOS .pkg installer
    async fn install_pkg(&self, pkg_path: &Path, dest_dir: &Path) -> Result<()> {
        println!("Installing Python .pkg package...");

        // Extract version from pkg filename (e.g., python-3.10.10-macos11.pkg)
        let version = pkg_path
            .file_name()
            .and_then(|f| f.to_str())
            .and_then(|name| {
                // Try to extract version from filename
                let parts: Vec<&str> = name.split('-').collect();
                if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                JcvmError::ExtractionFailed(
                    "Could not extract version from package filename".to_string(),
                )
            })?;

        let version_short = version.split('.').take(2).collect::<Vec<_>>().join(".");
        let system_python_path = PathBuf::from(format!(
            "/Library/Frameworks/Python.framework/Versions/{}",
            version_short
        ));

        // Check if Python is already installed at the system level
        let needs_install = !system_python_path.exists();

        if needs_install {
            // Install to system location (requires sudo)
            println!("Installing to system location (requires administrator privileges)...");
            let status = Command::new("sudo")
                .arg("installer")
                .arg("-pkg")
                .arg(pkg_path)
                .arg("-target")
                .arg("/")
                .status()
                .map_err(|e| {
                    JcvmError::ExtractionFailed(format!("PKG installation failed: {}", e))
                })?;

            if !status.success() {
                return Err(JcvmError::ExtractionFailed(
                    "Python PKG installation failed".to_string(),
                ));
            }

            println!("✓ System installation complete");
        } else {
            println!(
                "✓ Python {} already installed at system level",
                version_short
            );
        }

        // Copy the system Python installation to our managed directory
        // macOS .pkg installers install to /Library/Frameworks/Python.framework/Versions/X.Y/
        if system_python_path.exists() {
            println!(
                "Copying Python from {} to {}...",
                system_python_path.display(),
                dest_dir.display()
            );

            // Use cp -R to recursively copy the entire Python installation
            let copy_status = Command::new("cp")
                .arg("-R")
                .arg(&system_python_path)
                .arg(dest_dir)
                .status()
                .map_err(|e| {
                    JcvmError::ExtractionFailed(format!("Failed to copy Python files: {}", e))
                })?;

            if !copy_status.success() {
                return Err(JcvmError::ExtractionFailed(
                    "Failed to copy Python installation to JCVM directory".to_string(),
                ));
            }

            // The copied structure will be like: dest_dir/3.10/bin/python3
            // We need to move contents up one level to match expected structure
            let copied_version_dir = dest_dir.join(&version_short);
            if copied_version_dir.exists() {
                // Move contents from dest_dir/3.10/* to dest_dir/*
                for entry in std::fs::read_dir(&copied_version_dir)? {
                    let entry = entry?;
                    let dest_path = dest_dir.join(entry.file_name());
                    std::fs::rename(entry.path(), dest_path)?;
                }
                // Remove the now-empty version directory
                std::fs::remove_dir(&copied_version_dir)?;
            }

            println!("✓ Python copied to JCVM directory");
        } else {
            return Err(JcvmError::ExtractionFailed(format!(
                "Could not find Python installation at {} after PKG installation",
                system_python_path.display()
            )));
        }

        Ok(())
    }

    /// Installs Python using Windows .exe installer
    async fn install_exe(&self, exe_path: &Path, dest_dir: &Path) -> Result<()> {
        println!("Installing Python .exe package...");

        // Run the installer with custom installation directory
        let status = Command::new(exe_path)
            .arg("/quiet")
            .arg(format!("TargetDir={}", dest_dir.display()))
            .arg("InstallAllUsers=0")
            .arg("PrependPath=0") // Don't modify PATH
            .arg("Include_test=0")
            .status()
            .map_err(|e| JcvmError::ExtractionFailed(format!("EXE installation failed: {}", e)))?;

        if !status.success() {
            return Err(JcvmError::ExtractionFailed(
                "Python EXE installation failed".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl ToolInstaller for PythonInstaller {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        let version_str = distribution.version.to_string();

        // Check if already installed
        if dest_dir.exists() {
            return Err(JcvmError::VersionAlreadyInstalled(
                version_str.clone(),
                dest_dir.display().to_string(),
            ));
        }

        println!(
            "Installing Python {} for {}-{}",
            version_str,
            distribution.platform.to_string(),
            distribution.architecture.to_string()
        );

        // Determine cache file name
        let url_path = distribution
            .download_url
            .split('/')
            .last()
            .unwrap_or("python.tar.gz");
        let cache_file = self.cache_dir.join(url_path);

        // Download if not cached
        if !cache_file.exists() {
            println!("Downloading from {}...", distribution.download_url);
            std::fs::create_dir_all(&self.cache_dir)?;

            let downloader = Downloader::new();
            downloader
                .download_with_progress(&distribution.download_url, &cache_file)
                .await?;
        } else {
            println!("Using cached download: {}", cache_file.display());
        }

        // Verify checksum if available
        if let Some(ref checksum) = distribution.checksum {
            println!("Verifying checksum...");
            let is_valid = Downloader::verify_checksum(&cache_file, checksum).await?;

            if !is_valid {
                std::fs::remove_file(&cache_file)?;
                return Err(JcvmError::ChecksumMismatch {
                    file: cache_file.display().to_string(),
                });
            }
            println!("✓ Checksum verified");
        } else {
            println!("⚠️  No checksum available - skipping verification");
        }

        // Create destination directory
        std::fs::create_dir_all(&dest_dir)?;

        // Install based on archive type
        match distribution.archive_type {
            ArchiveType::TarGz => {
                // Check if this is a standalone build or source build
                if distribution
                    .metadata
                    .get("source")
                    .map(|s| s == "python-build-standalone")
                    .unwrap_or(false)
                {
                    // Extract standalone build (pre-built binary)
                    self.extract_tarball(&cache_file, &dest_dir).await?;
                } else {
                    // Build from source (python.org tar.xz)
                    self.build_from_source(&cache_file, &dest_dir).await?;
                }
            }
            ArchiveType::Pkg => {
                self.install_pkg(&cache_file, &dest_dir).await?;
            }
            ArchiveType::Exe => {
                self.install_exe(&cache_file, &dest_dir).await?;
            }
            _ => {
                return Err(JcvmError::UnsupportedPlatform {
                    os: format!("{:?} archive", distribution.archive_type),
                    arch: "N/A".to_string(),
                });
            }
        }

        println!(
            "✓ Python {} installed to {}",
            version_str,
            dest_dir.display()
        );

        // Determine executable path
        let executable_path = self.get_python_executable(&dest_dir, &distribution.platform);

        // Verify installation
        if !executable_path.exists() {
            return Err(JcvmError::InvalidToolStructure {
                tool: "python".to_string(),
                message: format!(
                    "Installation completed but Python executable not found at {:?}",
                    executable_path
                ),
            });
        }

        // Create 'python' symlink pointing to 'python3' for convenience
        // This allows users to use 'python' command instead of just 'python3'
        if !cfg!(windows) {
            let bin_dir = dest_dir.join("bin");
            let python_link = bin_dir.join("python");
            let python3_target = bin_dir.join("python3");

            if python3_target.exists() && !python_link.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::symlink;
                    if let Err(e) = symlink("python3", &python_link) {
                        println!("⚠️  Warning: Could not create 'python' symlink: {}", e);
                    } else {
                        println!("✓ Created 'python' symlink to 'python3'");
                    }
                }
            }
        }

        Ok(InstalledTool {
            tool_id: "python".to_string(),
            version: distribution.version.clone(),
            path: dest_dir.clone(),
            installed_at: chrono::Utc::now(),
            source: distribution
                .metadata
                .get("source")
                .cloned()
                .unwrap_or_else(|| "python.org".to_string()),
            executable_path: Some(executable_path),
        })
    }

    async fn uninstall(&self, installed: &InstalledTool) -> Result<()> {
        if !installed.path.exists() {
            return Err(JcvmError::ToolNotFound(format!(
                "Python {} is not installed at {}",
                installed.version,
                installed.path.display()
            )));
        }

        std::fs::remove_dir_all(&installed.path)?;
        println!("Successfully uninstalled Python {}", installed.version);

        Ok(())
    }

    async fn verify(&self, installed: &InstalledTool) -> Result<bool> {
        let python_exe = if cfg!(windows) {
            installed.path.join("python.exe")
        } else {
            installed.path.join("bin").join("python3")
        };

        if !python_exe.exists() {
            return Ok(false);
        }

        // Verify Python can run
        match Command::new(&python_exe).arg("--version").output() {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }
}

impl PythonInstaller {
    /// Gets the Python executable path for a given installation directory
    fn get_python_executable(&self, install_dir: &Path, platform: &Platform) -> PathBuf {
        match platform {
            Platform::Windows => install_dir.join("python.exe"),
            _ => {
                // On Unix-like systems, Python installs to bin/python3
                let bin_dir = install_dir.join("bin");
                bin_dir.join("python3")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_python_executable_unix() {
        let installer = PythonInstaller::new(PathBuf::from("/opt/jcvm/versions"));
        let install_dir = PathBuf::from("/opt/jcvm/versions/3.12.8");
        let exe = installer.get_python_executable(&install_dir, &Platform::Linux);

        assert_eq!(exe, PathBuf::from("/opt/jcvm/versions/3.12.8/bin/python3"));
    }

    #[test]
    fn test_get_python_executable_windows() {
        let installer = PythonInstaller::new(PathBuf::from("C:\\jcvm\\versions"));
        let install_dir = PathBuf::from("C:\\jcvm\\versions\\3.12.8");
        let exe = installer.get_python_executable(&install_dir, &Platform::Windows);

        assert_eq!(exe, install_dir.join("python.exe"));
    }
}

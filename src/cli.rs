use crate::api::AdoptiumApi;
use crate::config::Config;
use crate::detect::JavaDetector;
use crate::error::Result;
use crate::install::Installer;
use crate::models::{Architecture, Platform, Version};
use crate::shell::{generate_activation_script, Shell};
use crate::utils::{confirm, format_size, print_error, print_info, print_success, print_warning};
use crate::version_manager::VersionManager;
use clap::{Parser, Subcommand};
use colored::*;

#[derive(Parser)]
#[command(name = "jcvm")]
#[command(about = "Java Configuration & Version Manager", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(skip)]
    config: Config,
}

#[derive(Subcommand)]
enum Commands {
    /// List available JDK versions from remote
    #[command(alias = "ls-remote")]
    ListRemote {
        /// Show only LTS versions
        #[arg(long)]
        lts: bool,
    },

    /// Install a JDK version
    Install {
        /// Version to install (e.g., 21, 17.0.10)
        version: String,

        /// Force reinstall if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// List installed JDK versions
    #[command(alias = "ls")]
    List,

    /// Use a specific JDK version
    Use {
        /// Version to use
        version: String,
    },

    /// Show currently active JDK version
    Current,

    /// Set JDK version for current directory
    Local {
        /// Version to set (omit to show current)
        version: Option<String>,
    },

    /// Create or show aliases
    Alias {
        /// Alias name (e.g., default, latest)
        name: Option<String>,

        /// Version to alias
        version: Option<String>,
    },

    /// Uninstall a JDK version
    Uninstall {
        /// Version to uninstall
        version: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Install shell integration
    ShellInit {
        /// Shell type (bash, zsh, fish, powershell)
        #[arg(short, long)]
        shell: Option<String>,
    },

    /// Show which version would be used
    Which,

    /// Clean up cache and old downloads
    Clean {
        /// Remove all cached downloads
        #[arg(long)]
        all: bool,
    },

    /// Show JCVM configuration
    Config {
        /// Show specific config key
        key: Option<String>,
    },

    /// Run a command with a specific JDK version
    Exec {
        /// Version to use
        #[arg(short, long)]
        version: String,

        /// Command and arguments to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Detect existing Java installations on the system
    Detect {
        /// Automatically import all detected installations
        #[arg(short, long)]
        import: bool,
    },

    /// Import an existing Java installation into JCVM
    Import {
        /// Path to the Java installation (JAVA_HOME)
        path: String,
    },
}

impl Cli {
    pub fn new(config: Config) -> Self {
        let mut cli = Self::parse();
        cli.config = config;
        cli
    }

    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::ListRemote { lts } => self.list_remote(lts).await,
            Commands::Install { ref version, force } => self.install(version, force).await,
            Commands::List => self.list(),
            Commands::Use { ref version } => self.use_version(version),
            Commands::Current => self.current(),
            Commands::Local { ref version } => self.local(version.clone()),
            Commands::Alias { ref name, ref version } => self.alias(name.clone(), version.clone()),
            Commands::Uninstall { ref version, yes } => self.uninstall(version, yes),
            Commands::ShellInit { ref shell } => self.shell_init(shell.clone()),
            Commands::Which => self.which(),
            Commands::Clean { all } => self.clean(all),
            Commands::Config { ref key } => self.show_config(key.clone()),
            Commands::Exec { ref version, ref command } => self.exec(version, command.clone()).await,
            Commands::Detect { import } => self.detect(import),
            Commands::Import { ref path } => self.import(path),
        }
    }

    async fn list_remote(&self, lts_only: bool) -> Result<()> {
        let api = AdoptiumApi::new();

        print_info("Fetching available JDK versions...");

        let versions = if lts_only {
            api.list_lts_versions().await?
        } else {
            api.list_available_versions().await?
        };

        let lts_versions = api.list_lts_versions().await?;

        println!("\n{}", "Available JDK Versions:".green().bold());
        
        for version in versions.iter().rev() {
            let is_lts = lts_versions.contains(version);
            let lts_marker = if is_lts && self.config.show_lts_indicator {
                format!(" {}", "(LTS)".green())
            } else {
                String::new()
            };
            
            println!("  {}{}", version.to_string().cyan(), lts_marker);
        }

        println!("\n{}", "Usage:".yellow());
        println!("  jcvm install <version>");
        println!("  jcvm install 21");

        Ok(())
    }

    async fn install(&self, version_str: &str, force: bool) -> Result<()> {
        let version: Version = version_str.parse()?;
        let platform = Platform::current()?;
        let arch = Architecture::current()?;

        let api = AdoptiumApi::new();
        let distribution = api.find_distribution(&version, platform, arch).await?;

        let installer = Installer::new(self.config.clone());

        // Check if already installed
        let version_dir = self.config.get_version_dir(&version.to_string());
        if version_dir.exists() && !force {
            print_warning(&format!("JDK {} is already installed", version));
            
            if confirm("Reinstall?") {
                installer.uninstall(&version.to_string())?;
            } else {
                return Ok(());
            }
        } else if version_dir.exists() && force {
            installer.uninstall(&version.to_string())?;
        }

        // Show download info
        if let Some(size) = distribution.size {
            print_info(&format!("Download size: {}", format_size(size)));
        }

        let installed = installer.install(&distribution).await?;

        print_success(&format!(
            "JDK {} installed successfully",
            installed.version
        ));
        println!("  Path: {}", installed.path.display().to_string().dimmed());
        println!("\n{}", "Next steps:".yellow());
        println!("  jcvm use {}    # Activate this version", version);
        println!("  jcvm alias default {}    # Set as default", version);

        Ok(())
    }

    fn list(&self) -> Result<()> {
        let installer = Installer::new(self.config.clone());
        let installed = installer.list_installed()?;

        if installed.is_empty() {
            print_warning("No JDK versions installed");
            println!("\n{}", "Install a version:".yellow());
            println!("  jcvm install 21");
            return Ok(());
        }

        let manager = VersionManager::new(self.config.clone());
        let current = manager.get_current()?;
        let default = manager.get_default()?;

        println!("{}", "Installed JDK Versions:".green().bold());

        for jdk in installed {
            let version_str = jdk.version.to_string();
            let mut markers = Vec::new();

            if Some(&version_str) == current.as_ref() {
                markers.push("current".green().to_string());
            }

            if Some(&version_str) == default.as_ref() {
                markers.push("default".blue().to_string());
            }

            if jdk.version.is_lts() && self.config.show_lts_indicator {
                markers.push("LTS".yellow().to_string());
            }

            let marker_str = if markers.is_empty() {
                String::new()
            } else {
                format!(" ({})", markers.join(", "))
            };

            let prefix = if Some(&version_str) == current.as_ref() {
                "→".green().bold()
            } else {
                " ".normal()
            };

            println!("  {} {}{}", prefix, version_str.cyan(), marker_str);
        }

        if current.is_none() {
            println!("\n{}", "Activate a version:".yellow());
            println!("  jcvm use <version>");
        }

        Ok(())
    }

    fn use_version(&self, version: &str) -> Result<()> {
        let manager = VersionManager::new(self.config.clone());
        let version_path = manager.set_current(version)?;

        print_success(&format!("Now using JDK {}", version));

        // Check Java version
        let java_bin = version_path.join("bin").join("java");
        if java_bin.exists() {
            if let Ok(output) = std::process::Command::new(&java_bin)
                .arg("-version")
                .output()
            {
                let version_info = String::from_utf8_lossy(&output.stderr);
                if let Some(first_line) = version_info.lines().next() {
                    println!("{}", first_line.dimmed());
                }
            }
        }

        // Check if shell integration is installed
        let shell = Shell::detect();
        let has_shell_integration = if let Some(shell) = shell {
            if let Some(config_file) = shell.config_file() {
                if config_file.exists() {
                    std::fs::read_to_string(&config_file)
                        .map(|contents| contents.contains("JCVM_DIR"))
                        .unwrap_or(false)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if !has_shell_integration {
            println!("\n{}", "⚠️  Shell integration not installed".yellow().bold());
            println!("{}", "To activate in your current shell, run:".yellow());
            let activation = generate_activation_script(version_path.to_str().unwrap())?;
            println!("{}", activation.dimmed());
            println!("\n{}", "For automatic activation in new shells, run:".yellow());
            println!("  {}", "jcvm shell-init".cyan());
        }

        Ok(())
    }

    fn current(&self) -> Result<()> {
        let manager = VersionManager::new(self.config.clone());

        if let Some(version) = manager.get_current()? {
            println!("{} {}", "Current JDK:".green().bold(), version.cyan());

            let version_path = self.config.get_version_dir(&version);
            let java_bin = version_path.join("bin").join("java");
            
            if java_bin.exists() {
                if let Ok(output) = std::process::Command::new(&java_bin)
                    .arg("-version")
                    .output()
                {
                    let version_info = String::from_utf8_lossy(&output.stderr);
                    println!("{}", version_info.dimmed());
                }
            }
        } else {
            print_warning("No JCVM version currently active");
            
            // Check system Java
            if let Ok(output) = std::process::Command::new("java")
                .arg("-version")
                .output()
            {
                println!("\n{}", "System Java:".yellow());
                let version_info = String::from_utf8_lossy(&output.stderr);
                println!("{}", version_info.dimmed());
            }
        }

        Ok(())
    }

    fn local(&self, version: Option<String>) -> Result<()> {
        if let Some(version_str) = version {
            let version: Version = version_str.parse()?;
            
            // Check if installed
            let version_dir = self.config.get_version_dir(&version.to_string());
            if !version_dir.exists() {
                print_error(&format!("JDK {} is not installed", version));
                println!("\n{}", "Install it first:".yellow());
                println!("  jcvm install {}", version);
                return Ok(());
            }

            VersionManager::write_local_version(&version)?;
            print_success(&format!("Set local JDK version to {}", version));
            println!("  Created .java-version file");

            // Also activate it
            let manager = VersionManager::new(self.config.clone());
            manager.set_current(&version.to_string())?;
            println!("\n{}", "Version activated for current shell".green());
        } else {
            // Show current local version
            if let Some(version) = VersionManager::read_local_version()? {
                println!("{} {}", "Local JDK version:".green().bold(), version.to_string().cyan());
            } else {
                print_warning("No .java-version file in current directory");
                println!("\n{}", "Set local version:".yellow());
                println!("  jcvm local <version>");
            }
        }

        Ok(())
    }

    fn alias(&self, name: Option<String>, version: Option<String>) -> Result<()> {
        let manager = VersionManager::new(self.config.clone());

        match (name, version) {
            (Some(name), Some(version)) => {
                // Set alias
                manager.set_alias(&name, &version)?;
                print_success(&format!("Set alias '{}' to JDK {}", name, version));

                if name == "default" {
                    println!("\n{}", "The default version will be used on shell startup".dimmed());
                }
            }
            (Some(name), None) => {
                // Show specific alias
                if let Some(version) = manager.get_alias(&name)? {
                    println!("{} → {}", name.cyan(), version.green());
                } else {
                    print_warning(&format!("Alias '{}' is not set", name));
                }
            }
            _ => {
                // List all aliases
                println!("{}", "Aliases:".green().bold());

                for entry in std::fs::read_dir(&self.config.alias_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_symlink() {
                        if let Ok(target) = std::fs::read_link(&path) {
                            if let (Some(alias_name), Some(version)) = (
                                path.file_name().and_then(|n| n.to_str()),
                                target.file_name().and_then(|n| n.to_str()),
                            ) {
                                if alias_name != "current" {
                                    println!("  {} → {}", alias_name.cyan(), version.green());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn uninstall(&self, version: &str, skip_confirm: bool) -> Result<()> {
        let manager = VersionManager::new(self.config.clone());
        
        // Check if it's the current version
        if let Some(current) = manager.get_current()? {
            if current == version {
                print_warning(&format!("JDK {} is currently active", version));
            }
        }

        let should_uninstall = skip_confirm || confirm(&format!("Uninstall JDK {}?", version));

        if should_uninstall {
            let installer = Installer::new(self.config.clone());
            installer.uninstall(version)?;
        } else {
            print_info("Uninstall cancelled");
        }

        Ok(())
    }

    fn shell_init(&self, shell_type: Option<String>) -> Result<()> {
        // Check if this is the first run (no versions directory or empty)
        let is_first_run = !self.config.versions_dir.exists() 
            || std::fs::read_dir(&self.config.versions_dir)
                .map(|mut d| d.next().is_none())
                .unwrap_or(true);

        if is_first_run {
            println!("{}", "First time setup detected!".green().bold());
            print_info("Checking for existing Java installations...");
            
            let detector = JavaDetector::new(self.config.clone());
            if let Ok(detected) = detector.detect_all() {
                if !detected.is_empty() {
                    println!("\n{}", format!("Found {} Java installation(s):", detected.len()).green().bold());
                    
                    for (i, java) in detected.iter().enumerate() {
                        println!("  {}. JDK {} ({}) at {}", 
                            i + 1, 
                            java.version.to_string().cyan(),
                            java.source.yellow(),
                            java.path.display().to_string().dimmed()
                        );
                    }

                    println!();
                    if confirm("Import these installations into JCVM?") {
                        let mut imported_count = 0;
                        
                        for java in &detected {
                            match detector.import(java) {
                                Ok(_) => imported_count += 1,
                                Err(_) => {} // Silently skip duplicates
                            }
                        }

                        if imported_count > 0 {
                            print_success(&format!("Imported {} Java installation(s)", imported_count));
                            
                            // Set the first one as default if no default exists
                            if let Some(first) = detected.first() {
                                let manager = VersionManager::new(self.config.clone());
                                if manager.get_default()?.is_none() {
                                    let version_str = first.version.to_string();
                                    if let Err(_) = manager.set_alias("default", &version_str) {
                                        // Ignore error if already set
                                    } else {
                                        print_info(&format!("Set JDK {} as default", version_str));
                                    }
                                }
                            }
                        }
                    } else {
                        print_info("Skipped import. You can run 'jcvm detect' later to import them.");
                    }
                }
            }
            println!();
        }

        let shell = if let Some(shell_str) = shell_type {
            match shell_str.to_lowercase().as_str() {
                "bash" => Shell::Bash,
                "zsh" => Shell::Zsh,
                "fish" => Shell::Fish,
                "powershell" | "pwsh" => Shell::PowerShell,
                _ => {
                    print_error(&format!("Unsupported shell: {}", shell_str));
                    return Ok(());
                }
            }
        } else {
            Shell::detect().unwrap_or(Shell::Bash)
        };

        println!("{}", "Installing shell integration...".yellow());
        
        match shell.install_hook(&self.config) {
            Ok(_) => {
                print_success("Shell integration installed");
                
                if let Some(config_file) = shell.config_file() {
                    println!("  Updated: {}", config_file.display());
                    println!("\n{}", "Reload your shell:".yellow());
                    
                    match shell {
                        Shell::Bash => println!("  source ~/.bashrc"),
                        Shell::Zsh => println!("  source ~/.zshrc"),
                        Shell::Fish => println!("  source ~/.config/fish/config.fish"),
                        Shell::PowerShell => println!("  . $PROFILE"),
                    }
                }
            }
            Err(e) => {
                print_error(&format!("Failed to install shell integration: {}", e));
            }
        }

        Ok(())
    }

    fn which(&self) -> Result<()> {
        // Check local version first
        if let Some(local_version) = VersionManager::read_local_version()? {
            println!("{} {} (from .java-version)", 
                "Would use:".green().bold(), 
                local_version.to_string().cyan()
            );
            return Ok(());
        }

        // Check current version
        let manager = VersionManager::new(self.config.clone());
        if let Some(current) = manager.get_current()? {
            println!("{} {} (current)", 
                "Would use:".green().bold(), 
                current.cyan()
            );
            return Ok(());
        }

        // Check default version
        if let Some(default) = manager.get_default()? {
            println!("{} {} (default)", 
                "Would use:".green().bold(), 
                default.cyan()
            );
            return Ok(());
        }

        print_warning("No version configured");
        Ok(())
    }

    fn clean(&self, all: bool) -> Result<()> {
        let cache_dir = &self.config.cache_dir;

        if !cache_dir.exists() {
            print_info("Cache is already empty");
            return Ok(());
        }

        let entries: Vec<_> = std::fs::read_dir(cache_dir)?
            .filter_map(|e| e.ok())
            .collect();

        if entries.is_empty() {
            print_info("Cache is already empty");
            return Ok(());
        }

        let total_size: u64 = entries
            .iter()
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();

        println!("{} {} in cache ({} files)", 
            "Found:".yellow(), 
            format_size(total_size), 
            entries.len()
        );

        let should_clean = all || confirm("Remove all cached downloads?");

        if should_clean {
            for entry in entries {
                std::fs::remove_file(entry.path())?;
            }
            print_success(&format!("Cleaned {} from cache", format_size(total_size)));
        } else {
            print_info("Clean cancelled");
        }

        Ok(())
    }

    fn show_config(&self, key: Option<String>) -> Result<()> {
        if let Some(key_name) = key {
            match key_name.as_str() {
                "dir" | "jcvm_dir" => println!("{}", self.config.jcvm_dir.display()),
                "versions_dir" => println!("{}", self.config.versions_dir.display()),
                "cache_dir" => println!("{}", self.config.cache_dir.display()),
                "verify_checksums" => println!("{}", self.config.verify_checksums),
                "cache_downloads" => println!("{}", self.config.cache_downloads),
                _ => print_warning(&format!("Unknown config key: {}", key_name)),
            }
        } else {
            println!("{}", "JCVM Configuration:".green().bold());
            println!("  {} {}", "JCVM Directory:".cyan(), self.config.jcvm_dir.display());
            println!("  {} {}", "Versions Directory:".cyan(), self.config.versions_dir.display());
            println!("  {} {}", "Cache Directory:".cyan(), self.config.cache_dir.display());
            println!("  {} {}", "Verify Checksums:".cyan(), self.config.verify_checksums);
            println!("  {} {}", "Cache Downloads:".cyan(), self.config.cache_downloads);
            println!("  {} {} days", "Cache Retention:".cyan(), self.config.cache_retention_days);
        }

        Ok(())
    }

    async fn exec(&self, version: &str, command: Vec<String>) -> Result<()> {
        if command.is_empty() {
            print_error("No command specified");
            return Ok(());
        }

        let version_dir = self.config.get_version_dir(version);
        if !version_dir.exists() {
            print_error(&format!("JDK {} is not installed", version));
            return Ok(());
        }

        let java_home = version_dir.to_str().unwrap();
        let bin_dir = version_dir.join("bin");

        let mut cmd = std::process::Command::new(&command[0]);
        cmd.args(&command[1..]);
        cmd.env("JAVA_HOME", java_home);
        
        // Add JDK bin to PATH
        let path = std::env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", bin_dir.display(), path);
        cmd.env("PATH", new_path);

        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(1));
    }

    fn detect(&self, auto_import: bool) -> Result<()> {
        print_info("Detecting Java installations on the system...");
        
        let detector = JavaDetector::new(self.config.clone());
        let detected = detector.detect_all()?;

        if detected.is_empty() {
            print_warning("No Java installations detected");
            println!("\n{}", "Install a JDK version:".yellow());
            println!("  jcvm install 21");
            return Ok(());
        }

        println!("\n{}", format!("Found {} Java installation(s):", detected.len()).green().bold());
        
        for (i, java) in detected.iter().enumerate() {
            println!("\n{}. {}", i + 1, java.display_info());
            println!("   {}", java.raw_version.dimmed());
        }

        if auto_import {
            println!("\n{}", "Importing all detected installations...".yellow());
            
            let mut imported_count = 0;
            let mut skipped_count = 0;

            for java in &detected {
                match detector.import(java) {
                    Ok(_) => imported_count += 1,
                    Err(e) => {
                        print_warning(&format!("Skipped {}: {}", java.version, e));
                        skipped_count += 1;
                    }
                }
            }

            println!("\n{}", "Import Summary:".green().bold());
            println!("  {} imported", imported_count);
            if skipped_count > 0 {
                println!("  {} skipped (already managed)", skipped_count);
            }
        } else {
            println!("\n{}", "To import these installations:".yellow());
            println!("  jcvm detect --import          # Import all");
            println!("  jcvm import <path>            # Import specific installation");
        }

        Ok(())
    }

    fn import(&self, path: &str) -> Result<()> {
        use std::path::PathBuf;
        
        let java_home = PathBuf::from(shellexpand::tilde(path).to_string());
        
        if !java_home.exists() {
            print_error(&format!("Path does not exist: {}", path));
            return Ok(());
        }

        let detector = JavaDetector::new(self.config.clone());
        
        // Verify it's a valid Java installation
        let detected = detector.detect_all()?;
        let found = detected.iter().find(|j| j.path == java_home);

        if let Some(java) = found {
            match detector.import(java) {
                Ok(_) => {
                    println!("\n{}", "Next steps:".yellow());
                    println!("  jcvm use {}              # Activate this version", java.version);
                    println!("  jcvm alias default {}    # Set as default", java.version);
                }
                Err(e) => {
                    print_error(&format!("Failed to import: {}", e));
                }
            }
        } else {
            print_error("Not a valid Java installation or could not detect version");
            println!("\n{}", "The path should point to JAVA_HOME (contains bin/java)".dimmed());
        }

        Ok(())
    }
}

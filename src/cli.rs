use crate::config::Config;
use crate::core::plugin::PluginRegistry;
use crate::detect::JavaDetector;
use crate::error::{JcvmError, Result};
use crate::plugins;
use crate::shell::{generate_activation_script, Shell};
use crate::tool_manager::{ActivationContext, ManagedInstallation, ToolManager};
use crate::utils::{confirm, format_size, print_error, print_info, print_success, print_warning};
use crate::version_manager::VersionManager;
use clap::{Parser, Subcommand};
use colored::*;
use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Parser)]
#[command(name = "jcvm")]
#[command(
    about = "Universal Development Tool Version Manager",
    long_about = "Manage multiple development tools (Java, Node.js, Python, ...) with ease.\n\
                 Switch versions seamlessly, manage installations, and extend with plugins."
)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(skip)]
    config: Config,

    #[arg(skip)]
    registry: PluginRegistry,

    #[arg(skip)]
    tool_manager: ToolManager,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_target_defaults_to_java() {
        let (tool, version) = Cli::parse_tool_target("21").expect("parse");
        assert_eq!(tool, "java");
        assert_eq!(version, "21");
    }

    #[test]
    fn parse_target_respects_explicit_tool() {
        let (tool, version) = Cli::parse_tool_target("node@20.10.0").expect("parse");
        assert_eq!(tool, "node");
        assert_eq!(version, "20.10.0");
    }

    #[test]
    fn parse_target_rejects_missing_version() {
        let err = Cli::parse_tool_target("node@").unwrap_err();
        assert!(matches!(err, JcvmError::InvalidVersion(_)));
    }
}

#[derive(Subcommand)]
enum Commands {
    /// List available versions for a tool from remote (Java, Node.js, Python, ...)
    #[command(alias = "ls-remote")]
    ListRemote {
        /// Tool to list versions for (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Show only LTS/stable versions (when applicable)
        #[arg(long)]
        lts: bool,
    },

    /// Install a tool version (Java, Node.js, Python, ...)
    Install {
        /// Tool to install (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Version to install (e.g., 21, 17.0.10, 20.10.0)
        version: String,

        /// Force reinstall if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// List installed versions for a tool (Java, Node.js, Python, ...)
    #[command(alias = "ls")]
    List {
        /// Tool to list versions for (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Show installations for all tools
        #[arg(long)]
        all: bool,
    },

    /// Use a specific tool version
    Use {
        /// Tool to activate (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Version to use
        version: String,
    },

    /// Show currently active version for a tool
    Current {
        /// Tool to check (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Show active version for all tools
        #[arg(long)]
        all: bool,
    },

    /// Set tool version for current directory
    Local {
        /// Tool to configure (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Version to set (omit to show current)
        version: Option<String>,
    },

    /// Create or show aliases for a tool
    Alias {
        /// Tool to manage aliases for (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

        /// Alias name (e.g., default, latest)
        name: Option<String>,

        /// Version to alias
        version: Option<String>,
    },

    /// Uninstall a tool version
    Uninstall {
        /// Tool to uninstall from (defaults to 'java' for backward compatibility)
        #[arg(short, long, default_value = "java")]
        tool: String,

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
        /// Tool to detect (defaults to all tools)
        #[arg(short, long)]
        tool: Option<String>,

        /// Automatically import all detected installations
        #[arg(short, long)]
        import: bool,
    },

    /// Import an existing Java installation into JCVM
    Import {
        /// Path to the Java installation (JAVA_HOME)
        path: String,
    },

    /// Manage tool plugins (Java, Node.js, Python, ...)
    Tool {
        #[command(subcommand)]
        action: ToolCommands,
    },

    /// Quickly switch tool versions using shorthand like java@21 or node@20
    Switch {
        /// Target in the form <tool>@<version> (e.g., java@21, node@20.10.0)
        target: String,

        /// Install the version automatically if missing before switching
        #[arg(long)]
        install: bool,
    },
}

#[derive(Subcommand)]
enum ToolCommands {
    /// List installed versions
    List {
        /// Tool ID (java, node, python). Defaults to java unless --all is set.
        #[arg(short, long)]
        tool: Option<String>,

        /// Show installations for every tool
        #[arg(long)]
        all: bool,
    },

    /// Install a tool version via its plugin
    Install {
        /// Tool ID (java, node, python)
        tool: String,

        /// Version string to install
        version: String,

        /// Force reinstall if the version already exists
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall a tool version
    Uninstall {
        /// Tool ID
        tool: String,

        /// Version string to uninstall
        version: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
    },

    /// Show remote versions available for a tool
    Remote {
        /// Tool ID
        tool: String,

        /// Show only LTS/stable releases when available
        #[arg(long)]
        lts: bool,
    },

    /// Activate a specific tool version without shorthand parsing
    Use {
        /// Tool ID
        tool: String,

        /// Version string to activate
        version: String,
    },

    /// Show the active version for one or all tools
    Current {
        /// Tool ID (defaults to java unless --all is provided)
        #[arg(short, long)]
        tool: Option<String>,

        /// Show the active version for every tool
        #[arg(long)]
        all: bool,
    },

    /// Manage tool aliases (default, production, etc.)
    Alias {
        /// Tool ID
        tool: String,

        /// Alias name
        alias: String,

        /// Target version to set. If omitted, prints the current alias target.
        #[arg(long)]
        version: Option<String>,

        /// Remove the alias instead of setting a target
        #[arg(long)]
        unset: bool,
    },
}

impl Cli {
    pub fn new(config: Config) -> Result<Self> {
        let mut cli = Self::parse();
        let registry = plugins::load_builtin_plugins(&config)?;
        let tool_manager = ToolManager::new(config.clone(), registry.clone());
        cli.config = config;
        cli.registry = registry;
        cli.tool_manager = tool_manager;
        Ok(cli)
    }

    /// Get a formatted list of supported tools with their display names
    fn supported_tools_list(&self) -> String {
        match self.registry.list_plugins() {
            Ok(plugins) => {
                let tool_names: Vec<String> = plugins
                    .iter()
                    .filter_map(|id| {
                        self.tool_manager
                            .metadata(id)
                            .ok()
                            .map(|meta| format!("{} ({})", meta.display_name(), id))
                    })
                    .collect();

                if tool_names.is_empty() {
                    "No tools available".to_string()
                } else {
                    tool_names.join(", ")
                }
            }
            Err(_) => "Unable to list tools".to_string(),
        }
    }

    /// Get a formatted list of supported tools for error messages
    fn supported_tools_message(&self) -> String {
        let supported = self.supported_tools_list();
        format!("Supported tools: {}", supported)
    }

    /// Get tool metadata with a user-friendly error message
    fn get_tool_metadata(&self, tool_id: &str) -> Result<crate::core::traits::PluginMetadata> {
        self.tool_manager.metadata(tool_id).map_err(|_| {
            let supported = self.supported_tools_message();
            JcvmError::PluginError {
                plugin: tool_id.to_string(),
                message: format!("Tool not found. {}", supported),
            }
        })
    }

    pub async fn run(self) -> Result<()> {
        match self.command {
            Commands::ListRemote { ref tool, lts } => self.list_remote(tool, lts).await,
            Commands::Install {
                ref tool,
                ref version,
                force,
            } => self.install(tool, version, force).await,
            Commands::List { ref tool, all } => self.list(tool, all),
            Commands::Use {
                ref tool,
                ref version,
            } => self.use_version(tool, version).await,
            Commands::Current { ref tool, all } => self.current(tool, all),
            Commands::Local {
                ref tool,
                ref version,
            } => self.local(tool, version.clone()).await,
            Commands::Alias {
                ref tool,
                ref name,
                ref version,
            } => self.alias(tool, name.clone(), version.clone()),
            Commands::Uninstall {
                ref tool,
                ref version,
                yes,
            } => self.uninstall(tool, version, yes).await,
            Commands::ShellInit { ref shell } => self.shell_init(shell.clone()),
            Commands::Which => self.which(),
            Commands::Clean { all } => self.clean(all),
            Commands::Config { ref key } => self.show_config(key.clone()),
            Commands::Exec {
                ref version,
                ref command,
            } => self.exec(version, command.clone()).await,
            Commands::Detect { ref tool, import } => self.detect(tool.clone(), import).await,
            Commands::Import { ref path } => self.import(path),
            Commands::Tool { ref action } => self.handle_tool(action).await,
            Commands::Switch {
                ref target,
                install,
            } => self.quick_switch(target, install).await,
        }
    }

    async fn list_remote(&self, tool_id: &str, lts_only: bool) -> Result<()> {
        // Get tool metadata for display
        let metadata = self.get_tool_metadata(tool_id)?;

        print_info(&format!(
            "Fetching available {} versions...",
            metadata.display_name()
        ));

        let versions = self
            .tool_manager
            .list_remote_versions(tool_id, lts_only)
            .await?;

        if versions.is_empty() {
            print_warning(&format!(
                "No versions found for {}",
                metadata.display_name()
            ));
            return Ok(());
        }

        println!(
            "\n{}",
            format!("Available {} Versions:", metadata.display_name())
                .green()
                .bold()
        );

        for version in versions.iter().rev() {
            let lts_marker = if version.is_lts && self.config.show_lts_indicator {
                format!(" {}", "(LTS)".green())
            } else {
                String::new()
            };

            println!("  {}{}", version.raw.cyan(), lts_marker);
        }

        println!("\n{}", "Usage:".yellow());
        println!("  jcvm install --tool {} <version>", tool_id);
        println!(
            "  jcvm install --tool {} {}",
            tool_id,
            versions.last().map(|v| v.raw.as_str()).unwrap_or("VERSION")
        );

        Ok(())
    }

    async fn install(&self, tool_id: &str, version_str: &str, force: bool) -> Result<()> {
        // Get tool metadata for display
        let metadata = self.get_tool_metadata(tool_id)?;

        print_info(&format!(
            "Installing {} version {}...",
            metadata.display_name(),
            version_str
        ));

        let installed = self
            .tool_manager
            .install(tool_id, version_str, force)
            .await?;

        print_success(&format!(
            "{} {} installed successfully",
            metadata.display_name(),
            installed.version.raw
        ));
        println!("  Path: {}", installed.path.display().to_string().dimmed());
        println!("\n{}", "Next steps:".yellow());
        println!(
            "  jcvm use --tool {} {}    # Activate this version",
            tool_id, version_str
        );
        println!(
            "  jcvm alias --tool {} default {}    # Set as default",
            tool_id, version_str
        );

        Ok(())
    }

    fn list(&self, tool_id: &str, show_all: bool) -> Result<()> {
        if show_all {
            // List all tools
            let plugins = self.registry.list_plugins()?;
            let mut has_any = false;

            for plugin_id in plugins {
                let installed = self.tool_manager.list_installed(Some(&plugin_id))?;
                if !installed.is_empty() {
                    has_any = true;
                    let metadata = self.tool_manager.metadata(&plugin_id)?;
                    println!(
                        "{}",
                        format!("\n{} Installations:", metadata.display_name())
                            .green()
                            .bold()
                    );
                    self.print_installed_list(&installed)?;
                }
            }

            if !has_any {
                print_warning("No tool versions installed");
                println!("\n{}", "Install a tool version:".yellow());
                println!("  jcvm install --tool java 21");
                println!("  jcvm install --tool node 20.10.0");
            }
        } else {
            // List specific tool
            let installed = self.tool_manager.list_installed(Some(tool_id))?;
            let metadata = self.get_tool_metadata(tool_id)?;

            if installed.is_empty() {
                print_warning(&format!(
                    "No {} versions installed",
                    metadata.display_name()
                ));
                println!("\n{}", "Install a version:".yellow());
                println!("  jcvm install --tool {} <version>", tool_id);
                return Ok(());
            }

            println!(
                "{}",
                format!("Installed {} Versions:", metadata.display_name())
                    .green()
                    .bold()
            );
            self.print_installed_list(&installed)?;

            let has_current = installed.iter().any(|i| i.is_current);
            if !has_current {
                println!("\n{}", "Activate a version:".yellow());
                println!("  jcvm use --tool {} <version>", tool_id);
            }
        }

        Ok(())
    }

    fn print_installed_list(&self, installed: &[ManagedInstallation]) -> Result<()> {
        for inst in installed {
            let mut markers = Vec::new();

            if inst.is_current {
                markers.push("current".green().to_string());
            }

            if inst.is_default {
                markers.push("default".blue().to_string());
            }

            if inst.version.is_lts && self.config.show_lts_indicator {
                markers.push("LTS".yellow().to_string());
            }

            let marker_str = if markers.is_empty() {
                String::new()
            } else {
                format!(" ({})", markers.join(", "))
            };

            let prefix = if inst.is_current {
                "→".green().bold()
            } else {
                " ".normal()
            };

            println!("  {} {}{}", prefix, inst.version.raw.cyan(), marker_str);
        }
        Ok(())
    }

    async fn use_version(&self, tool_id: &str, version: &str) -> Result<()> {
        let metadata = self.get_tool_metadata(tool_id)?;
        let ctx = self.tool_manager.set_current(tool_id, version).await?;

        print_success(&format!(
            "Now using {} {}",
            metadata.display_name(),
            version
        ));
        println!("  Path: {}", ctx.home_path.display().to_string().dimmed());

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
            println!(
                "\n{}",
                "⚠️  Shell integration not installed".yellow().bold()
            );
            println!("{}", "To activate in your current shell, run:".yellow());
            let activation = generate_activation_script(ctx.home_path.to_str().unwrap())?;
            println!("{}", activation.dimmed());
            println!(
                "\n{}",
                "For automatic activation in new shells, run:".yellow()
            );
            println!("  {}", "jcvm shell-init".cyan());
        }

        Ok(())
    }

    fn current(&self, tool_id: &str, show_all: bool) -> Result<()> {
        if show_all {
            // Show current for all tools
            let plugins = self.registry.list_plugins()?;
            let mut has_any = false;

            for plugin_id in plugins {
                if let Ok(Some(version)) = self.tool_manager.get_current(&plugin_id) {
                    has_any = true;
                    let metadata = self.tool_manager.metadata(&plugin_id)?;
                    println!(
                        "{}: {}",
                        metadata.display_name().green().bold(),
                        version.cyan()
                    );
                }
            }

            if !has_any {
                print_warning("No tool versions currently active");
            }
        } else {
            // Show current for specific tool
            let metadata = self.get_tool_metadata(tool_id)?;

            if let Some(version) = self.tool_manager.get_current(tool_id)? {
                println!(
                    "{}: {}",
                    format!("Current {}", metadata.display_name())
                        .green()
                        .bold(),
                    version.cyan()
                );
            } else {
                print_warning(&format!(
                    "No {} version currently active",
                    metadata.display_name()
                ));

                // For Java, check system Java as fallback
                if tool_id == "java" {
                    if let Ok(output) = std::process::Command::new("java").arg("-version").output()
                    {
                        println!("\n{}", "System Java:".yellow());
                        let version_info = String::from_utf8_lossy(&output.stderr);
                        println!("{}", version_info.dimmed());
                    }
                }
            }
        }

        Ok(())
    }

    async fn local(&self, tool_id: &str, version: Option<String>) -> Result<()> {
        let metadata = self.get_tool_metadata(tool_id)?;
        let version_file = format!(".{}-version", tool_id);

        if let Some(version_str) = version {
            // Check if version is installed
            let installs = self.tool_manager.list_installed(Some(tool_id))?;
            let is_installed = installs.iter().any(|i| i.version.raw == version_str);

            if !is_installed {
                print_error(&format!(
                    "{} {} is not installed",
                    metadata.display_name(),
                    version_str
                ));
                println!("\n{}", "Install it first:".yellow());
                println!("  jcvm install --tool {} {}", tool_id, version_str);
                return Ok(());
            }

            // Write version file
            std::fs::write(&version_file, format!("{}\n", version_str))?;
            print_success(&format!(
                "Set local {} version to {}",
                metadata.display_name(),
                version_str
            ));
            println!("  Created {} file", version_file);

            // Activate it for current shell
            if (self.tool_manager.set_current(tool_id, &version_str).await).is_ok() {
                println!("\n{}", "Version activated for current shell".green());
            }
        } else {
            // Show current local version
            if let Ok(content) = std::fs::read_to_string(&version_file) {
                let version = content.trim();
                println!(
                    "{}: {}",
                    format!("Local {} version", metadata.display_name())
                        .green()
                        .bold(),
                    version.cyan()
                );
            } else {
                print_warning(&format!("No {} file in current directory", version_file));
                println!("\n{}", "Set local version:".yellow());
                println!("  jcvm local --tool {} <version>", tool_id);
            }
        }

        Ok(())
    }

    async fn handle_tool(&self, action: &ToolCommands) -> Result<()> {
        match action {
            ToolCommands::List { tool, all } => {
                let installs = if *all {
                    self.tool_manager.list_installed(None)?
                } else {
                    let tool_id = tool.as_deref().unwrap_or("java").to_lowercase();
                    self.tool_manager.metadata(&tool_id)?;
                    self.tool_manager.list_installed(Some(&tool_id))?
                };

                if installs.is_empty() {
                    print_warning("No managed tool versions found");
                } else {
                    self.render_installations(installs);
                }
                Ok(())
            }
            ToolCommands::Install {
                tool,
                version,
                force,
            } => {
                let tool_id = tool.to_lowercase();
                let metadata = self.tool_manager.metadata(&tool_id)?;
                let installed = self.tool_manager.install(&tool_id, version, *force).await?;

                print_success(&format!(
                    "Installed {} {}",
                    metadata.display_name().cyan(),
                    installed.version.to_string().green()
                ));
                println!("  Tool: {}", metadata.id.dimmed());
                println!("  Path: {}", installed.path.display().to_string().dimmed());
                println!("  Source: {}", installed.source.dimmed());
                println!("\n{}", "Try it now:".yellow());
                println!("  jcvm tool use {} {}", metadata.id, installed.version.raw);
                Ok(())
            }
            ToolCommands::Uninstall { tool, version, yes } => {
                let tool_id = tool.to_lowercase();
                let metadata = self.tool_manager.metadata(&tool_id)?;

                if !*yes {
                    let prompt = format!(
                        "Remove {} {}?",
                        metadata.display_name().cyan(),
                        version.green()
                    );
                    if !confirm(&prompt) {
                        print_info("Aborted");
                        return Ok(());
                    }
                }

                self.tool_manager.uninstall(&tool_id, version).await?;
                print_success(&format!(
                    "Removed {} {}",
                    metadata.display_name().cyan(),
                    version.green()
                ));
                Ok(())
            }
            ToolCommands::Remote { tool, lts } => {
                let tool_id = tool.to_lowercase();
                let metadata = self.tool_manager.metadata(&tool_id)?;
                print_info(&format!(
                    "Fetching remote versions for {}...",
                    metadata.id.cyan()
                ));
                let versions = self
                    .tool_manager
                    .list_remote_versions(&tool_id, *lts)
                    .await?;

                if versions.is_empty() {
                    print_warning("No remote versions reported");
                    return Ok(());
                }

                println!(
                    "\n{}",
                    format!("Available {} versions:", metadata.display_name())
                        .green()
                        .bold()
                );

                for version in versions {
                    let mut marker = String::new();
                    if version.is_lts {
                        marker.push_str(&format!(" {}", "(LTS)".yellow()));
                    }
                    if let Some(meta) = &version.metadata {
                        if !meta.is_empty() {
                            marker.push_str(&format!(" {}", meta.dimmed()));
                        }
                    }
                    println!("  {}{}", version.raw.cyan(), marker);
                }

                println!("\n{}", "Quick switch:".yellow());
                println!("  jcvm switch {}@<version>", metadata.id);
                Ok(())
            }
            ToolCommands::Use { tool, version } => {
                let tool_id = tool.to_lowercase();
                let ctx = self.activate_tool(&tool_id, version).await?;
                self.print_activation(&tool_id, &ctx);
                Ok(())
            }
            ToolCommands::Current { tool, all } => {
                if *all {
                    let installs = self.tool_manager.list_installed(None)?;
                    let active: Vec<_> = installs.into_iter().filter(|i| i.is_current).collect();
                    if active.is_empty() {
                        print_warning("No active tool versions");
                    } else {
                        self.render_installations(active);
                    }
                } else {
                    let tool_id = tool.as_deref().unwrap_or("java").to_lowercase();
                    self.tool_manager.metadata(&tool_id)?;
                    match self.tool_manager.get_current(&tool_id)? {
                        Some(version) => {
                            let metadata = self.tool_manager.metadata(&tool_id)?;
                            println!(
                                "{} {}",
                                format!("{} current:", metadata.display_name())
                                    .green()
                                    .bold(),
                                version.cyan()
                            );
                        }
                        None => {
                            print_warning("No active version");
                            println!("  jcvm tool use {} <version>", tool_id);
                        }
                    }
                }
                Ok(())
            }
            ToolCommands::Alias {
                tool,
                alias,
                version,
                unset,
            } => {
                let tool_id = tool.to_lowercase();
                self.tool_manager.metadata(&tool_id)?;

                if *unset {
                    self.tool_manager.delete_alias(&tool_id, alias)?;
                    print_success(&format!(
                        "Removed {} alias {}",
                        tool_id.cyan(),
                        alias.green()
                    ));
                    return Ok(());
                }

                if let Some(value) = version {
                    self.tool_manager.set_alias(&tool_id, alias, value)?;
                    print_success(&format!(
                        "{} alias {} → {}",
                        tool_id.cyan(),
                        alias.green(),
                        value.cyan()
                    ));
                } else {
                    match self.tool_manager.get_alias(&tool_id, alias)? {
                        Some(target) => {
                            println!(
                                "{} {}",
                                format!("{} alias {}:", tool_id, alias).green().bold(),
                                target.cyan()
                            );
                        }
                        None => {
                            print_warning(&format!(
                                "Alias {} not set for {}",
                                alias.cyan(),
                                tool_id.cyan()
                            ));
                        }
                    }
                }
                Ok(())
            }
        }
    }

    async fn quick_switch(&self, target: &str, install_if_missing: bool) -> Result<()> {
        let (tool_id, version) = Self::parse_tool_target(target)?;
        self.tool_manager.metadata(&tool_id)?;

        match self.activate_tool(&tool_id, &version).await {
            Ok(ctx) => {
                self.print_activation(&tool_id, &ctx);
                Ok(())
            }
            Err(JcvmError::VersionNotFound(_)) if install_if_missing => {
                print_info(&format!(
                    "{} not installed. Fetching...",
                    format!("{} {}", tool_id, version).cyan()
                ));
                self.tool_manager.install(&tool_id, &version, false).await?;
                let ctx = self.activate_tool(&tool_id, &version).await?;
                self.print_activation(&tool_id, &ctx);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    async fn activate_tool(&self, tool_id: &str, version: &str) -> Result<ActivationContext> {
        let ctx = self.tool_manager.set_current(tool_id, version).await?;
        Ok(ctx)
    }

    fn print_activation(&self, tool_id: &str, ctx: &ActivationContext) {
        let display_name = self
            .tool_manager
            .metadata(tool_id)
            .map(|m| m.name)
            .unwrap_or_else(|_| tool_id.to_string());

        print_success(&format!(
            "Now using {} {}",
            display_name.cyan(),
            ctx.version.raw.green()
        ));
        println!("  Home: {}", ctx.home_path.display().to_string().dimmed());

        if !ctx.env.is_empty() {
            println!("  Env:");
            for (key, value) in &ctx.env {
                println!("    {}={}", key.green(), value.dimmed());
            }
        }

        if tool_id == "java" {
            let home_str = ctx.home_path.display().to_string();
            self.print_shell_hint(&home_str);
        }
    }

    fn print_shell_hint(&self, java_home: &str) {
        let shell = Shell::detect();
        let has_integration = if let Some(shell) = shell {
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

        if !has_integration {
            println!(
                "\n{}",
                "⚠️  Shell integration not installed".yellow().bold()
            );
            println!("{}", "To activate in your current shell, run:".yellow());
            if let Ok(activation) = generate_activation_script(java_home) {
                println!("{}", activation.dimmed());
            }
            println!(
                "\n{}",
                "For automatic activation in new shells, run:".yellow()
            );
            println!("  {}", "jcvm shell-init".cyan());
        }
    }

    fn render_installations(&self, installs: Vec<ManagedInstallation>) {
        let mut grouped: BTreeMap<String, Vec<ManagedInstallation>> = BTreeMap::new();
        for install in installs {
            grouped
                .entry(install.tool_id.clone())
                .or_default()
                .push(install);
        }

        for (tool_id, mut entries) in grouped {
            let title = self
                .tool_manager
                .metadata(&tool_id)
                .map(|m| format!("{} ({})", m.name, m.id))
                .unwrap_or_else(|_| tool_id.clone());
            println!("\n{}", title.green().bold());

            entries.sort_by(|a, b| a.version.raw.cmp(&b.version.raw));

            for entry in entries {
                let mut markers = Vec::new();
                if entry.is_current {
                    markers.push("current".green().to_string());
                }
                if entry.is_default {
                    markers.push("default".blue().to_string());
                }
                if entry.version.is_lts {
                    markers.push("LTS".yellow().to_string());
                }

                let mut line = format!("  {}", entry.version.raw.cyan());
                if !markers.is_empty() {
                    let mut marker_text = String::new();
                    let _ = write!(marker_text, " ({})", markers.join(", "));
                    line.push_str(&marker_text);
                }

                println!("{}", line);
                println!("    {}", entry.path.display().to_string().dimmed());
            }
        }
    }

    fn parse_tool_target(target: &str) -> Result<(String, String)> {
        let trimmed = target.trim();
        if trimmed.is_empty() {
            return Err(JcvmError::InvalidVersion("missing target".to_string()));
        }

        let (tool, version) = if let Some((tool, version)) = trimmed.split_once('@') {
            (tool.to_lowercase(), version.trim().to_string())
        } else {
            ("java".to_string(), trimmed.to_string())
        };

        if version.is_empty() {
            return Err(JcvmError::InvalidVersion(trimmed.to_string()));
        }
        Ok((tool, version))
    }

    fn alias(&self, tool_id: &str, name: Option<String>, version: Option<String>) -> Result<()> {
        let metadata = self.get_tool_metadata(tool_id)?;

        match (name, version) {
            (Some(name), Some(version)) => {
                // Set alias
                self.tool_manager.set_alias(tool_id, &name, &version)?;
                print_success(&format!(
                    "Set {} alias '{}' to version {}",
                    metadata.display_name(),
                    name,
                    version
                ));

                if name == "default" {
                    println!(
                        "\n{}",
                        "The default version will be used on shell startup".dimmed()
                    );
                }
            }
            (Some(name), None) => {
                // Show specific alias
                if let Some(version) = self.tool_manager.get_alias(tool_id, &name)? {
                    println!(
                        "{} {} → {}",
                        metadata.display_name(),
                        name.cyan(),
                        version.green()
                    );
                } else {
                    print_warning(&format!(
                        "{} alias '{}' is not set",
                        metadata.display_name(),
                        name
                    ));
                }
            }
            _ => {
                // List all aliases for this tool
                println!(
                    "{}",
                    format!("{} Aliases:", metadata.display_name())
                        .green()
                        .bold()
                );

                let alias_dir = self.config.tool_alias_dir(tool_id);
                if alias_dir.exists() {
                    let mut found_any = false;
                    for entry in std::fs::read_dir(&alias_dir)? {
                        let entry = entry?;
                        let path = entry.path();

                        if path.is_symlink() {
                            if let Ok(target) = std::fs::read_link(&path) {
                                if let (Some(alias_name), Some(_)) = (
                                    path.file_name().and_then(|n| n.to_str()),
                                    target.file_name().and_then(|n| n.to_str()),
                                ) {
                                    if let Ok(Some(version)) =
                                        self.tool_manager.get_alias(tool_id, alias_name)
                                    {
                                        found_any = true;
                                        println!("  {} → {}", alias_name.cyan(), version.green());
                                    }
                                }
                            }
                        }
                    }
                    if !found_any {
                        println!("  (none)");
                    }
                } else {
                    println!("  (none)");
                }
            }
        }

        Ok(())
    }

    async fn uninstall(&self, tool_id: &str, version: &str, skip_confirm: bool) -> Result<()> {
        let metadata = self.get_tool_metadata(tool_id)?;

        // Check if it's the current version
        if let Ok(Some(current)) = self.tool_manager.get_current(tool_id) {
            if current == version {
                print_warning(&format!(
                    "{} {} is currently active",
                    metadata.display_name(),
                    version
                ));
            }
        }

        let should_uninstall = skip_confirm
            || confirm(&format!(
                "Uninstall {} {}?",
                metadata.display_name(),
                version
            ));

        if should_uninstall {
            self.tool_manager.uninstall(tool_id, version).await?;
            print_success(&format!(
                "Uninstalled {} {}",
                metadata.display_name(),
                version
            ));
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
            print_info("Checking for existing tool installations...");

            // Use async runtime for detection
            let runtime = tokio::runtime::Runtime::new()?;
            let tool_manager = self.tool_manager.clone();

            let detection_results =
                runtime.block_on(async { tool_manager.detect_and_import_all().await });

            if let Ok(results) = detection_results {
                let total_tools = results.len();
                let total_imported: usize = results.iter().map(|(_, imported, _)| imported).sum();

                if total_tools > 0 {
                    println!("\n{}", "Found external installations:".green().bold());

                    for (tool_id, imported, detected) in &results {
                        let metadata = self.tool_manager.metadata(tool_id).ok();
                        let tool_name = metadata
                            .as_ref()
                            .map(|m| m.display_name())
                            .unwrap_or(tool_id);

                        if *detected > 0 {
                            println!(
                                "  {}: {} detected, {} imported",
                                tool_name.cyan(),
                                detected,
                                imported
                            );
                        }
                    }

                    if total_imported > 0 {
                        println!();
                        print_success(&format!(
                            "Imported {} installation(s) across {} tool(s)",
                            total_imported, total_tools
                        ));

                        // Set default for each tool if not already set
                        for (tool_id, imported, _) in &results {
                            if *imported > 0 {
                                // Get the first installed version and set as default
                                if let Ok(installations) =
                                    self.tool_manager.list_installed(Some(tool_id))
                                {
                                    if let Some(first) = installations.first() {
                                        let _ = self.tool_manager.set_alias(
                                            tool_id,
                                            "default",
                                            &first.version.raw,
                                        );
                                    }
                                }
                            }
                        }
                    }
                } else {
                    print_info("No external installations detected.");
                    println!("\n{}", "You can install tools later with:".yellow());
                    println!("  jcvm install --tool java 21");
                    println!("  jcvm install --tool node 20");
                    println!("  jcvm install --tool python 3.12");
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
            println!(
                "{} {} (from .java-version)",
                "Would use:".green().bold(),
                local_version.to_string().cyan()
            );
            return Ok(());
        }

        // Check current version
        let manager = VersionManager::new(self.config.clone());
        if let Some(current) = manager.get_current()? {
            println!(
                "{} {} (current)",
                "Would use:".green().bold(),
                current.cyan()
            );
            return Ok(());
        }

        // Check default version
        if let Some(default) = manager.get_default()? {
            println!(
                "{} {} (default)",
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

        println!(
            "{} {} in cache ({} files)",
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
            println!(
                "  {} {}",
                "JCVM Directory:".cyan(),
                self.config.jcvm_dir.display()
            );
            println!(
                "  {} {}",
                "Versions Directory:".cyan(),
                self.config.versions_dir.display()
            );
            println!(
                "  {} {}",
                "Cache Directory:".cyan(),
                self.config.cache_dir.display()
            );
            println!(
                "  {} {}",
                "Verify Checksums:".cyan(),
                self.config.verify_checksums
            );
            println!(
                "  {} {}",
                "Cache Downloads:".cyan(),
                self.config.cache_downloads
            );
            println!(
                "  {} {} days",
                "Cache Retention:".cyan(),
                self.config.cache_retention_days
            );
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

    async fn detect(&self, tool_filter: Option<String>, auto_import: bool) -> Result<()> {
        if let Some(tool_id) = tool_filter {
            // Detect for a specific tool
            self.detect_single_tool(&tool_id, auto_import).await
        } else {
            // Detect for all tools
            self.detect_all_tools(auto_import).await
        }
    }

    async fn detect_single_tool(&self, tool_id: &str, auto_import: bool) -> Result<()> {
        let metadata = self.get_tool_metadata(tool_id)?;

        print_info(&format!(
            "Detecting {} installations on the system...",
            metadata.display_name()
        ));

        let detected = self.tool_manager.detect_tool_installations(tool_id).await?;

        if detected.is_empty() {
            print_warning(&format!(
                "No {} installations detected",
                metadata.display_name()
            ));
            println!(
                "\n{}",
                format!("Install a {} version:", metadata.display_name()).yellow()
            );
            println!("  jcvm install --tool {} <version>", tool_id);
            return Ok(());
        }

        println!(
            "\n{}",
            format!(
                "Found {} {} installation(s):",
                detected.len(),
                metadata.display_name()
            )
            .green()
            .bold()
        );

        for (i, installation) in detected.iter().enumerate() {
            println!(
                "\n{}. {} {} ({})",
                i + 1,
                metadata.display_name(),
                installation.version.to_string().cyan(),
                installation.source.yellow()
            );
            println!(
                "   Path: {}",
                installation.path.display().to_string().dimmed()
            );
        }

        if auto_import {
            println!("\n{}", "Importing all detected installations...".yellow());

            let mut imported_count = 0;
            let mut skipped_count = 0;

            for installation in &detected {
                match self
                    .tool_manager
                    .import_tool_installation(tool_id, installation)
                    .await
                {
                    Ok(_) => {
                        imported_count += 1;
                        print_success(&format!(
                            "Imported {} {}",
                            metadata.display_name(),
                            installation.version
                        ));
                    }
                    Err(e) => {
                        if e.to_string().contains("already") {
                            skipped_count += 1;
                        } else {
                            print_warning(&format!(
                                "Failed to import {}: {}",
                                installation.version, e
                            ));
                        }
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
            println!("  jcvm detect --tool {} --import    # Import all", tool_id);
            println!("  jcvm import <path>                  # Import specific installation");
        }

        Ok(())
    }

    async fn detect_all_tools(&self, auto_import: bool) -> Result<()> {
        print_info("Detecting all tool installations on the system...");

        if auto_import {
            let results = self.tool_manager.detect_and_import_all().await?;

            if results.is_empty() {
                print_warning("No external installations detected for any tool");
                println!("\n{}", "Install tools with:".yellow());
                println!("  jcvm install --tool java 21");
                println!("  jcvm install --tool node 20");
                println!("  jcvm install --tool python 3.12");
                return Ok(());
            }

            println!("\n{}", "Detection and Import Results:".green().bold());

            let mut total_detected = 0;
            let mut total_imported = 0;

            for (tool_id, imported, detected) in &results {
                total_detected += detected;
                total_imported += imported;

                let metadata = self.tool_manager.metadata(tool_id).ok();
                let tool_name = metadata
                    .as_ref()
                    .map(|m| m.display_name())
                    .unwrap_or(tool_id);

                println!(
                    "  {}: {} detected, {} imported",
                    tool_name.cyan(),
                    detected,
                    imported
                );
            }

            println!();
            print_success(&format!(
                "Imported {} installation(s) across {} tool(s)",
                total_imported,
                results.len()
            ));
        } else {
            let results = self.tool_manager.detect_all().await?;

            if results.is_empty() {
                print_warning("No external installations detected for any tool");
                println!("\n{}", "Install tools with:".yellow());
                println!("  jcvm install --tool java 21");
                println!("  jcvm install --tool node 20");
                println!("  jcvm install --tool python 3.12");
                return Ok(());
            }

            println!("\n{}", "Detection Results:".green().bold());

            for (tool_id, detected) in &results {
                let metadata = self.tool_manager.metadata(tool_id).ok();
                let tool_name = metadata
                    .as_ref()
                    .map(|m| m.display_name())
                    .unwrap_or(tool_id);

                println!("  {}: {} detected", tool_name.cyan(), detected);
            }

            println!();
            println!("{}", "To import detected installations:".yellow());
            println!("  jcvm detect --import              # Import all tools");
            println!("  jcvm detect --tool <name> --import # Import specific tool");
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
                    println!(
                        "  jcvm use {}              # Activate this version",
                        java.version
                    );
                    println!("  jcvm alias default {}    # Set as default", java.version);
                }
                Err(e) => {
                    print_error(&format!("Failed to import: {}", e));
                }
            }
        } else {
            print_error("Not a valid Java installation or could not detect version");
            println!(
                "\n{}",
                "The path should point to JAVA_HOME (contains bin/java)".dimmed()
            );
        }

        Ok(())
    }
}

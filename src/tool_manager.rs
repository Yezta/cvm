use crate::config::Config;
use crate::core::plugin::PluginRegistry;
use crate::core::traits::{
    Architecture, InstalledTool, Platform, PluginMetadata, ToolInfo, ToolPlugin, ToolVersion,
};
use crate::error::{JcvmError, Result};
use chrono::{DateTime, Utc};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::warn;

const MANIFEST_FILE: &str = ".jcvm-manifest.json";

#[derive(Clone)]
pub struct ToolManager {
    config: Config,
    registry: PluginRegistry,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new(Config::default(), PluginRegistry::default())
    }
}

#[derive(Debug, Clone)]
pub struct ManagedInstallation {
    pub tool_id: String,
    pub version: ToolVersion,
    pub path: PathBuf,
    pub is_current: bool,
    pub is_default: bool,
    pub installed_at: DateTime<Utc>,
    pub manifest: Option<InstalledTool>,
}

#[derive(Debug, Clone)]
pub struct ActivationContext {
    pub tool_id: String,
    pub tool_info: ToolInfo,
    pub version: ToolVersion,
    pub install_path: PathBuf,
    pub home_path: PathBuf,
    pub env: Vec<(String, String)>,
}

impl ToolManager {
    pub fn new(config: Config, registry: PluginRegistry) -> Self {
        Self { config, registry }
    }

    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    pub fn metadata(&self, tool_id: &str) -> Result<PluginMetadata> {
        self.registry.get_metadata(tool_id)
    }

    pub async fn list_remote_versions(
        &self,
        tool_id: &str,
        lts_only: bool,
    ) -> Result<Vec<ToolVersion>> {
        let plugin = self.plugin(tool_id)?;
        plugin.list_remote_versions(lts_only).await
    }

    pub async fn install(
        &self,
        tool_id: &str,
        version_str: &str,
        force: bool,
    ) -> Result<InstalledTool> {
        let plugin = self.plugin(tool_id)?;
        let version = plugin.parse_version(version_str)?;
        let dest_dir = self.config.tool_version_dir(tool_id, &version.raw)?;

        if dest_dir.exists() {
            if force {
                self.uninstall(tool_id, &version.raw).await?;
            } else {
                return Err(JcvmError::VersionAlreadyInstalled(
                    version.raw.clone(),
                    dest_dir.display().to_string(),
                ));
            }
        }

        if let Some(parent) = dest_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        let (platform, arch) = Self::detect_platform()?;
        if !plugin.supports_platform(platform, arch) {
            return Err(JcvmError::UnsupportedPlatform {
                os: platform.to_string(),
                arch: arch.to_string(),
            });
        }

        let distribution = plugin.find_distribution(&version, platform, arch).await?;
        let installed = plugin.install(&distribution, &dest_dir).await?;
        self.write_manifest(&installed)?;
        Ok(installed)
    }

    pub async fn uninstall(&self, tool_id: &str, version_str: &str) -> Result<()> {
        let plugin = self.plugin(tool_id)?;
        let install_dir = self.resolve_install_dir(tool_id, version_str)?;

        let manifest = self.read_manifest(&install_dir)?;
        let installed = match manifest {
            Some(m) => m,
            None => {
                // Manifest is missing - this could indicate corrupted installation data
                warn!(
                    "Manifest file not found for {} {} at {}. This may indicate a corrupted installation.",
                    tool_id, version_str, install_dir.display()
                );

                // Try to parse the version, but fail if parsing fails
                let parsed = plugin.parse_version(version_str).map_err(|e| {
                    JcvmError::InvalidToolStructure {
                        tool: tool_id.to_string(),
                        message: format!(
                            "Cannot uninstall: manifest missing and version parsing failed ({}). \
                            Installation data may be corrupted at: {}",
                            e,
                            install_dir.display()
                        ),
                    }
                })?;

                InstalledTool {
                    tool_id: tool_id.to_string(),
                    version: parsed,
                    path: install_dir.clone(),
                    installed_at: Utc::now(),
                    source: "unknown".to_string(),
                    executable_path: None,
                }
            }
        };

        plugin.uninstall(&installed).await?;
        let _ = fs::remove_file(self.manifest_path(&install_dir));
        self.cleanup_aliases(tool_id, &install_dir)?;
        Ok(())
    }

    /// Detect all external installations for a specific tool
    pub async fn detect_tool_installations(
        &self,
        tool_id: &str,
    ) -> Result<Vec<crate::core::traits::DetectedInstallation>> {
        let plugin = self.plugin(tool_id)?;
        plugin.detect_installations().await
    }

    /// Import a detected installation for a specific tool
    pub async fn import_tool_installation(
        &self,
        tool_id: &str,
        detected: &crate::core::traits::DetectedInstallation,
    ) -> Result<InstalledTool> {
        let plugin = self.plugin(tool_id)?;
        let dest_dir = self
            .config
            .tool_version_dir(tool_id, &detected.version.raw)?;

        // Check if already imported
        if dest_dir.exists() {
            return Err(JcvmError::VersionAlreadyInstalled(
                detected.version.raw.clone(),
                dest_dir.display().to_string(),
            ));
        }

        if let Some(parent) = dest_dir.parent() {
            fs::create_dir_all(parent)?;
        }

        let installed = plugin.import_installation(detected, &dest_dir).await?;

        // Write the manifest
        self.write_manifest(&installed)?;

        Ok(installed)
    }

    /// Detect all external installations for all registered plugins
    /// Returns a list of (tool_id, total_detected_count) without importing
    pub async fn detect_all(&self) -> Result<Vec<(String, usize)>> {
        let mut results = Vec::new();

        let tool_ids = self.registry.list_plugins()?;

        for tool_id in tool_ids {
            let plugin = match self.plugin(&tool_id) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let detected = match plugin.detect_installations().await {
                Ok(d) => d,
                Err(_) => continue,
            };

            let total_count = detected.len();
            if total_count > 0 {
                results.push((tool_id, total_count));
            }
        }

        Ok(results)
    }

    /// Detect and import all external installations for all registered plugins
    /// Returns a list of (tool_id, imported_count, total_detected_count)
    pub async fn detect_and_import_all(&self) -> Result<Vec<(String, usize, usize)>> {
        let mut results = Vec::new();

        let tool_ids = self.registry.list_plugins()?;

        for tool_id in tool_ids {
            let plugin = match self.plugin(&tool_id) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let detected = match plugin.detect_installations().await {
                Ok(d) => d,
                Err(_) => continue,
            };

            if detected.is_empty() {
                continue;
            }

            let mut imported_count = 0;
            let total_count = detected.len();

            for installation in detected {
                let dest_dir = self
                    .config
                    .tool_version_dir(&tool_id, &installation.version.raw)?;

                // Skip if already managed
                if dest_dir.exists() {
                    continue;
                }

                if let Some(parent) = dest_dir.parent() {
                    fs::create_dir_all(parent)?;
                }

                if let Ok(installed) = plugin.import_installation(&installation, &dest_dir).await {
                    // Write manifest for imported installation
                    if self.write_manifest(&installed).is_ok() {
                        imported_count += 1;
                    }
                }
            }

            if total_count > 0 {
                results.push((tool_id, imported_count, total_count));
            }
        }

        Ok(results)
    }

    pub async fn set_current(&self, tool_id: &str, version_str: &str) -> Result<ActivationContext> {
        let plugin = self.plugin(tool_id)?;
        let install_dir = self.resolve_install_dir(tool_id, version_str)?;

        if !plugin.validate_installation(&install_dir)? {
            return Err(JcvmError::InvalidToolStructure {
                tool: tool_id.to_string(),
                message: format!(
                    "Installation at {} failed validation",
                    install_dir.display()
                ),
            });
        }

        let (home_path, env_vars) = self.load_env(&plugin, &install_dir)?;

        let current_link = self.config.tool_current_symlink(tool_id)?;
        self.replace_symlink(&home_path, &current_link)?;

        if tool_id == "java" {
            let legacy_current = self.config.alias_dir.join("current");
            if legacy_current != current_link {
                self.replace_symlink(&home_path, &legacy_current)?;
            }
        }

        let manifest = self.read_manifest(&install_dir)?;
        let version = match manifest.as_ref() {
            Some(m) => m.version.clone(),
            None => {
                // Manifest is missing - log a warning but continue with parsed version
                warn!(
                    "Manifest file not found for {} {} at {}. This may indicate a corrupted installation.",
                    tool_id, version_str, install_dir.display()
                );

                // Try to parse the version, but fail if parsing fails
                plugin
                    .parse_version(version_str)
                    .map_err(|e| JcvmError::InvalidToolStructure {
                        tool: tool_id.to_string(),
                        message: format!(
                            "Cannot activate: manifest missing and version parsing failed ({}). \
                            Installation data may be corrupted at: {}",
                            e,
                            install_dir.display()
                        ),
                    })?
            }
        };

        let info = plugin.info();

        Ok(ActivationContext {
            tool_id: tool_id.to_string(),
            tool_info: info,
            version,
            install_path: install_dir,
            home_path,
            env: env_vars,
        })
    }

    pub fn set_alias(&self, tool_id: &str, alias: &str, version_str: &str) -> Result<()> {
        let plugin = self.plugin(tool_id)?;
        let install_dir = self.resolve_install_dir(tool_id, version_str)?;
        let (home_path, _env) = self.load_env(&plugin, &install_dir)?;
        let alias_path = self.config.tool_alias_path(tool_id, alias)?;
        self.replace_symlink(&home_path, &alias_path)?;

        if tool_id == "java" {
            let legacy_alias = self.config.alias_dir.join(alias);
            if legacy_alias != alias_path {
                self.replace_symlink(&home_path, &legacy_alias)?;
            }
        }
        Ok(())
    }

    pub fn delete_alias(&self, tool_id: &str, alias: &str) -> Result<()> {
        let alias_path = self.config.tool_alias_path(tool_id, alias)?;
        self.remove_link(&alias_path)?;
        if tool_id == "java" {
            let legacy_alias = self.config.alias_dir.join(alias);
            let _ = self.remove_link(&legacy_alias);
        }
        Ok(())
    }

    pub fn get_alias(&self, tool_id: &str, alias: &str) -> Result<Option<String>> {
        let alias_path = self.config.tool_alias_path(tool_id, alias)?;
        if let Some(target) = self.symlink_target(&alias_path)? {
            let installs = self.list_installed(Some(tool_id))?;
            if let Some(found) = installs
                .into_iter()
                .find(|i| Self::target_matches(&target, &i.path))
            {
                return Ok(Some(found.version.raw));
            }
        }

        if tool_id == "java" {
            let legacy_alias = self.config.alias_dir.join(alias);
            if let Some(target) = self.symlink_target(&legacy_alias)? {
                let installs = self.list_installed(Some(tool_id))?;
                if let Some(found) = installs
                    .into_iter()
                    .find(|i| Self::target_matches(&target, &i.path))
                {
                    return Ok(Some(found.version.raw));
                }
            }
        }

        Ok(None)
    }

    pub fn get_current(&self, tool_id: &str) -> Result<Option<String>> {
        let installs = self.list_installed(Some(tool_id))?;
        Ok(installs
            .into_iter()
            .find(|i| i.is_current)
            .map(|i| i.version.raw))
    }

    pub fn list_installed(&self, tool_filter: Option<&str>) -> Result<Vec<ManagedInstallation>> {
        let mut tool_ids = if let Some(tool) = tool_filter {
            vec![tool.to_string()]
        } else {
            self.registry.list_plugins()?
        };

        tool_ids.sort();

        let mut results = Vec::new();

        for tool_id in tool_ids {
            let plugin = self.plugin(&tool_id)?;
            let versions_root = self.config.tool_versions_dir(&tool_id);
            let current_link = self.config.tool_current_symlink(&tool_id)?;
            let default_link = self.config.tool_default_symlink(&tool_id)?;
            let mut current_links = vec![current_link.clone()];
            let mut default_links = vec![default_link.clone()];

            if tool_id == "java" {
                current_links.push(self.config.alias_dir.join("current"));
                default_links.push(self.config.alias_dir.join("default"));
            }

            let mut seen_versions = HashSet::new();

            if versions_root.exists() {
                for entry in fs::read_dir(&versions_root)? {
                    let entry = entry?;
                    let file_type = entry.file_type()?;

                    // Skip if it's not a directory and not a symlink
                    if !file_type.is_dir() && !file_type.is_symlink() {
                        continue;
                    }

                    let folder = entry.file_name().to_string_lossy().to_string();
                    let version = match plugin.parse_version(&folder) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    if !seen_versions.insert(version.raw.clone()) {
                        continue;
                    }

                    let path = entry.path();
                    let manifest = self.read_manifest(&path)?;
                    let installed_at = manifest
                        .as_ref()
                        .map(|m| m.installed_at)
                        .or_else(|| Self::metadata_timestamp(&path))
                        .unwrap_or_else(Utc::now);

                    let is_current = self.links_point_to(&current_links, &path)?;
                    let is_default = self.links_point_to(&default_links, &path)?;

                    results.push(ManagedInstallation {
                        tool_id: tool_id.clone(),
                        version,
                        path,
                        is_current,
                        is_default,
                        installed_at,
                        manifest,
                    });
                }
            }

            if tool_id == "java" {
                self.collect_legacy_java(
                    &plugin,
                    &mut seen_versions,
                    &current_links,
                    &default_links,
                    &mut results,
                )?;
            }
        }

        results.sort_by(|a, b| {
            a.tool_id
                .cmp(&b.tool_id)
                .then_with(|| Self::compare_versions_desc(&a.version, &b.version))
        });

        Ok(results)
    }

    fn plugin(&self, tool_id: &str) -> Result<Arc<dyn ToolPlugin>> {
        self.registry.get(tool_id)
    }

    fn detect_platform() -> Result<(Platform, Architecture)> {
        let platform = match std::env::consts::OS {
            "macos" => Platform::Mac,
            "linux" => Platform::Linux,
            "windows" => Platform::Windows,
            other => {
                return Err(JcvmError::UnsupportedPlatform {
                    os: other.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                })
            }
        };

        let arch = match std::env::consts::ARCH {
            "x86_64" | "amd64" => Architecture::X64,
            "aarch64" | "arm64" => Architecture::Aarch64,
            "x86" | "i686" => Architecture::X86,
            "arm" => Architecture::Arm,
            other => {
                return Err(JcvmError::UnsupportedPlatform {
                    os: platform.to_string(),
                    arch: other.to_string(),
                })
            }
        };

        Ok((platform, arch))
    }

    fn write_manifest(&self, installed: &InstalledTool) -> Result<()> {
        let manifest_path = self.manifest_path(&installed.path);

        // If the path is a symlink, we need to write the manifest in a metadata directory
        // instead of following the symlink (which would modify external installations)
        let write_path = if installed.path.is_symlink() {
            // Create metadata directory alongside versions
            let metadata_dir = self.config.versions_dir.join(".metadata");
            fs::create_dir_all(&metadata_dir)?;

            // Use tool_id and version to create unique metadata path
            let metadata_file = format!("{}_{}.json", installed.tool_id, installed.version.raw);
            metadata_dir.join(metadata_file)
        } else {
            manifest_path
        };

        let file = fs::File::create(write_path)?;
        serde_json::to_writer_pretty(BufWriter::new(file), installed)?;
        Ok(())
    }

    fn read_manifest(&self, install_dir: &Path) -> Result<Option<InstalledTool>> {
        // First try the standard manifest path
        let manifest_path = self.manifest_path(install_dir);
        if manifest_path.exists() {
            let file = fs::File::open(manifest_path)?;
            let manifest = serde_json::from_reader(BufReader::new(file))?;
            return Ok(Some(manifest));
        }

        // If not found and the path is a symlink, check metadata directory
        if install_dir.is_symlink() {
            // Extract tool_id and version from path
            if let Some(version_str) = install_dir.file_name().and_then(|n| n.to_str()) {
                if let Some(tool_dir) = install_dir.parent() {
                    if let Some(tool_id) = tool_dir.file_name().and_then(|n| n.to_str()) {
                        let metadata_dir = self.config.versions_dir.join(".metadata");
                        let metadata_file = format!("{}_{}.json", tool_id, version_str);
                        let metadata_path = metadata_dir.join(metadata_file);

                        if metadata_path.exists() {
                            let file = fs::File::open(metadata_path)?;
                            let manifest = serde_json::from_reader(BufReader::new(file))?;
                            return Ok(Some(manifest));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    fn manifest_path(&self, install_dir: &Path) -> PathBuf {
        install_dir.join(MANIFEST_FILE)
    }

    /// Find the best matching installed version for a given version prefix.
    ///
    /// This is a generic feature that works for all plugins by default.
    /// Each plugin's `parse_version` method is used to validate candidates.
    ///
    /// # Examples
    /// - Python: "3.13" → "3.13.7", "3.10" → "3.10.18" (highest patch)
    /// - Node.js: "22" → "22.19.0", "18" → "18.20.8"
    /// - Java: "21" → "21.0.7", "1.8" → "1.8.0_452"
    ///
    /// # Returns
    /// - `Ok(Some(version_string))` if a matching version is found
    /// - `Ok(None)` if no matching version exists
    fn find_matching_version(&self, tool_id: &str, version_prefix: &str) -> Result<Option<String>> {
        let plugin = self.plugin(tool_id)?;
        let versions_root = self.config.tool_versions_dir(tool_id);

        if !versions_root.exists() {
            return Ok(None);
        }

        let mut candidates = Vec::new();

        for entry in fs::read_dir(&versions_root)? {
            let entry = entry?;
            let file_type = entry.file_type()?;

            if !file_type.is_dir() && !file_type.is_symlink() {
                continue;
            }

            let folder = entry.file_name().to_string_lossy().to_string();

            // Check if this folder starts with the version prefix
            if folder.starts_with(version_prefix) {
                // Try to parse as a version to validate it
                if let Ok(parsed_version) = plugin.parse_version(&folder) {
                    candidates.push((folder, parsed_version));
                }
            }
        }

        // If we have candidates, sort them and return the best match
        if !candidates.is_empty() {
            // Sort by version in descending order (highest version first)
            candidates.sort_by(|(_, a), (_, b)| Self::compare_versions_desc(a, b));

            // Return the highest matching version
            Ok(Some(candidates[0].0.clone()))
        } else {
            Ok(None)
        }
    }

    fn resolve_install_dir(&self, tool_id: &str, version: &str) -> Result<PathBuf> {
        let primary = self.config.tool_version_dir(tool_id, version)?;
        if primary.exists() {
            return Ok(primary);
        }

        // Try fuzzy matching for partial versions (e.g., "3.13" -> "3.13.7")
        if let Ok(Some(matched_version)) = self.find_matching_version(tool_id, version) {
            let matched_path = self.config.tool_version_dir(tool_id, &matched_version)?;
            if matched_path.exists() {
                return Ok(matched_path);
            }
        }

        if tool_id == "java" {
            let legacy = self.config.versions_dir.join(version);
            if legacy.exists() {
                return Ok(legacy);
            }
        }

        Err(JcvmError::VersionNotFound(format!(
            "{}@{}",
            tool_id, version
        )))
    }

    fn cleanup_aliases(&self, tool_id: &str, install_dir: &Path) -> Result<()> {
        let current = self.config.tool_current_symlink(tool_id)?;
        if self.link_points_to(&current, install_dir)? {
            self.remove_link(&current)?;
        }

        if tool_id == "java" {
            let legacy_current = self.config.alias_dir.join("current");
            if self.link_points_to(&legacy_current, install_dir)? {
                self.remove_link(&legacy_current)?;
            }
        }

        let default = self.config.tool_default_symlink(tool_id)?;
        if self.link_points_to(&default, install_dir)? {
            self.remove_link(&default)?;
        }

        if tool_id == "java" {
            let legacy_default = self.config.alias_dir.join("default");
            if self.link_points_to(&legacy_default, install_dir)? {
                self.remove_link(&legacy_default)?;
            }
        }

        Ok(())
    }

    fn replace_symlink(&self, target: &Path, link: &Path) -> Result<()> {
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)?;
        }

        self.remove_link(link)?;

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)?;
        }

        #[cfg(windows)]
        {
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(target, link)?;
            } else {
                std::os::windows::fs::symlink_file(target, link)?;
            }
        }

        Ok(())
    }

    fn remove_link(&self, link: &Path) -> Result<()> {
        match fs::symlink_metadata(link) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || metadata.is_file() {
                    fs::remove_file(link)?;
                } else if metadata.is_dir() {
                    fs::remove_dir_all(link)?;
                }
                Ok(())
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn symlink_target(&self, link: &Path) -> Result<Option<PathBuf>> {
        match fs::symlink_metadata(link) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    Ok(Some(fs::read_link(link)?))
                } else if metadata.is_dir() {
                    Ok(Some(link.to_path_buf()))
                } else {
                    Ok(None)
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn link_points_to(&self, link: &Path, install_dir: &Path) -> Result<bool> {
        if let Some(target) = self.symlink_target(link)? {
            Ok(Self::target_matches(&target, install_dir))
        } else {
            Ok(false)
        }
    }

    fn target_matches(target: &Path, install_dir: &Path) -> bool {
        target == install_dir || target.starts_with(install_dir)
    }

    fn load_env(
        &self,
        plugin: &Arc<dyn ToolPlugin>,
        install_dir: &Path,
    ) -> Result<(PathBuf, Vec<(String, String)>)> {
        let install_path = install_dir.to_path_buf();
        let env = plugin.get_environment_vars(&install_path)?;
        let home = Self::extract_home_path(&env, install_dir);
        Ok((home, env))
    }

    fn extract_home_path(env: &[(String, String)], install_dir: &Path) -> PathBuf {
        for (key, value) in env.iter() {
            if key.ends_with("_HOME") {
                let cleaned = Self::sanitize_home_value(value);
                if !cleaned.is_empty() {
                    return PathBuf::from(cleaned);
                }
            }
        }
        install_dir.to_path_buf()
    }

    fn sanitize_home_value(value: &str) -> String {
        let mut cleaned = value.trim().to_string();

        for delimiter in ['$', '%'] {
            if let Some(idx) = cleaned.find(delimiter) {
                cleaned.truncate(idx);
            }
        }

        for separator in [':', ';'] {
            if let Some(idx) = Self::find_separator_index(&cleaned, separator) {
                cleaned.truncate(idx);
            }
        }

        cleaned = cleaned.trim().to_string();
        cleaned.trim_end_matches(['/', '\\']).trim().to_string()
    }

    fn find_separator_index(value: &str, separator: char) -> Option<usize> {
        if separator != ':' {
            return value.find(separator);
        }

        value
            .char_indices()
            .find(|(idx, ch)| *ch == ':' && !Self::is_drive_colon(value, *idx))
            .map(|(idx, _)| idx)
    }

    fn is_drive_colon(value: &str, idx: usize) -> bool {
        if idx != 1 {
            return false;
        }

        let bytes = value.as_bytes();
        matches!(bytes.first(), Some(b) if b.is_ascii_alphabetic())
            && matches!(bytes.get(2), Some(b'\\' | b'/'))
    }

    fn collect_legacy_java(
        &self,
        plugin: &Arc<dyn ToolPlugin>,
        seen: &mut HashSet<String>,
        current_links: &[PathBuf],
        default_links: &[PathBuf],
        results: &mut Vec<ManagedInstallation>,
    ) -> Result<()> {
        for entry in fs::read_dir(&self.config.versions_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(name.as_str(), "java" | "node" | "python") {
                continue;
            }

            if !seen.insert(name.clone()) {
                continue;
            }

            let version = match plugin.parse_version(&name) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let path = entry.path();
            let manifest = self.read_manifest(&path)?;
            let installed_at = manifest
                .as_ref()
                .map(|m| m.installed_at)
                .or_else(|| Self::metadata_timestamp(&path))
                .unwrap_or_else(Utc::now);

            let is_current = self.links_point_to(current_links, &path)?;
            let is_default = self.links_point_to(default_links, &path)?;

            results.push(ManagedInstallation {
                tool_id: "java".to_string(),
                version,
                path,
                is_current,
                is_default,
                installed_at,
                manifest,
            });
        }

        Ok(())
    }

    fn metadata_timestamp(path: &Path) -> Option<DateTime<Utc>> {
        fs::metadata(path)
            .ok()
            .and_then(|meta| meta.created().or_else(|_| meta.modified()).ok())
            .map(DateTime::<Utc>::from)
    }

    fn compare_versions_desc(a: &ToolVersion, b: &ToolVersion) -> Ordering {
        b.major
            .cmp(&a.major)
            .then(b.minor.unwrap_or(0).cmp(&a.minor.unwrap_or(0)))
            .then(b.patch.unwrap_or(0).cmp(&a.patch.unwrap_or(0)))
            .then(b.raw.cmp(&a.raw))
    }

    fn links_point_to(&self, links: &[PathBuf], install_dir: &Path) -> Result<bool> {
        for link in links {
            if self.link_points_to(link, install_dir)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sanitize_home_value_strips_env_and_path_segments() {
        let input = "/opt/jcvm/java/21/bin:$PATH";
        let sanitized = ToolManager::sanitize_home_value(input);
        assert_eq!(sanitized, "/opt/jcvm/java/21/bin".to_string());

        let input_win = "C:\\jcvm\\java\\21\\bin;%PATH%";
        let sanitized_win = ToolManager::sanitize_home_value(input_win);
        assert_eq!(sanitized_win, "C:\\jcvm\\java\\21\\bin".to_string());
    }

    #[test]
    fn test_version_comparison() {
        let v3_10_10 = ToolVersion::new("3.10.10".to_string(), 3, Some(10), Some(10));
        let v3_10_18 = ToolVersion::new("3.10.18".to_string(), 3, Some(10), Some(18));
        let v3_13_7 = ToolVersion::new("3.13.7".to_string(), 3, Some(13), Some(7));

        // compare_versions_desc compares b to a (b.cmp(a)), so it sorts descending
        // When v3_10_10 is 'a' and v3_10_18 is 'b', we expect Greater (b > a)
        assert_eq!(
            ToolManager::compare_versions_desc(&v3_10_10, &v3_10_18),
            Ordering::Greater
        );
        assert_eq!(
            ToolManager::compare_versions_desc(&v3_10_18, &v3_10_10),
            Ordering::Less
        );
        assert_eq!(
            ToolManager::compare_versions_desc(&v3_10_18, &v3_13_7),
            Ordering::Greater
        );
        assert_eq!(
            ToolManager::compare_versions_desc(&v3_10_10, &v3_10_10),
            Ordering::Equal
        );
    }

    #[tokio::test]
    async fn test_uninstall_fails_on_missing_manifest_and_invalid_version() {
        // Setup temporary directories
        let temp_dir = TempDir::new().unwrap();

        // Create a proper config with all directories
        let mut config = Config::default();
        config.jcvm_dir = temp_dir.path().to_path_buf();
        config.versions_dir = temp_dir.path().join("versions");
        config.alias_dir = temp_dir.path().join("alias");
        config.cache_dir = temp_dir.path().join("cache");

        std::fs::create_dir_all(&config.versions_dir).unwrap();
        std::fs::create_dir_all(&config.alias_dir).unwrap();
        std::fs::create_dir_all(&config.cache_dir).unwrap();

        // Load the builtin plugins (including Python)
        let registry = crate::plugins::load_builtin_plugins(&config).unwrap();
        let manager = ToolManager::new(config, registry);

        // Create an installation directory without a manifest
        let install_dir = manager
            .config
            .versions_dir
            .join("python")
            .join("invalid-version");
        std::fs::create_dir_all(&install_dir).unwrap();

        // Attempt to uninstall with an invalid version string
        // This should fail because:
        // 1. The manifest is missing
        // 2. The version string "invalid-version" cannot be parsed
        let result = manager.uninstall("python", "invalid-version").await;

        // Verify that the error is an InvalidToolStructure error
        assert!(result.is_err());
        match result.unwrap_err() {
            JcvmError::InvalidToolStructure { tool, message } => {
                assert_eq!(tool, "python");
                assert!(message.contains("manifest missing"));
                assert!(message.contains("version parsing failed"));
                assert!(message.contains("corrupted"));
            }
            other => panic!("Expected InvalidToolStructure error, got: {:?}", other),
        }
    }

    #[test]
    fn test_read_manifest_returns_none_for_missing_file() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = Config::default();
        config.versions_dir = temp_dir.path().join("versions");

        let manager = ToolManager::new(config, PluginRegistry::default());

        // Create a directory without a manifest
        let install_dir = temp_dir.path().join("test-install");
        std::fs::create_dir_all(&install_dir).unwrap();

        // read_manifest should return Ok(None) when manifest doesn't exist
        let result = manager.read_manifest(&install_dir);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}

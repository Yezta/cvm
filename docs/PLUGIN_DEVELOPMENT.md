# Plugin Development Guide

## Overview

JCVM (now a **Universal Version Manager**) has been architected to support version management for any tool, language, or application. This guide will help you create your own plugin to add support for new tools.

## Architecture

### Core Concepts

The plugin system is built around several key traits:

1. **ToolProvider** - Handles version discovery and metadata
2. **ToolInstaller** - Manages installation and removal
3. **ToolDetector** - Detects existing installations
4. **ToolPlugin** - Combines all three traits

### Directory Structure

```
src/
├── core/
│   ├── traits.rs      # Core trait definitions
│   ├── plugin.rs      # Plugin registry
│   └── mod.rs
├── plugins/
│   ├── java/          # Example: Java plugin
│   │   ├── api.rs
│   │   ├── installer.rs
│   │   ├── detector.rs
│   │   └── mod.rs
│   ├── nodejs/        # Your plugin here
│   ├── python/
│   └── mod.rs
```

## Creating a New Plugin

### Step 1: Create Plugin Directory Structure

```bash
mkdir -p src/plugins/my_tool
touch src/plugins/my_tool/mod.rs
touch src/plugins/my_tool/api.rs
touch src/plugins/my_tool/installer.rs
touch src/plugins/my_tool/detector.rs
```

### Step 2: Implement ToolProvider Trait

In `api.rs`, create a struct that implements version discovery:

```rust
use crate::core::traits::{
    Architecture, ToolDistribution, ToolVersion, Platform
};
use crate::error::Result;

pub struct MyToolApi {
    // HTTP client, API endpoints, etc.
}

impl MyToolApi {
    pub fn new() -> Self {
        Self { /* ... */ }
    }

    pub async fn list_available_versions(&self) -> Result<Vec<u32>> {
        // Call API to get available versions
        // Return list of version numbers
    }

    pub async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        // Find download URL for specific version/platform/arch
        // Return ToolDistribution with download details
    }
}
```

### Step 3: Implement ToolInstaller Trait

In `installer.rs`, create installation logic:

```rust
use crate::core::traits::{InstalledTool, ToolDistribution};
use crate::error::Result;
use std::path::PathBuf;
use async_trait::async_trait;

pub struct MyToolInstaller {
    // Downloader, etc.
}

impl MyToolInstaller {
    pub fn new() -> Self {
        Self { /* ... */ }
    }
}

#[async_trait]
impl MyToolInstaller {
    pub async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        // 1. Download the tool archive
        // 2. Extract to dest_dir
        // 3. Verify installation
        // 4. Return InstalledTool info
    }

    pub async fn uninstall(&self, installed: &InstalledTool) -> Result<()> {
        // Remove installation directory
        std::fs::remove_dir_all(&installed.path)?;
        Ok(())
    }

    pub async fn verify(&self, installed: &InstalledTool) -> Result<bool> {
        // Check if installation is still valid
        Ok(installed.path.exists())
    }
}
```

### Step 4: Implement ToolDetector Trait

In `detector.rs`, add logic to find existing installations:

```rust
use crate::core::traits::{DetectedInstallation, InstalledTool, ToolVersion};
use crate::error::Result;
use std::path::PathBuf;
use async_trait::async_trait;

pub struct MyToolDetector;

impl MyToolDetector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MyToolDetector {
    pub async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();

        // Check common installation paths
        // Check environment variables
        // Check system PATH
        
        // Return list of detected installations
        Ok(detected)
    }

    pub async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        // Create symlink or copy to managed location
        // Return InstalledTool info
    }
}
```

### Step 5: Create Plugin Module

In `mod.rs`, tie everything together:

```rust
mod api;
mod detector;
mod installer;

use crate::core::traits::{
    Architecture, DetectedInstallation, InstalledTool, Platform,
    PluginCategory, PluginMetadata, ToolDistribution, ToolInfo,
    ToolPlugin, ToolProvider, ToolVersion,
};
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

pub use api::MyToolApi;
pub use detector::MyToolDetector;
pub use installer::MyToolInstaller;

pub struct MyToolPlugin {
    api: MyToolApi,
    installer: MyToolInstaller,
    detector: MyToolDetector,
}

impl MyToolPlugin {
    pub fn new() -> Self {
        Self {
            api: MyToolApi::new(),
            installer: MyToolInstaller::new(),
            detector: MyToolDetector::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "mytool".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: "Your Name".to_string(),
            platforms: vec![Platform::Mac, Platform::Linux, Platform::Windows],
            architectures: vec![Architecture::X64, Architecture::Aarch64],
            category: PluginCategory::Tool, // or Language, Database, etc.
            builtin: false,
        }
    }
}

#[async_trait]
impl ToolProvider for MyToolPlugin {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: "mytool".to_string(),
            name: "My Tool".to_string(),
            description: "Description of my tool".to_string(),
            homepage: Some("https://mytool.org".to_string()),
            docs_url: Some("https://docs.mytool.org".to_string()),
        }
    }

    async fn list_remote_versions(&self, _lts_only: bool) -> Result<Vec<ToolVersion>> {
        self.api.list_available_versions().await
            .map(|versions| versions.into_iter()
                .map(|v| ToolVersion::new(v.to_string(), v, None, None))
                .collect())
    }

    async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        self.api.find_distribution(version, platform, arch).await
    }

    fn parse_version(&self, version_str: &str) -> Result<ToolVersion> {
        // Parse version string like "1.2.3"
        let parts: Vec<&str> = version_str.split('.').collect();
        let major = parts[0].parse().unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok());
        let patch = parts.get(2).and_then(|s| s.parse().ok());
        
        Ok(ToolVersion::new(version_str.to_string(), major, minor, patch))
    }

    fn validate_installation(&self, path: &PathBuf) -> Result<bool> {
        // Check if path contains valid installation
        Ok(path.join("bin/mytool").exists())
    }

    fn get_executable_paths(&self, install_path: &PathBuf) -> Result<Vec<PathBuf>> {
        Ok(vec![install_path.join("bin/mytool")])
    }

    fn get_environment_vars(&self, install_path: &PathBuf) -> Result<Vec<(String, String)>> {
        Ok(vec![
            ("MYTOOL_HOME".to_string(), install_path.display().to_string()),
            ("PATH".to_string(), format!("{}/bin:$PATH", install_path.display())),
        ])
    }
}

#[async_trait]
impl crate::core::traits::ToolInstaller for MyToolPlugin {
    async fn install(
        &self,
        distribution: &ToolDistribution,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        self.installer.install(distribution, dest_dir).await
    }

    async fn uninstall(&self, installed: &InstalledTool) -> Result<()> {
        self.installer.uninstall(installed).await
    }

    async fn verify(&self, installed: &InstalledTool) -> Result<bool> {
        self.installer.verify(installed).await
    }
}

#[async_trait]
impl crate::core::traits::ToolDetector for MyToolPlugin {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        self.detector.detect_installations().await
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        self.detector.import_installation(detected, dest_dir).await
    }
}

impl ToolPlugin for MyToolPlugin {
    fn supports_platform(&self, platform: Platform, arch: Architecture) -> bool {
        // Define which platforms/architectures you support
        matches!(platform, Platform::Mac | Platform::Linux | Platform::Windows)
            && matches!(arch, Architecture::X64 | Architecture::Aarch64)
    }
}
```

### Step 6: Register Your Plugin

Add your plugin to `src/plugins/mod.rs`:

```rust
pub mod java;
pub mod mytool; // Add your plugin here
```

### Step 7: Register Plugin at Startup

In your application initialization code:

```rust
use crate::core::plugin::PluginRegistry;
use crate::plugins::mytool::{MyToolPlugin};

let registry = PluginRegistry::new();
registry.register(
    Box::new(MyToolPlugin::new()),
    MyToolPlugin::metadata()
)?;
```

## Real-World Examples

### Example 1: Node.js Plugin

```rust
// Node.js uses nodejs.org API for releases
pub async fn list_available_versions(&self) -> Result<Vec<u32>> {
    let response = self.client
        .get("https://nodejs.org/dist/index.json")
        .send()
        .await?;
    
    let versions: Vec<NodeVersion> = response.json().await?;
    Ok(versions.iter()
        .map(|v| v.version.trim_start_matches('v')
            .split('.')
            .next()
            .unwrap()
            .parse()
            .unwrap())
        .collect())
}
```

### Example 2: Python Plugin

```rust
// Python uses python.org downloads
pub async fn find_distribution(
    &self,
    version: &ToolVersion,
    platform: Platform,
    arch: Architecture,
) -> Result<ToolDistribution> {
    let os_name = match platform {
        Platform::Mac => "macos",
        Platform::Linux => "linux",
        Platform::Windows => "windows",
    };
    
    let url = format!(
        "https://www.python.org/ftp/python/{}/{}-{}-{}.tar.gz",
        version.raw, version.raw, os_name, arch
    );
    
    Ok(ToolDistribution {
        tool_id: "python".to_string(),
        version: version.clone(),
        platform,
        architecture: arch,
        download_url: url,
        // ... other fields
    })
}
```

### Example 3: MongoDB Compass Plugin

```rust
// Compass is distributed via GitHub releases
pub async fn list_available_versions(&self) -> Result<Vec<ToolVersion>> {
    let response = self.client
        .get("https://api.github.com/repos/mongodb-js/compass/releases")
        .send()
        .await?;
    
    let releases: Vec<GithubRelease> = response.json().await?;
    Ok(releases.iter()
        .filter_map(|r| self.parse_version(&r.tag_name).ok())
        .collect())
}
```

## Best Practices

### 1. Error Handling
- Use proper error types from `crate::error::JcvmError`
- Provide descriptive error messages
- Handle network failures gracefully

### 2. Version Parsing
- Be flexible with version formats
- Handle different version schemes (semver, date-based, etc.)
- Validate version strings

### 3. Platform Support
- Clearly define which platforms/architectures you support
- Test on all supported platforms
- Handle platform-specific paths correctly

### 4. Installation
- Use the shared `Downloader` for consistency
- Verify checksums when available
- Extract to temporary directory first, then move
- Clean up on failure

### 5. Detection
- Check standard installation paths
- Respect environment variables
- Handle symlinks properly
- Deduplicate detected installations

### 6. Testing
- Write unit tests for version parsing
- Test installation/uninstallation
- Test detection on different systems
- Mock external APIs

## Plugin Categories

Choose the appropriate category for your plugin:

- **Language**: Programming languages (Java, Python, Ruby, Go)
- **Runtime**: Runtime environments (Node.js, Deno, Bun)
- **Database**: Database tools (MongoDB Compass, MySQL Workbench)
- **Tool**: Development tools (Git, Maven, Gradle)
- **Browser**: Web browsers (Chrome, Firefox)
- **Editor**: Code editors/IDEs (VS Code, IntelliJ)
- **Other**: Anything else

## UI Integration

Your plugin automatically supports UI integration through the API server. The UI can:

1. List available plugins: `GET /api/plugins`
2. Get plugin info: `GET /api/plugins/{tool_id}`
3. List versions: `GET /api/plugins/{tool_id}/versions`
4. Install version: `POST /api/plugins/{tool_id}/install`
5. Uninstall version: `DELETE /api/plugins/{tool_id}/versions/{version}`
6. Switch version: `POST /api/plugins/{tool_id}/use`

## User-Defined Plugins

Users can create custom plugins for any tool by:

1. Creating a plugin following this guide
2. Building it as a Rust library
3. Placing it in `~/.jcvm/plugins/`
4. Plugin will be auto-discovered and loaded

Alternatively, plugins can be defined via configuration:

```toml
# ~/.jcvm/config.toml
[[plugins]]
id = "my-custom-tool"
name = "My Custom Tool"
download_url_pattern = "https://example.com/releases/{version}/tool-{platform}-{arch}.tar.gz"
executable_path = "bin/mytool"
version_pattern = "\\d+\\.\\d+\\.\\d+"
```

## Future Enhancements

Planned features:

- **Plugin Marketplace**: Share plugins with the community
- **Auto-Update**: Plugins can self-update
- **Dependencies**: Plugins can depend on other tools
- **Hooks**: Pre/post install hooks
- **Custom Commands**: Plugins can add custom CLI commands

## Getting Help

- Check existing plugins in `src/plugins/` for examples
- Read the trait documentation in `src/core/traits.rs`
- Open an issue on GitHub for questions
- Join our Discord community

## Contributing

To contribute your plugin:

1. Fork the repository
2. Create your plugin following this guide
3. Add tests
4. Update documentation
5. Submit a pull request

Thank you for extending JCVM! 🚀

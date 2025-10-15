# Quick Start: Creating a Node.js Plugin

This is a condensed guide showing exactly how to create a Node.js plugin for JCVM in under 2 hours.

## Step 1: Create Directory Structure (2 minutes)

```bash
mkdir -p src/plugins/nodejs
touch src/plugins/nodejs/mod.rs
touch src/plugins/nodejs/api.rs
touch src/plugins/nodejs/installer.rs
touch src/plugins/nodejs/detector.rs
```

## Step 2: API Client (30 minutes)

**File**: `src/plugins/nodejs/api.rs`

```rust
use crate::core::traits::{Architecture, ArchiveType, Platform, ToolDistribution, ToolVersion};
use crate::error::{JcvmError, Result};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

const NODEJS_API: &str = "https://nodejs.org/dist/index.json";

#[derive(Debug, Deserialize)]
struct NodeRelease {
    version: String,
    lts: serde_json::Value,
}

pub struct NodeJsApi {
    client: Client,
}

impl NodeJsApi {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent(concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap(),
        }
    }

    pub async fn list_available_versions(&self) -> Result<Vec<u32>> {
        let releases: Vec<NodeRelease> = self.client
            .get(NODEJS_API)
            .send()
            .await?
            .json()
            .await?;

        let versions: Vec<u32> = releases
            .iter()
            .map(|r| r.version.trim_start_matches('v').split('.').next().unwrap().parse().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(versions)
    }

    pub async fn list_lts_versions(&self) -> Result<Vec<u32>> {
        let releases: Vec<NodeRelease> = self.client
            .get(NODEJS_API)
            .send()
            .await?
            .json()
            .await?;

        let versions: Vec<u32> = releases
            .iter()
            .filter(|r| !r.lts.is_null())
            .map(|r| r.version.trim_start_matches('v').split('.').next().unwrap().parse().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(versions)
    }

    pub async fn find_distribution(
        &self,
        version: &ToolVersion,
        platform: Platform,
        arch: Architecture,
    ) -> Result<ToolDistribution> {
        let os = match platform {
            Platform::Mac => "darwin",
            Platform::Linux => "linux",
            Platform::Windows => "win",
        };

        let arch_str = match arch {
            Architecture::X64 => "x64",
            Architecture::Aarch64 => "arm64",
            _ => "x64",
        };

        let extension = if platform == Platform::Windows { "zip" } else { "tar.gz" };
        
        let url = format!(
            "https://nodejs.org/dist/v{}/node-v{}-{}-{}.{}",
            version.raw, version.raw, os, arch_str, extension
        );

        Ok(ToolDistribution {
            tool_id: "node".to_string(),
            version: version.clone(),
            platform,
            architecture: arch,
            download_url: url,
            checksum: None,
            size: None,
            archive_type: if extension == "zip" { ArchiveType::Zip } else { ArchiveType::TarGz },
            metadata: HashMap::new(),
        })
    }
}
```

## Step 3: Installer (20 minutes)

**File**: `src/plugins/nodejs/installer.rs`

Copy from `src/plugins/java/installer.rs` and change:
- Replace "java" → "node"
- Replace "JDK" → "Node.js"
- Update executable checks: `bin/java` → `bin/node`

(The installer logic is generic and reusable!)

## Step 4: Detector (30 minutes)

**File**: `src/plugins/nodejs/detector.rs`

```rust
use crate::core::traits::{DetectedInstallation, InstalledTool, ToolVersion};
use crate::error::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct NodeJsDetector;

impl NodeJsDetector {
    pub fn new() -> Self {
        Self
    }

    fn verify_node_home(&self, path: &Path) -> Result<Option<DetectedInstallation>> {
        let node_bin = if path.join("bin/node").exists() {
            path.join("bin/node")
        } else if path.join("node.exe").exists() {
            path.join("node.exe")
        } else {
            return Ok(None);
        };

        let output = match Command::new(&node_bin).arg("--version").output() {
            Ok(o) => o,
            Err(_) => return Ok(None),
        };

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim().trim_start_matches('v');
        
        let parts: Vec<&str> = version_str.split('.').collect();
        if let Some(major_str) = parts.get(0) {
            if let Ok(major) = major_str.parse::<u32>() {
                let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
                let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());
                
                let is_lts = matches!(major, 14 | 16 | 18 | 20);
                
                return Ok(Some(DetectedInstallation {
                    tool_id: "node".to_string(),
                    version: ToolVersion::new(version_str.to_string(), major, minor, patch).with_lts(is_lts),
                    path: path.to_path_buf(),
                    source: "detected".to_string(),
                    executable_path: Some(node_bin),
                }));
            }
        }

        Ok(None)
    }

    fn check_common_paths(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();
        let paths = if cfg!(windows) {
            vec!["C:\\Program Files\\nodejs", "C:\\Program Files (x86)\\nodejs"]
        } else {
            vec!["/usr/local/bin", "/opt/nodejs", "/usr/lib/nodejs"]
        };

        for path_str in paths {
            let path = PathBuf::from(path_str);
            if let Ok(Some(installation)) = self.verify_node_home(&path) {
                detected.push(installation);
            }
        }
        Ok(detected)
    }
}

#[async_trait]
impl crate::core::traits::ToolDetector for NodeJsDetector {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        let mut detected = Vec::new();
        detected.extend(self.check_common_paths()?);
        detected.sort_by(|a, b| a.path.cmp(&b.path));
        detected.dedup_by(|a, b| a.path == b.path);
        Ok(detected)
    }

    async fn import_installation(
        &self,
        detected: &DetectedInstallation,
        dest_dir: &PathBuf,
    ) -> Result<InstalledTool> {
        #[cfg(unix)]
        std::os::unix::fs::symlink(&detected.path, dest_dir)?;
        
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&detected.path, dest_dir)?;

        Ok(InstalledTool {
            tool_id: "node".to_string(),
            version: detected.version.clone(),
            path: dest_dir.clone(),
            installed_at: chrono::Utc::now(),
            source: detected.source.clone(),
            executable_path: detected.executable_path.clone(),
        })
    }
}
```

## Step 5: Plugin Module (15 minutes)

**File**: `src/plugins/nodejs/mod.rs`

```rust
mod api;
mod detector;
mod installer;

use crate::core::traits::*;
use crate::error::Result;
use async_trait::async_trait;
use std::path::PathBuf;

pub use api::NodeJsApi;
pub use detector::NodeJsDetector;
pub use installer::NodeJsInstaller;

pub struct NodeJsPlugin {
    api: NodeJsApi,
    installer: NodeJsInstaller,
    detector: NodeJsDetector,
}

impl NodeJsPlugin {
    pub fn new() -> Self {
        Self {
            api: NodeJsApi::new(),
            installer: NodeJsInstaller::new(),
            detector: NodeJsDetector::new(),
        }
    }

    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            id: "node".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            author: "JCVM Contributors".to_string(),
            platforms: vec![Platform::Mac, Platform::Linux, Platform::Windows],
            architectures: vec![Architecture::X64, Architecture::Aarch64],
            category: PluginCategory::Runtime,
            builtin: true,
        }
    }
}

#[async_trait]
impl ToolProvider for NodeJsPlugin {
    fn info(&self) -> ToolInfo {
        ToolInfo {
            id: "node".to_string(),
            name: "Node.js".to_string(),
            description: "Node.js JavaScript runtime".to_string(),
            homepage: Some("https://nodejs.org".to_string()),
            docs_url: Some("https://nodejs.org/docs".to_string()),
        }
    }

    async fn list_remote_versions(&self, lts_only: bool) -> Result<Vec<ToolVersion>> {
        if lts_only {
            let versions = self.api.list_lts_versions().await?;
            Ok(versions.into_iter()
                .map(|major| ToolVersion::new(major.to_string(), major, None, None).with_lts(true))
                .collect())
        } else {
            let versions = self.api.list_available_versions().await?;
            Ok(versions.into_iter()
                .map(|major| {
                    let is_lts = matches!(major, 14 | 16 | 18 | 20);
                    ToolVersion::new(major.to_string(), major, None, None).with_lts(is_lts)
                })
                .collect())
        }
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
        let cleaned = version_str.trim_start_matches('v');
        let parts: Vec<&str> = cleaned.split('.').collect();
        
        let major = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok());
        let patch = parts.get(2).and_then(|s| s.parse().ok());
        
        Ok(ToolVersion::new(cleaned.to_string(), major, minor, patch))
    }

    fn validate_installation(&self, path: &PathBuf) -> Result<bool> {
        Ok(path.join("bin/node").exists() || path.join("node.exe").exists())
    }

    fn get_executable_paths(&self, install_path: &PathBuf) -> Result<Vec<PathBuf>> {
        if cfg!(windows) {
            Ok(vec![install_path.join("node.exe"), install_path.join("npm.cmd")])
        } else {
            Ok(vec![
                install_path.join("bin/node"),
                install_path.join("bin/npm"),
            ])
        }
    }

    fn get_environment_vars(&self, install_path: &PathBuf) -> Result<Vec<(String, String)>> {
        Ok(vec![
            ("NODE_HOME".to_string(), install_path.display().to_string()),
            ("PATH".to_string(), format!("{}/bin:$PATH", install_path.display())),
        ])
    }
}

#[async_trait]
impl ToolInstaller for NodeJsPlugin {
    async fn install(&self, distribution: &ToolDistribution, dest_dir: &PathBuf) -> Result<InstalledTool> {
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
impl ToolDetector for NodeJsPlugin {
    async fn detect_installations(&self) -> Result<Vec<DetectedInstallation>> {
        self.detector.detect_installations().await
    }

    async fn import_installation(&self, detected: &DetectedInstallation, dest_dir: &PathBuf) -> Result<InstalledTool> {
        self.detector.import_installation(detected, dest_dir).await
    }
}

impl ToolPlugin for NodeJsPlugin {
    fn supports_platform(&self, platform: Platform, arch: Architecture) -> bool {
        matches!(
            (platform, arch),
            (Platform::Mac, Architecture::X64)
                | (Platform::Mac, Architecture::Aarch64)
                | (Platform::Linux, Architecture::X64)
                | (Platform::Linux, Architecture::Aarch64)
                | (Platform::Windows, Architecture::X64)
        )
    }
}
```

## Step 6: Register Plugin (5 minutes)

**File**: `src/plugins/mod.rs`

```rust
pub mod java;
pub mod nodejs;  // Add this line
```

## Done! ✅

You now have a fully functional Node.js plugin for JCVM!

## Usage

```bash
# List Node.js versions
jcvm list-remote node

# Install Node.js 20
jcvm install node 20

# Use Node.js 20
jcvm use node 20

# Set for project
jcvm local node 20
```

## Time Breakdown

- API Client: 30 minutes
- Installer: 20 minutes (mostly copy-paste)
- Detector: 30 minutes
- Main Plugin: 15 minutes
- Registration: 5 minutes
- Testing: 20 minutes

**Total: ~2 hours**

## Key Insights

1. **Most code is reusable**: The installer and much of the detector can be copied from Java plugin
2. **Only API is tool-specific**: Each tool has its own API/download pattern
3. **Traits guide implementation**: Following the traits ensures completeness
4. **Pattern is consistent**: Once you've done one, others are quick

## Next: Python Plugin

Python would be similar but with:
- API: `https://www.python.org/ftp/python/`
- Executable: `python` or `python3`
- Detection paths: `/usr/bin/python`, `/usr/local/bin/python3`

Estimated time: **2 hours**

## Next: Compass Plugin

Compass from GitHub releases:
- API: `https://api.github.com/repos/mongodb-js/compass/releases`
- Application bundle on macOS
- Executable location varies by platform

Estimated time: **3 hours** (more complex due to app bundle handling)

---

**The plugin architecture makes adding new tools incredibly fast and consistent!**

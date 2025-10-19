use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// JDK version representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub build: Option<String>,
}

impl Version {
    /// Create a version with a major component only
    #[allow(dead_code)]
    pub fn new(major: u32) -> Self {
        Self {
            major,
            minor: None,
            patch: None,
            build: None,
        }
    }

    /// Create a version with explicit minor component
    #[allow(dead_code)]
    pub fn with_minor(major: u32, minor: u32) -> Self {
        let mut version = Self::new(major);
        version.minor = Some(minor);
        version
    }

    /// Create a version with explicit minor and patch components
    #[allow(dead_code)]
    pub fn with_patch(major: u32, minor: u32, patch: u32) -> Self {
        let mut version = Self::with_minor(major, minor);
        version.patch = Some(patch);
        version
    }

    /// Attach build metadata to the version
    #[allow(dead_code)]
    pub fn with_build<T: Into<String>>(mut self, build: T) -> Self {
        let build = build.into();
        if build.is_empty() {
            self.build = None;
        } else {
            self.build = Some(build);
        }
        self
    }

    /// Check if this version description matches a concrete version
    #[allow(dead_code)]
    pub fn matches(&self, other: &Self) -> bool {
        if self.major != other.major {
            return false;
        }
        if let Some(minor) = self.minor {
            if other.minor != Some(minor) {
                return false;
            }
        }
        if let Some(patch) = self.patch {
            if other.patch != Some(patch) {
                return false;
            }
        }
        if let Some(build) = &self.build {
            return other
                .build
                .as_deref()
                .map(|candidate| candidate == build)
                .unwrap_or(false);
        }
        true
    }

    #[allow(dead_code)]
    pub fn is_lts(&self) -> bool {
        matches!(self.major, 8 | 11 | 17 | 21)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(minor) = self.minor {
            write!(f, ".{}", minor)?;
        }
        if let Some(patch) = self.patch {
            write!(f, ".{}", patch)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl FromStr for Version {
    type Err = crate::error::JcvmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        let (version_part, build_part) = match trimmed.split_once('+') {
            Some((version, build)) => (version, Some(build.trim().to_string())),
            None => (trimmed, None),
        };

        let parts: Vec<&str> = version_part.split('.').collect();

        let major = parts.first()
            .and_then(|p| p.trim().parse::<u32>().ok())
            .ok_or_else(|| crate::error::JcvmError::InvalidVersion(s.to_string()))?;

        let minor = parts.get(1).and_then(|p| p.trim().parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.trim().parse::<u32>().ok());

        let build = build_part.filter(|b| !b.is_empty());

        Ok(Version {
            major,
            minor,
            patch,
            build,
        })
    }
}

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Mac,
    Linux,
    Windows,
}

impl Platform {
    #[allow(dead_code)]
    pub fn current() -> crate::error::Result<Self> {
        match std::env::consts::OS {
            "macos" => Ok(Platform::Mac),
            "linux" => Ok(Platform::Linux),
            "windows" => Ok(Platform::Windows),
            os => Err(crate::error::JcvmError::UnsupportedPlatform {
                os: os.to_string(),
                arch: Architecture::current()?.to_string(),
            }),
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            Platform::Mac => "mac",
            Platform::Linux => "linux",
            Platform::Windows => "windows",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Supported architectures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    X64,
    Aarch64,
}

impl Architecture {
    #[allow(dead_code)]
    pub fn current() -> crate::error::Result<Self> {
        match std::env::consts::ARCH {
            "x86_64" | "amd64" => Ok(Architecture::X64),
            "aarch64" | "arm64" => Ok(Architecture::Aarch64),
            arch => Err(crate::error::JcvmError::UnsupportedPlatform {
                os: Platform::current()?.to_string(),
                arch: arch.to_string(),
            }),
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        match self {
            Architecture::X64 => "x64",
            Architecture::Aarch64 => "aarch64",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// JDK distribution information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JdkDistribution {
    pub version: Version,
    pub platform: Platform,
    pub architecture: Architecture,
    pub download_url: String,
    pub checksum: Option<String>,
    pub size: Option<u64>,
}

/// Installed JDK information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledJdk {
    pub version: Version,
    pub path: std::path::PathBuf,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub distribution: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = "21".parse::<Version>().unwrap();
        assert_eq!(v.major, 21);
        assert_eq!(v.minor, None);

        let v = "17.0.10".parse::<Version>().unwrap();
        assert_eq!(v.major, 17);
        assert_eq!(v.minor, Some(0));
        assert_eq!(v.patch, Some(10));

        let v = "17.0.2+8".parse::<Version>().unwrap();
        assert_eq!(v.major, 17);
        assert_eq!(v.minor, Some(0));
        assert_eq!(v.patch, Some(2));
        assert_eq!(v.build.as_deref(), Some("8"));
    }

    #[test]
    fn test_version_matching() {
        let v1 = Version::new(21);
        let v2 = Version::with_patch(21, 0, 1);

        assert!(v1.matches(&v2));
        assert!(!v2.matches(&v1));

        let build_specific = Version::with_patch(21, 0, 1).with_build("10");
        let with_build = Version::new(21).with_build("10");
        assert!(Version::new(21).matches(&build_specific));
        assert!(with_build.matches(&build_specific));
        assert!(!build_specific.matches(&with_build));
    }

    #[test]
    fn test_lts_versions() {
        assert!(Version::new(21).is_lts());
        assert!(Version::new(17).is_lts());
        assert!(Version::new(11).is_lts());
        assert!(!Version::new(20).is_lts());
    }
}

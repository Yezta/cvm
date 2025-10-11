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
        let parts: Vec<&str> = s.split('.').collect();
        
        let major = parts
            .get(0)
            .and_then(|p| p.parse::<u32>().ok())
            .ok_or_else(|| crate::error::JcvmError::InvalidVersion(s.to_string()))?;

        let minor = parts.get(1).and_then(|p| p.parse::<u32>().ok());
        let patch = parts.get(2).and_then(|p| p.parse::<u32>().ok());

        Ok(Version {
            major,
            minor,
            patch,
            build: None,
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
    }

    #[test]
    fn test_version_matching() {
        let v1 = Version::new(21);
        let v2 = Version::with_patch(21, 0, 1);
        
        assert!(v1.matches(&v2));
        assert!(!v2.matches(&v1));
    }

    #[test]
    fn test_lts_versions() {
        assert!(Version::new(21).is_lts());
        assert!(Version::new(17).is_lts());
        assert!(Version::new(11).is_lts());
        assert!(!Version::new(20).is_lts());
    }
}

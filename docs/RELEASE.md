# JCVM Release Guide

## Overview

JCVM uses GitHub Actions to automatically build and publish releases for multiple platforms (macOS Intel & Apple Silicon, Linux x86_64 & ARM64, Windows x86_64). This guide covers everything you need to know about creating releases, from setup to post-release monitoring.

## Table of Contents

- [Quick Reference](#quick-reference)
- [Release Workflow](#release-workflow)
- [Pre-Release Checklist](#pre-release-checklist)
- [Version Numbering](#version-numbering)
- [Platform Support](#platform-support)
- [Installation Methods](#installation-methods)
- [Release Infrastructure](#release-infrastructure)
- [Troubleshooting](#troubleshooting)
- [Post-Release](#post-release)

## Quick Reference

### Pre-Release Commands

```bash
# 1. Update version
echo "2.0.1" > VERSION

# 2. Update Cargo.toml version
# Edit Cargo.toml: version = "2.0.1"

# 3. Update CHANGELOG.md
# Add release notes for v2.0.1

# 4. Test locally
cargo test --all
cargo build --release
./target/release/jcvm --version

# 5. Commit and push
git add VERSION Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to 2.0.1"
git push origin main
```

### Quick Installation Commands (For Users)

```bash
# Quick install (pre-built binary)
curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/install-binary.sh | bash

# Build from source
curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/install.sh | bash

# Custom install directory
INSTALL_DIR=/usr/local/bin bash install-binary.sh

# Specific version
VERSION=1.0.0 bash install-binary.sh
```

## Release Workflow

### Automatic Release on Main Branch (Recommended)

When code is merged to the `main` branch:

1. **CI Tests Run**: All tests must pass on multiple platforms
2. **Version Check**: The version is read from the `VERSION` file
3. **Release Creation**: A GitHub release is automatically created with the version tag
4. **Binary Building**: Release binaries are built for all supported platforms
5. **Asset Upload**: All binaries and checksums are uploaded to the release
6. **Documentation**: Release notes are auto-generated with installation instructions

### Manual Release Trigger

You can also trigger a release manually:

1. Go to the **Actions** tab in GitHub
2. Select the **Release** workflow
3. Click **Run workflow**
4. Optionally specify a custom version (otherwise uses `VERSION` file)
5. Click "Run workflow" button

## Pre-Release Checklist

Before merging to `main` for a release:

- [ ] Update `VERSION` file with the new version number (e.g., `1.0.1`)
- [ ] Update `CHANGELOG.md` with release notes following [Keep a Changelog](https://keepachangelog.com/) format
- [ ] Update version in `Cargo.toml`
- [ ] Run local tests: `cargo test --all`
- [ ] Run local build: `cargo build --release`
- [ ] Test the binary locally: `./target/release/jcvm --version`
- [ ] Ensure all CI checks pass on your PR
- [ ] Review and merge PR to `main`

## Version Numbering

JCVM follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH
  │     │      │
  │     │      └─ Bug fixes (2.0.0 → 2.0.1)
  │     └──────── New features, backward compatible (2.0.0 → 2.1.0)
  └────────────── Breaking changes, incompatible API changes (1.0.0 → 2.0.0)
```

### Examples

- **PATCH**: Bug fix, documentation update → `1.0.0` → `1.0.1`
- **MINOR**: New plugin, new command, new feature → `1.0.0` → `1.1.0`
- **MAJOR**: CLI restructure, removed commands, API changes → `1.0.0` → `2.0.0`

## Platform Support

| Platform | Target Triple | Archive Format |
|----------|--------------|----------------|
| macOS (Apple Silicon) | `aarch64-apple-darwin` | `.tar.gz` |
| macOS (Intel) | `x86_64-apple-darwin` | `.tar.gz` |
| Linux (x86_64) | `x86_64-unknown-linux-gnu` | `.tar.gz` |
| Linux (ARM64) | `aarch64-unknown-linux-gnu` | `.tar.gz` |
| Windows (x86_64) | `x86_64-pc-windows-msvc` | `.zip` |

## Release Artifacts

Each release includes:

### Binary Archives

Each platform gets an archive with the following naming pattern:
- `jcvm-v{VERSION}-{TARGET}.tar.gz` (Unix)
- `jcvm-v{VERSION}-{TARGET}.zip` (Windows)

### Checksums

Each archive has a corresponding `.sha256` file for verification.

### Archive Contents

```
jcvm-v2.0.1-x86_64-apple-darwin/
├── jcvm                    # The binary executable
├── README.md               # Project documentation
├── LICENSE                 # MIT license
├── CHANGELOG.md            # Version history
└── install.sh              # Local installation script
```

## Installation Methods

### Method 1: Quick Install Script (Recommended)

**For pre-built binaries:**

```bash
curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/install-binary.sh | bash
```

**For building from source:**

```bash
curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/install.sh | bash
```

### Method 2: Manual Download

1. Visit the [releases page](https://github.com/Yezta/cvm/releases)
2. Download the appropriate archive for your platform
3. Extract the archive: `tar xzf jcvm-v*.tar.gz` (or unzip on Windows)
4. Run the included `install.sh` (or `install.bat` on Windows)

### Method 3: Build from Source

```bash
git clone https://github.com/Yezta/cvm.git
cd jcvm
cargo build --release
cp target/release/jcvm ~/.local/bin/
```

### Installation Verification

```bash
# Verify installation
jcvm --version

# Test basic functionality
jcvm list-remote --tool java --lts
```

## Release Infrastructure

### GitHub Actions Workflows

#### `.github/workflows/release.yml`

Handles the automated release process:

- Triggers on push to `main` or manual workflow dispatch
- Creates GitHub release with version tag from `VERSION` file
- Builds binaries for all 5 supported platforms
- Generates SHA256 checksums for all artifacts
- Uploads release artifacts and checksums
- Includes auto-generated installation instructions

#### `.github/workflows/ci.yml`

Runs continuous integration:

- Triggers on push and pull requests
- Runs tests on Ubuntu, macOS, and Windows
- Performs code quality checks (rustfmt, clippy)
- Runs security audits (cargo-audit)
- Generates code coverage reports
- Runs integration tests

### Installation Scripts

#### `install-binary.sh`

Downloads pre-built binaries from GitHub releases:

- Auto-detects user's platform (OS + architecture)
- Downloads the appropriate binary archive
- Installs to `~/.local/bin` by default (customizable via `INSTALL_DIR`)
- Provides SHA256 checksum verification capability
- User-friendly with colored output and error handling
- Supports version selection via `VERSION` environment variable

#### `install.sh`

Builds from source using Cargo:

- For users who prefer/need to compile locally
- Requires Rust toolchain to be installed
- Builds in release mode with optimizations

## Troubleshooting

### Release Failed to Create

**Issue**: GitHub Actions workflow fails to create release

**Solutions**:
- Check that the version in `VERSION` doesn't already have a git tag
- Verify all required secrets are configured in GitHub repository settings
- Review the GitHub Actions logs for specific error messages
- Ensure the `VERSION` file contains a valid semantic version number

```bash
# Delete existing tag if needed
git tag -d v2.0.1
git push origin :refs/tags/v2.0.1

# Then re-run the release workflow
```

### Binary Build Failed

**Issue**: Build fails for specific platform

**Solutions**:
- Check the build logs in GitHub Actions for the failing platform
- Ensure all dependencies are properly specified in `Cargo.toml`
- Verify cross-compilation setup is correct
- Test the build locally for the specific platform

```bash
# Test local build for a specific target
cargo build --release --target x86_64-unknown-linux-gnu
```

### Installation Script Doesn't Work

**Issue**: Users report installation script failures

**Solutions**:
- Verify the GitHub repository is public or user has access
- Check that release assets were properly uploaded
- Ensure archive naming matches the expected pattern
- Verify download URLs are correct

```bash
# Test release availability
curl -I https://github.com/Yezta/cvm/releases/latest

# Check specific download URL
curl -I https://github.com/Yezta/cvm/releases/download/v1.0.0/jcvm-v1.0.0-x86_64-apple-darwin.tar.gz
```

### Checksum Verification Failed

**Issue**: Checksum doesn't match downloaded file

**Solutions**:
- Re-download the file (might have been corrupted during transfer)
- Verify the checksum file was uploaded correctly
- Check network stability
- Try a different mirror or CDN (if applicable)

## Post-Release

After a successful release:

1. **Verify the Release**
   - Check the [releases page](https://github.com/Yezta/cvm/releases)
   - Verify all platform artifacts are present
   - Test download links

2. **Test Installation**
   - Test on different platforms (macOS, Linux, Windows if possible)
   - Verify the installation script works correctly
   - Check that the binary runs and shows correct version

3. **Update Documentation**
   - Ensure README reflects any new features
   - Update any version-specific documentation
   - Check that all links work correctly

4. **Announce the Release**
   - Update project website (if applicable)
   - Social media announcements
   - Notify users via GitHub Discussions or mailing list
   - Update any package managers (Homebrew, etc.)

5. **Monitor**
   - Watch for issue reports
   - Monitor download statistics
   - Respond to user feedback

## Environment Variables

The installation scripts support these environment variables:

- `INSTALL_DIR` - Installation directory (default: `~/.local/bin`)
- `VERSION` - Specific version to install (default: `latest`)
- `GITHUB_REPO` - GitHub repository (default: `Yezta/cvm`)

**Examples:**

```bash
# Custom installation directory
INSTALL_DIR=/usr/local/bin ./install-binary.sh

# Install specific version
VERSION=1.0.0 ./install-binary.sh

# Combine both
INSTALL_DIR=/opt/bin VERSION=1.0.0 ./install-binary.sh
```

## Security

- **Build Transparency**: All release binaries are built in GitHub Actions with public audit logs
- **Checksums**: SHA256 checksums are provided for all artifacts to verify integrity
- **Dependency Auditing**: Dependencies are automatically audited with `cargo audit` in CI
- **Code Scanning**: Code is scanned for security issues before release
- **Supply Chain**: Minimal dependencies, all from crates.io with version pinning

## Monitoring & Analytics

Track release health using:

- **Download Counts**: Per-platform statistics in GitHub Insights
- **Build Times**: Success rates and duration in GitHub Actions
- **Test Coverage**: Code coverage trends (Codecov integration available)
- **Security Vulnerabilities**: Automated scanning via cargo-audit
- **User Adoption**: Version distribution from telemetry (if implemented)

## First-Time Release Setup

Before your very first release:

1. **Update Repository URLs**
   
   Replace `Yezta/cvm` with your actual GitHub username/org in:
   - `.github/workflows/release.yml`
   - `install-binary.sh`
   - `install.sh`
   - `Cargo.toml`
   - This documentation file
   - `README.md`

2. **Set Initial Version**
   
   ```bash
   echo "1.0.0" > VERSION
   # Also update in Cargo.toml: version = "1.0.0"
   ```

3. **Test Locally**
   
   ```bash
   cargo build --release
   cargo test --all
   ./target/release/jcvm --version
   ```

4. **Push to GitHub**
   
   ```bash
   git add .
   git commit -m "chore: prepare for first release"
   git push origin main
   ```

5. **Enable Branch Protection** (Recommended)
   
   - Go to Settings → Branches
   - Add rule for `main` branch
   - Require status checks to pass before merging
   - Require pull request reviews

## Support

For issues with releases:

- **Issues**: [GitHub Issues](https://github.com/Yezta/cvm/issues)
- **Documentation**: [Project README](https://github.com/Yezta/cvm)
- **Discussions**: [GitHub Discussions](https://github.com/Yezta/cvm/discussions)

When reporting a release issue, please include:
- Platform and architecture (e.g., macOS Apple Silicon, Linux x86_64)
- Version attempting to install
- Full error message
- Steps to reproduce

## Additional Resources

- **GitHub Actions Documentation**: https://docs.github.com/en/actions
- **Semantic Versioning Spec**: https://semver.org/
- **Keep a Changelog**: https://keepachangelog.com/
- **Rust Cross-Compilation**: https://rust-lang.github.io/rustup/cross-compilation.html
- **Version Management Guide**: [docs/VERSION_MANAGEMENT.md](VERSION_MANAGEMENT.md)
- **Architecture Documentation**: [docs/ARCHITECTURE.md](ARCHITECTURE.md)

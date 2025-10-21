# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-10-10

### Added

- Initial release of JCVM (Java Configuration & Version Manager)
- Install JDK versions from Eclipse Temurin (Adoptium)
- List available remote JDK versions
- List installed local JDK versions
- Switch between installed JDK versions
- Auto-switch based on `.java-version` file in project directories
- Set global default JDK version using aliases
- Set local project-specific JDK version
- Uninstall JDK versions
- Automatic JAVA_HOME and PATH management
- Support for macOS and Linux
- Color-coded terminal output
- Interactive installation script
- Comprehensive documentation

### Features

- ✅ NVM-like command interface
- ✅ Project-specific version management with `.java-version`
- ✅ Automatic version switching on directory change
- ✅ Support for LTS versions marking
- ✅ Clean uninstall process
- ✅ Shell integration for bash and zsh

### Documentation

- Complete README with usage examples
- Installation guide
- Contributing guidelines
- License (MIT)

## [Unreleased]

### Changed - Rust Rewrite

**Major Rewrite**: Complete rewrite of JCVM from Shell script to Rust for better performance, reliability, and maintainability.

#### New Features

- New Rust-based implementation with Cargo build system
- Modular architecture with separate modules:
  - `api.rs` - API client for JDK distribution sources
  - `cli.rs` - Command-line interface and argument parsing
  - `config.rs` - Configuration management
  - `detect.rs` - Automatic version detection
  - `download.rs` - Download functionality
  - `error.rs` - Error handling
  - `install.rs` - Installation logic
  - `models.rs` - Data models
  - `shell.rs` - Shell integration
  - `utils.rs` - Utility functions
  - `version_manager.rs` - Core version management
- New test shell integration script (`test-shell-integration.sh`)
- Cargo configuration (`Cargo.toml`)
- Project configuration file (`config.toml`)
- GitHub Copilot instructions (`.github/copilot-instructions.md`)

#### Files Removed

- Legacy shell script implementation (`jcvm.sh`)
- Redundant documentation files:
  - `ARCHITECTURE.md`
  - `FAQ.md`
  - `GET_STARTED.md`
  - `PROJECT_SUMMARY.md`
  - `QUICKSTART.md`
  - `examples/README.md`
- Legacy test scripts (`test.sh`, `show-structure.sh`)

#### Files Modified

- Updated `.gitignore` for Rust project structure
- Enhanced `README.md` with Rust-specific information
- Updated `TESTING.md` for Rust testing approach
- Improved `install.sh` for Rust binary installation

#### Technical Improvements

- Better error handling with Rust's type system
- Improved performance with compiled binary
- Enhanced code maintainability and testability
- Type-safe configuration and API interactions
- Cross-platform compatibility improvements

### Planned

- Support for additional JDK distributions (Oracle, Corretto, Azul Zulu, GraalVM)
- Windows support
- Integration with build tools (Maven, Gradle)
- Version migration tools
- Enhanced error handling and validation
- Update command to upgrade JCVM itself
- Cache management commands
- Checksum verification for downloads

## [2.1.0] - 2025-10-21

### ✨ Features

- feat: streamline GitHub Actions workflows for version management and releases (c8e8499)
- feat: enhance automated workflows for versioning and releases (54266b8)

### 🐛 Bug Fixes

- fix: simplify Cargo.toml update process in version bump workflow (b21bf2a)
- fix: improve comments for clarity in version bump workflow and CLI tool commands (d8e5786)
- fix: correct output format for coverage report and enhance release summary message (c64634c)

### 🔧 Maintenance

- chore: track Cargo.lock for reproducible builds (b555a6e)

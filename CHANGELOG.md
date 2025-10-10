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

### Planned
- Support for additional JDK distributions (Oracle, Corretto, Azul Zulu, GraalVM)
- Windows support
- Integration with build tools (Maven, Gradle)
- Version migration tools
- Enhanced error handling and validation
- Update command to upgrade JCVM itself
- Cache management commands
- Checksum verification for downloads

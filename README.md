# JCVM - Universal Development Tool Version Manager

**A fast, secure, and modern version manager for Java, Node.js, Python, and more - written in Rust** 🚀

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)

## ✨ Features

- 🚀 **Fast & Secure**: Built with Rust for maximum performance and safety
- 🔧 **Multi-Tool Support**: Manage Java, Node.js, Python, and more from a single tool
- 🔄 **Easy Version Switching**: Seamlessly switch between multiple versions of any tool
- 🎯 **Fuzzy Version Matching**: Use `jcvm use --tool python 3.13` to match `3.13.7` automatically
- 📁 **Project-based Configuration**: Auto-switch based on `.java-version`, `.node-version`, `.python-version` files
- 🌍 **Global & Local Versions**: Set system-wide defaults and project-specific versions
- 🔐 **Checksum Verification**: Automatic verification of downloaded packages
- 📊 **Progress Indicators**: Beautiful progress bars for downloads and installations
- 🎨 **Rich CLI Experience**: Colored output, interactive prompts, and helpful messages
- 🐚 **Shell Integration**: Works with Bash, Zsh, Fish, and PowerShell
- 🔌 **Extensible Plugin System**: Add support for new tools via plugins
- 💾 **Smart Caching**: Cache downloads to save bandwidth

## 📦 Installation

### Using Pre-built Binaries (Recommended)

Download the latest binary for your platform from the [releases page](https://github.com/Yezta/cvm/releases).

### Building from Source

Requires Rust 1.70 or later. [Install Rust](https://rustup.rs/) if you haven't already.

```bash
# Clone the repository
git clone https://github.com/Yezta/cvm.git
cd jcvm

# Build in release mode
cargo build --release

# Install (optional)
cargo install --path .
```

### Shell Integration

After installing, set up shell integration:

```bash
jcvm shell-init
```

This will automatically detect your shell and add the necessary configuration. Then reload your shell:

```bash
# For Bash
source ~/.bashrc

# For Zsh
source ~/.zshrc

# For Fish
source ~/.config/fish/config.fish

# For PowerShell
. $PROFILE
```

## 🚀 Quick Start

### 1. List Available Versions

```bash
# Java (default tool for backward compatibility)
jcvm list-remote           # All Java versions
jcvm list-remote --lts     # Java LTS versions only

# Node.js
jcvm list-remote --tool node       # All Node.js versions
jcvm list-remote --tool node --lts # Node.js LTS versions

# Python
jcvm list-remote --tool python     # All Python versions
```

### 2. Install a Tool Version

```bash
# Java
jcvm install 21                    # Install latest JDK 21 (default tool)
jcvm install --tool java 17        # Explicit Java installation

# Node.js
jcvm install --tool node 20.10.0   # Install Node.js 20.10.0
jcvm install --tool node 18        # Install latest Node.js 18.x

# Python
jcvm install --tool python 3.12    # Install Python 3.12
```

### 3. Use a Tool Version

```bash
# Java
jcvm use 21                        # Switch to JDK 21 (default tool)
jcvm use --tool java 17            # Explicit Java switch

# Node.js
jcvm use --tool node 20.10.0       # Switch to Node.js 20.10.0

# Python
jcvm use --tool python 3.12        # Switch to Python 3.12
```

**💡 Fuzzy Version Matching**: You don't need to specify the full version!

```bash
# These work automatically (matches highest installed patch version):
jcvm use --tool python 3.13        # Matches 3.13.7
jcvm use --tool node 22            # Matches 22.19.0
jcvm use --tool java 21            # Matches 21.0.7

# Exact versions still work:
jcvm use --tool python 3.10.10     # Matches exactly 3.10.10
```

### 4. Set Project-specific Version

```bash
cd my-project

# Java
jcvm local 17                      # Creates .java-version file

# Node.js
jcvm local --tool node 20.10.0     # Creates .node-version file

# Python
jcvm local --tool python 3.12      # Creates .python-version file
```

Now whenever you `cd` into this directory, JCVM will automatically switch to the configured versions!

### 5. Import Existing Java Installations

JCVM can detect and import Java installations already on your system:

```bash
# Detect existing Java installations
jcvm detect

# Auto-import all detected installations
jcvm detect --import

# Import a specific installation
jcvm import /Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home
```

**Benefits:**

- Manage all Java versions through JCVM, even those installed outside of it
- Uninstalling from JCVM only removes the symlink, not the original installation
- Use the same commands (`use`, `local`, `alias`) for all versions

**Note:** On first run of `jcvm shell-init`, you'll be prompted to automatically import detected installations.

### 6. Set Global Default

```bash
jcvm alias default 21      # Set JDK 21 as default
```

## 📚 Commands

### Version Management

```bash
# List available versions
jcvm list-remote                      # List Java versions (default)
jcvm list-remote --tool node --lts    # List Node.js LTS versions
jcvm list-remote --tool python        # List Python versions

# Install versions
jcvm install 21                       # Install JDK 21 (default tool)
jcvm install --tool node 20.10.0      # Install Node.js 20.10.0
jcvm install --tool python 3.12 -f    # Force reinstall Python 3.12

# Uninstall versions
jcvm uninstall 21                     # Uninstall JDK 21 (default tool)
jcvm uninstall --tool node 20.10.0    # Uninstall Node.js 20.10.0

# List installed versions
jcvm list                             # List Java versions (default)
jcvm list --tool node                 # List Node.js versions
jcvm list --all                       # List all tool versions
```

### Detection & Import (Java only)

```bash
jcvm detect                # Detect existing Java installations
jcvm detect --import       # Auto-import all detected installations
jcvm import <path>         # Import a specific Java installation
```

### Version Switching

```bash
# Activate a version
jcvm use 21                           # Use JDK 21 (default tool)
jcvm use --tool node 20.10.0          # Use Node.js 20.10.0
jcvm use --tool python 3.12           # Use Python 3.12

# Check current version
jcvm current                          # Show current Java version
jcvm current --tool node              # Show current Node.js version
jcvm current --all                    # Show all active versions

# Show which version would be used
jcvm which                            # Show effective version path

# Set project-local version
jcvm local 17                         # Set local Java version
jcvm local --tool node 20.10.0        # Set local Node.js version
```

### Aliases

```bash
# List aliases
jcvm alias                            # List Java aliases (default)
jcvm alias --tool node                # List Node.js aliases

# Create/show aliases
jcvm alias default 21                 # Set Java default alias
jcvm alias --tool node lts 20.10.0    # Set Node.js lts alias
```

### Utilities

```bash
jcvm exec -v 17 mvn clean  # Run command with specific JDK
jcvm clean                 # Clean download cache
jcvm clean --all           # Remove all cached files
jcvm config                # Show configuration
jcvm shell-init            # Install shell integration
```

## 🏗️ Architecture

### Project Structure

```text
jcvm/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # CLI interface and commands
│   ├── api.rs               # Adoptium API client
│   ├── config.rs            # Configuration management
│   ├── detect.rs            # System Java detection & import
│   ├── download.rs          # Download with progress & verification
│   ├── install.rs           # Installation & extraction logic
│   ├── version_manager.rs   # Version switching logic
│   ├── shell.rs             # Shell integration
│   ├── models.rs            # Data models
│   ├── error.rs             # Error types
│   └── utils.rs             # Utility functions
├── Cargo.toml               # Dependencies
└── README.md                # This file
```

### Key Design Decisions

1. **Rust for Performance**: Leverages Rust's safety and speed for reliable operations
2. **Async Operations**: Uses Tokio for concurrent downloads and API requests
3. **Type Safety**: Strong typing prevents common errors
4. **Modular Design**: Clear separation of concerns for maintainability
5. **User Experience First**: Rich CLI with colors, progress bars, and helpful messages

## 🔧 Configuration

JCVM stores its configuration in `~/.jcvm/config.toml` (or platform-specific location).

### Configuration Options

```toml
default_distribution = "adoptium"  # JDK distribution
verify_checksums = true            # Verify download checksums
cache_downloads = true             # Cache downloaded files
cache_retention_days = 30          # Days to keep cache
show_lts_indicator = true          # Show LTS markers
parallel_downloads = true          # Enable parallel downloads
```

### Environment Variables

- `JCVM_DIR`: Override default JCVM directory (default: `~/.jcvm`)

## 🔄 Migration from Shell Version

If you're migrating from the shell-based JCVM:

1. **Your installed JDKs are compatible**: The Rust version uses the same directory structure
2. **Aliases are preserved**: All your aliases will continue to work
3. **`.java-version` files work**: No changes needed to your projects

Simply install the Rust version and run:

```bash
jcvm list                  # See your existing installations
jcvm shell-init            # Update shell configuration
```

## 🛡️ Security Features

- ✅ **Checksum Verification**: All downloads verified with SHA-256
- ✅ **Safe File Operations**: Rust's ownership prevents common vulnerabilities
- ✅ **No Arbitrary Code Execution**: Pure installation without running scripts
- ✅ **Secure HTTPS**: All downloads over encrypted connections
- ✅ **Input Validation**: All user inputs are validated and sanitized

## 🌟 Improvements Over Shell Version

| Feature | Shell Version | Rust Version |
|---------|--------------|--------------|
| Performance | Moderate | **Fast** ⚡ |
| Progress Indicators | Basic | **Rich & Interactive** |
| Error Handling | Basic | **Comprehensive** |
| Checksum Verification | Optional | **Always On** |
| Parallel Operations | No | **Yes** |
| Interactive Prompts | Limited | **Full Featured** |
| Code Safety | Bash scripting | **Rust Type Safety** |
| Cross-platform | macOS/Linux | **macOS/Linux/Windows** |
| Binary Size | N/A | **~5MB** |
| Dependencies | curl/wget/jq | **Self-contained** |

## 🧪 Testing

Run the test suite:

```bash
cargo test
```

Run with coverage:

```bash
cargo tarpaulin --out Html
```

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with ❤️ using [Rust](https://www.rust-lang.org/)
- JDK distributions provided by [Eclipse Adoptium](https://adoptium.net/)
- Inspired by [NVM](https://github.com/nvm-sh/nvm) for Node.js

## 📧 Support

- 📖 [Documentation](https://github.com/Yezta/cvm)
- 📚 [Complete Documentation](docs/)
  - [Architecture Guide](docs/ARCHITECTURE.md)
  - [Plugin Development](docs/PLUGIN_DEVELOPMENT.md)
  - [Version Management](docs/VERSION_MANAGEMENT.md)
  - [Release Process](docs/RELEASE.md)
  - [Testing Guide](docs/TESTING.md)
  - [Roadmap](docs/ROADMAP.md)
- 🐛 [Issue Tracker](https://github.com/Yezta/cvm/issues)
- 💬 [Discussions](https://github.com/Yezta/cvm/discussions)

---

<div align="center">

**[Website](https://jcvm.dev)** • **[Documentation](https://docs.jcvm.dev)** • **[Changelog](CHANGELOG.md)**

Made with 🦀 by the JCVM team

</div>

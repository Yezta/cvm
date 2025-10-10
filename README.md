# JCVM - Java Configuration & Version Manager

A version manager for JDK, similar to NVM for Node.js. Easily install, manage, and switch between multiple JDK versions.

## Features

- 🚀 Install multiple JDK versions from various distributions (Eclipse Temurin, Oracle OpenJDK, etc.)
- 🔄 Seamlessly switch between installed versions
- 📁 Auto-switch based on `.java-version` file in project directories
- 🌍 Set global default JDK version
- 📋 List available and installed versions
- 🗑️ Uninstall versions you no longer need
- 🔧 Automatic JAVA_HOME and PATH management

## Installation

### macOS / Linux

```bash
curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
```

Or manually:

```bash
git clone https://github.com/yourusername/jcvm.git ~/.jcvm
cd ~/.jcvm
chmod +x jcvm.sh
```

Add to your shell profile (`~/.zshrc`, `~/.bashrc`, or `~/.bash_profile`):

```bash
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"
```

Reload your shell:

```bash
source ~/.zshrc  # or ~/.bashrc
```

## Usage

### List available JDK versions

```bash
jcvm list-remote
```

### Install a JDK version

```bash
jcvm install 21        # Install latest JDK 21
jcvm install 17.0.10   # Install specific version
jcvm install 11 --lts  # Install LTS version
```

### List installed versions

```bash
jcvm list
```

### Use a specific version

```bash
jcvm use 21           # Use JDK 21 in current shell
jcvm use 17           # Switch to JDK 17
```

### Set global default version

```bash
jcvm alias default 21  # Set JDK 21 as default
```

### Set local version for a project

```bash
cd my-project
jcvm local 17         # Creates .java-version file
```

When you enter a directory with a `.java-version` file, JCVM will automatically switch to that version.

### Uninstall a version

```bash
jcvm uninstall 11
```

### Display current version

```bash
jcvm current
java -version
```

### Show help

```bash
jcvm help
```

## How It Works

JCVM manages multiple JDK installations by:

1. **Installation**: Downloads JDK distributions to `~/.jcvm/versions/`
2. **Version Switching**: Creates shims that update `JAVA_HOME` and `PATH`
3. **Auto-switching**: Detects `.java-version` files and switches automatically when changing directories
4. **Isolation**: Each JDK version is completely isolated

## Supported JDK Distributions

- Eclipse Temurin (Adoptium)
- Oracle OpenJDK
- Amazon Corretto
- Azul Zulu
- GraalVM

## Configuration

JCVM stores its data in:

- `~/.jcvm/` - Main directory
- `~/.jcvm/versions/` - Installed JDK versions
- `~/.jcvm/alias/` - Version aliases
- `~/.jcvm/config` - Configuration file

## Project-Specific Configuration

Create a `.java-version` file in your project root:

```
17
```

or with more specificity:

```
17.0.10
```

When you `cd` into the directory, JCVM will automatically switch to that version.

## Comparison with Other Tools

| Feature | JCVM | SDKMAN! | jEnv |
|---------|------|---------|------|
| Install JDKs | ✅ | ✅ | ❌ |
| Auto-switch | ✅ | ✅ | ✅ |
| Project config | ✅ (.java-version) | ✅ (.sdkmanrc) | ✅ (.java-version) |
| Lightweight | ✅ | ❌ | ✅ |
| POSIX compliant | ✅ | ❌ | ❌ |

## Troubleshooting

### Command not found

Make sure JCVM is properly loaded in your shell profile and the profile has been sourced.

### Permission errors

JCVM works in user space and doesn't require sudo. If you get permission errors, check your `~/.jcvm` directory permissions.

### JDK not switching

Run `jcvm current` to see which version is active. Make sure you've sourced your shell profile after installation.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

MIT License - see LICENSE file for details

## Credits

Inspired by:
- [NVM](https://github.com/nvm-sh/nvm) - Node Version Manager
- [SDKMAN!](https://sdkman.io/) - The Software Development Kit Manager
- [jEnv](https://www.jenv.be/) - Java Environment Manager

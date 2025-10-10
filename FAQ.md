# Frequently Asked Questions (FAQ)

## General Questions

### What is JCVM?

JCVM (Java Configuration & Version Manager) is a command-line tool that helps you install, manage, and switch between multiple JDK versions on your system. It's inspired by NVM (Node Version Manager) and provides similar functionality for Java development.

### Why should I use JCVM instead of manually managing JDK installations?

- **Easy version switching**: Switch between JDK versions with a single command
- **Project isolation**: Different projects can use different JDK versions
- **Auto-switching**: Automatically use the right JDK version based on project configuration
- **No sudo required**: All installations are in user space
- **Clean uninstall**: Remove versions you don't need anymore
- **Automatic PATH management**: No manual environment variable configuration

### How is JCVM different from SDKMAN! or jEnv?

| Feature | JCVM | SDKMAN! | jEnv |
|---------|------|---------|------|
| JDK installation | ✅ | ✅ | ❌ (only manages existing) |
| Auto-switch on cd | ✅ | ✅ | ✅ |
| Lightweight | ✅ | ❌ | ✅ |
| POSIX shell compliant | ✅ | ❌ | ❌ |
| Inspired by NVM | ✅ | ❌ | ❌ |
| Multiple SDK support | ❌ (Java only) | ✅ | ❌ |

JCVM is specifically designed for Java developers who want an NVM-like experience.

## Installation

### Where does JCVM install JDK versions?

All JDK versions are installed in `~/.jcvm/versions/`. Each version gets its own directory.

### Can I use JCVM without admin/sudo privileges?

Yes! JCVM works entirely in user space and doesn't require any administrative privileges.

### How do I update JCVM itself?

Currently:
```bash
cd ~/.jcvm
git pull origin main
```

An automatic update command is planned for future releases.

### Can I install JCVM system-wide?

JCVM is designed to be a per-user installation. Each user should install their own copy.

## Usage

### How do I know which JDK version I'm currently using?

```bash
jcvm current
# or
java -version
echo $JAVA_HOME
```

### Can I use multiple JDK versions simultaneously?

Each shell session uses one JDK version at a time, but different terminal windows can use different versions. Projects with `.java-version` files will automatically use their specified version.

### What is the `.java-version` file?

It's a simple text file containing the JDK version number. When you `cd` into a directory with this file, JCVM automatically switches to that version. Example:

```
21
```

or more specific:

```
21.0.1
```

### How do I set a default JDK version?

```bash
jcvm alias default 21
```

This version will be used in new shell sessions.

### Can I install multiple versions of the same major release?

Not directly. JCVM currently uses major version numbers (8, 11, 17, 21) as identifiers. Support for minor versions may be added in future releases.

## JDK Sources

### Which JDK distributions does JCVM support?

Currently, JCVM uses **Eclipse Temurin** (formerly AdoptOpenJDK) from the Adoptium project as the primary source. This provides:

- High-quality, TCK-certified builds
- Regular updates
- LTS support
- Cross-platform availability

Support for additional distributions (Oracle OpenJDK, Amazon Corretto, Azul Zulu, GraalVM) is planned.

### Are the JDKs from Eclipse Temurin safe to use?

Yes! Eclipse Temurin is:
- Maintained by the Eclipse Adoptium project
- TCK (Technology Compatibility Kit) certified
- Used by millions of developers worldwide
- Backed by major companies including Microsoft, Red Hat, IBM, and others

### Can I use JCVM with my existing JDK installations?

JCVM manages its own installations in `~/.jcvm/versions/`. Your existing system JDK installations remain untouched. You can still use them by not activating any JCVM version.

## Troubleshooting

### JCVM command not found

Make sure you've:
1. Installed JCVM correctly
2. Added the initialization script to your shell profile
3. Reloaded your shell: `source ~/.zshrc` or `source ~/.bashrc`

### Java version doesn't change after `jcvm use`

Try:
1. Check if JCVM is loaded: `echo $JCVM_DIR`
2. Verify the version is installed: `jcvm list`
3. Reload your shell: `source ~/.zshrc`
4. Check your PATH: `echo $PATH | tr ':' '\n' | grep jcvm`

### Auto-switching doesn't work

Verify:
1. You have a `.java-version` file in the directory
2. The version in the file is installed: `jcvm list`
3. Your shell supports the cd hook (bash or zsh)

### Download fails or is very slow

- Check your internet connection
- The Adoptium CDN might be experiencing issues
- Try again later
- Check if you can access https://adoptium.net in your browser

### How do I completely uninstall JCVM?

```bash
# Remove JCVM directory
rm -rf ~/.jcvm

# Remove from shell profile (edit manually)
# Remove these lines from ~/.zshrc or ~/.bashrc:
# export JCVM_DIR="$HOME/.jcvm"
# [ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"

# Reload shell
source ~/.zshrc  # or ~/.bashrc
```

## Platform-Specific

### Does JCVM work on Windows?

Not yet. JCVM currently supports macOS and Linux. Windows support is planned for future releases. In the meantime, Windows users can:
- Use WSL2 (Windows Subsystem for Linux)
- Use SDKMAN! for Windows
- Use jEnv with manual JDK installation

### Which Linux distributions are supported?

JCVM should work on any Linux distribution with:
- bash or zsh shell
- curl or wget
- tar
- Standard GNU utilities

Tested on: Ubuntu, Debian, Fedora, CentOS, Arch Linux

### Does JCVM work on Apple Silicon (M1/M2/M3)?

Yes! JCVM automatically detects ARM64 architecture and downloads the appropriate JDK builds.

## Advanced

### Can I use JCVM in CI/CD pipelines?

Yes! Example:

```bash
# Install JCVM
curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
export JCVM_DIR="$HOME/.jcvm"
source $JCVM_DIR/jcvm.sh

# Install and use specific version
jcvm install 21
jcvm use 21

# Run your build
./mvnw clean package
```

### Can I use JCVM with Docker?

Yes! Include in your Dockerfile:

```dockerfile
RUN curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
RUN . ~/.bashrc && jcvm install 21
```

### How do I migrate from jEnv or SDKMAN!?

1. Install JCVM
2. List your current JDK installations
3. Note which versions you need
4. Install them with JCVM: `jcvm install <version>`
5. Update your projects' `.java-version` files
6. Optionally uninstall the old version manager

### Can I customize the JCVM installation directory?

Yes, set the `JCVM_DIR` environment variable before installation:

```bash
export JCVM_DIR="$HOME/.config/jcvm"
curl -o- https://install-script-url | bash
```

## Contributing

### How can I contribute to JCVM?

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines. We welcome:
- Bug reports
- Feature requests
- Documentation improvements
- Code contributions
- Testing on different platforms

### I found a bug, what should I do?

Open an issue on GitHub with:
- Description of the bug
- Steps to reproduce
- Your OS and shell version
- Error messages

### Can I add support for other JDK distributions?

Yes! We'd love contributions to add support for:
- Oracle OpenJDK
- Amazon Corretto
- Azul Zulu
- GraalVM
- Others

Check the contribution guidelines and open a PR!

---

Still have questions? [Open an issue](https://github.com/yourusername/jcvm/issues) on GitHub!

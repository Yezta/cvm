# JCVM Project Architecture

This document describes the architecture and design decisions of JCVM.

## Project Overview

JCVM (Java Configuration & Version Manager) is a shell-based tool for managing multiple JDK installations, inspired by NVM (Node Version Manager).

## Core Design Principles

1. **User-space installation**: No sudo/admin privileges required
2. **Shell integration**: Works seamlessly with bash and zsh
3. **POSIX compliance**: Uses standard shell scripting for portability
4. **Automatic management**: Handles JAVA_HOME and PATH automatically
5. **Project-based configuration**: Support for `.java-version` files
6. **Minimal dependencies**: Only requires curl/wget and tar

## Directory Structure

```
~/.jcvm/
├── jcvm.sh              # Main script
├── versions/            # Installed JDK versions
│   ├── 21/              # JDK 21
│   ├── 17/              # JDK 17
│   └── ...
├── alias/               # Version aliases (symlinks)
│   ├── default -> ../versions/21
│   └── current -> ../versions/21
└── cache/               # Download cache
    └── *.tar.gz
```

## Core Components

### 1. Version Management

- **Installation**: Downloads JDK from Adoptium API
- **Storage**: Each version in separate directory under `~/.jcvm/versions/`
- **Switching**: Updates JAVA_HOME and PATH environment variables
- **Aliases**: Symlinks for named versions (e.g., `default`)

### 2. Auto-Switching

Uses shell hooks to detect directory changes:

- **zsh**: Uses `chpwd` hook via `add-zsh-hook`
- **bash**: Wraps the `cd` command with custom function

When entering a directory with `.java-version`, automatically switches to that version.

### 3. Environment Management

```bash
JAVA_HOME="$JCVM_VERSIONS_DIR/<version>"
PATH="$JAVA_HOME/bin:$PATH"
```

- Removes old JCVM paths from PATH before adding new one
- Prevents PATH pollution
- Maintains correct precedence

## API Integration

### Adoptium API

JCVM uses the Eclipse Temurin (Adoptium) API:

```
Base URL: https://api.adoptium.net/v3
```

Key endpoints:
- `/info/available_releases` - List available JDK versions
- `/assets/latest/{version}/hotspot` - Get download links

### Platform Detection

- **OS**: Detects macOS, Linux, Windows via `uname -s`
- **Architecture**: Detects x64, aarch64, etc. via `uname -m`
- Downloads appropriate binary for detected platform

## Command Flow

### Install Command

```
jcvm install 21
    ↓
Detect OS & Architecture
    ↓
Query Adoptium API for JDK 21
    ↓
Download to ~/.jcvm/cache/
    ↓
Extract to ~/.jcvm/versions/21/
    ↓
Clean up cache
    ↓
Success message
```

### Use Command

```
jcvm use 21
    ↓
Check if version exists
    ↓
Set JAVA_HOME=~/.jcvm/versions/21
    ↓
Update PATH
    ↓
Create symlink: alias/current -> versions/21
    ↓
Verify with java -version
```

### Auto-Switch

```
cd /path/to/project
    ↓
chpwd hook triggered (zsh) or cd wrapper (bash)
    ↓
Check for .java-version file
    ↓
Read version from file
    ↓
Compare with current version
    ↓
If different: jcvm use <version>
```

## File Formats

### .java-version

Simple text file containing version number:

```
21
```

or with minor version:

```
21.0.1
```

### Alias Files

Symlinks in `~/.jcvm/alias/`:

```bash
default -> ../versions/21
current -> ../versions/17
```

## Shell Integration

### Initialization

Added to `~/.zshrc` or `~/.bashrc`:

```bash
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"
```

### Hook Registration

**Zsh:**
```bash
autoload -U add-zsh-hook
add-zsh-hook chpwd jcvm_cd_hook
```

**Bash:**
```bash
jcvm_cd() {
    builtin cd "$@" && jcvm_auto_switch
}
alias cd='jcvm_cd'
```

## Error Handling

- **Download failures**: Retry logic, clear error messages
- **Missing versions**: Suggest installation command
- **Permission issues**: Check directory permissions
- **Network errors**: Graceful degradation

## Security Considerations

1. **Download verification**: Uses HTTPS for all downloads
2. **User space**: No system-wide modifications
3. **Trusted source**: Eclipse Temurin is TCK-certified
4. **No sudo**: Eliminates privilege escalation risks
5. **Isolated versions**: Each JDK in separate directory

## Performance Optimizations

1. **Caching**: Downloads cached to avoid re-downloading
2. **Lazy loading**: Only loads when shell starts
3. **Fast switching**: Environment variable updates only
4. **Minimal overhead**: Lightweight shell functions

## Extensibility

### Adding New Distributions

To add support for other JDK distributions:

1. Add distribution-specific API client functions
2. Update `jcvm_install` to support distribution selection
3. Add distribution parameter to commands
4. Update documentation

Example:
```bash
jcvm install 21 --dist=corretto
```

### Adding New Commands

1. Create function: `jcvm_<command_name>()`
2. Add case in main dispatcher
3. Update help text
4. Add tests

## Testing Strategy

### Manual Testing

- Install on fresh system
- Test all commands
- Verify auto-switching
- Check cross-platform compatibility

### Automated Testing

- `test.sh` script validates core functions
- CI/CD can run integration tests
- Future: Add unit tests for individual functions

## Future Enhancements

1. **Windows support**: PowerShell version
2. **Multiple distributions**: Oracle, Corretto, Zulu, GraalVM
3. **Version ranges**: Support for `>=17` in `.java-version`
4. **Migration tools**: Import from jEnv/SDKMAN!
5. **Plugin system**: Extensible architecture
6. **Caching improvements**: Checksums, CDN fallbacks
7. **Update command**: Self-update capability
8. **GUI**: Optional graphical interface

## Comparison with Alternatives

### vs. SDKMAN!

- JCVM: Lighter, Java-focused, NVM-like UX
- SDKMAN!: Heavier, multi-SDK support, more features

### vs. jEnv

- JCVM: Downloads and manages JDKs
- jEnv: Only manages existing installations

### vs. Manual Management

- JCVM: Automated, consistent, easy switching
- Manual: Error-prone, requires expertise

## Dependencies

**Required:**
- bash or zsh shell
- curl or wget
- tar
- Standard Unix utilities (mkdir, ln, etc.)

**Optional:**
- git (for clone-based installation)
- jq (for better JSON parsing, future enhancement)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for:
- Code style guidelines
- Testing requirements
- Pull request process
- Issue reporting

## References

- [NVM Architecture](https://github.com/nvm-sh/nvm)
- [Adoptium API](https://api.adoptium.net/)
- [POSIX Shell Scripting](https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html)
- [Shell Hooks](https://zsh.sourceforge.io/Doc/Release/Functions.html)

---

**Design Philosophy**: Simple, reliable, user-friendly version management for Java developers.

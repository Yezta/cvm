# JCVM Architecture - Universal Version Manager

## Vision

JCVM has evolved from a Java-specific version manager into a **Universal Version Manager** capable of managing any tool, programming language, runtime, database, or application that requires version management.

## Core Architecture

### 1. Plugin-Based Design

```text
┌─────────────────────────────────────────────────────────┐
│                     CLI / UI / API                       │
│                  (User Interface Layer)                  │
└─────────────────────────────────────────────────────────┘
          │
┌─────────────────────────────────────────────────────────┐
│                    Tool Manager                         │
│    (Activation, installs, multi-tool orchestration)     │
└─────────────────────────────────────────────────────────┘
          │
┌─────────────────────────────────────────────────────────┐
│                  Plugin Registry                         │
│            (Dynamic Plugin Management)                   │
└─────────────────────────────────────────────────────────┘
          │
  ┌───────────────────┼───────────────────┐
  │                   │                   │
┌───────▼────────┐  ┌──────▼───────┐  ┌───────▼────────┐
│  Java Plugin   │  │ Node Plugin  │  │ Python Plugin  │
│                │  │              │  │                │
│ - Provider     │  │ - Provider   │  │ - Provider     │
│ - Installer    │  │ - Installer  │  │ - Installer    │
│ - Detector     │  │ - Detector   │  │ - Detector     │
└────────────────┘  └──────────────┘  └────────────────┘
```text

### 2. Core Traits

Every tool plugin must implement three core traits:

#### ToolProvider

- Discovers available versions from remote sources
- Provides tool metadata and information
- Parses version strings
- Validates installations

#### ToolInstaller

- Downloads and installs tool versions
- Uninstalls versions
- Verifies installation integrity

#### ToolDetector

- Detects existing system installations
- Imports external installations into management

### 3. Directory Structure

```text
~/.jcvm/
├── config.toml              # Global configuration
├── versions/                # Managed tool versions
│   ├── java/
│   │   ├── 21/
│   │   ├── 17/
│   │   └── 11/
│   ├── node/
│   │   ├── 20.10.0/
│   │   └── 18.17.0/
│   ├── python/
│   │   ├── 3.11.5/
│   │   └── 3.10.12/
│   └── compass/
│       └── 1.39.4/
├── alias/                   # Version aliases
│   ├── java/
│   │   ├── current -> ../versions/java/21
│   │   └── default -> ../versions/java/17
│   ├── node/
│   │   └── current -> ../versions/node/20.10.0
│   └── python/
│       └── current -> ../versions/python/3.11.5
├── cache/                   # Downloaded archives
└── plugins/                 # User-defined plugins (future)
```text

### 4. Tool and Plugin Management

The `ToolManager` coordinates every built-in plugin. It understands how to:

- Resolve installs across different directory layouts (legacy Java-only and the new per-tool folders)
- Create and prune aliases (`current`, `default`, or user-defined labels)
- Activate tools on demand via `jcvm tool use <tool> <version>` or the quick `jcvm switch <tool>@<version>` shortcut
- Persist metadata about each installation in a lightweight JSON manifest for future verification

`PluginRegistry` remains responsible for dynamic discovery but now stores plugins behind `Arc<dyn ToolPlugin>` so calls are thread-safe and cloning the registry is cheap.

### 5. Tool-Specific Configuration

Each tool maintains its own configuration within `config.toml`:

```toml
# Global settings
default_distribution = "adoptium"
verify_checksums = true
cache_downloads = true

[tools.java]
default_version = "21"
lts_only = false
distribution = "adoptium"

[tools.node]
default_version = "20"
npm_registry = "https://registry.npmjs.org"

[tools.python]
default_version = "3.11"

[tools.compass]
default_version = "1.39.4"
auto_update = false
```

## CLI & API Integration

### Everyday Commands

```bash
# Quick switch with auto-install if missing
jcvm switch node@20 --install

# List everything the manager knows about
jcvm tool list --all

# Inspect remote catalogs per tool
jcvm tool remote python --lts

# Alias workflows
jcvm tool alias java default --version 21
jcvm tool alias node production --version 20.10.0
```

### REST API Endpoints

The system exposes a comprehensive REST API for UI consumption:

```text
# Plugin Management
GET    /api/plugins                    # List all available plugins
GET    /api/plugins/{tool_id}          # Get plugin info
POST   /api/plugins/register           # Register new plugin (from UI)

# Version Management
GET    /api/plugins/{tool_id}/versions          # List available versions
GET    /api/plugins/{tool_id}/versions/remote   # List remote versions
GET    /api/plugins/{tool_id}/versions/installed # List installed versions

# Installation
POST   /api/plugins/{tool_id}/install            # Install a version
DELETE /api/plugins/{tool_id}/versions/{version} # Uninstall a version

# Version Switching
POST   /api/plugins/{tool_id}/use                # Set current version
GET    /api/plugins/{tool_id}/current            # Get current version

# Detection & Import
GET    /api/plugins/{tool_id}/detect             # Detect system installations
POST   /api/plugins/{tool_id}/import             # Import external installation

# Aliases
POST   /api/plugins/{tool_id}/alias              # Create alias
GET    /api/plugins/{tool_id}/aliases            # List aliases
```

### Example API Usage

```bash
# List all plugins
curl http://localhost:8080/api/plugins

# List Java versions
curl http://localhost:8080/api/plugins/java/versions

# Install Node.js 20
curl -X POST http://localhost:8080/api/plugins/node/install \
  -H "Content-Type: application/json" \
  -d '{"version": "20.10.0"}'

# Switch to Python 3.11
curl -X POST http://localhost:8080/api/plugins/python/use \
  -H "Content-Type: application/json" \
  -d '{"version": "3.11.5"}'
```

## Plugin Development Workflow

### 1. Built-in Plugins

Built-in plugins are compiled into the binary:

- **Java** (JDK from Adoptium)
- **Node.js** (from nodejs.org)
- **Python** (from python.org)
- **Compass** (MongoDB Compass from GitHub releases)

### 2. User-Defined Plugins

Users can create plugins in two ways:

#### A. Rust Plugin (Compiled)

1. Create plugin following trait system
2. Compile as dynamic library
3. Place in `~/.jcvm/plugins/`
4. Auto-discovered on startup

#### B. Declarative Plugin (Configuration)

For simple tools with standard patterns:

```toml
[[user_plugins]]
id = "my-tool"
name = "My Custom Tool"
category = "tool"

[user_plugins.discovery]
api_url = "https://api.github.com/repos/user/tool/releases"
version_pattern = "v?(\\d+\\.\\d+\\.\\d+)"

[user_plugins.download]
url_pattern = "https://github.com/user/tool/releases/download/v{version}/tool-{platform}-{arch}.tar.gz"
archive_type = "tar.gz"
executable_path = "bin/tool"

[user_plugins.environment]
TOOL_HOME = "{install_path}"
PATH = "{install_path}/bin:$PATH"
```

## UI Integration Example

### React UI Component

```typescript
// List all managed tools
const ToolsView = () => {
  const [plugins, setPlugins] = useState([]);
  
  useEffect(() => {
    fetch('http://localhost:8080/api/plugins')
      .then(res => res.json())
      .then(data => setPlugins(data));
  }, []);
  
  return (
    <div>
      {plugins.map(plugin => (
        <ToolCard 
          key={plugin.id}
          plugin={plugin}
          onInstall={(version) => installVersion(plugin.id, version)}
          onSwitch={(version) => switchVersion(plugin.id, version)}
        />
      ))}
    </div>
  );
};

// Install a version
const installVersion = async (toolId, version) => {
  await fetch(`http://localhost:8080/api/plugins/${toolId}/install`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ version })
  });
};
```

## Shell Integration

The system maintains backward compatibility with shell integration:

```bash
# .zshrc / .bashrc
export JCVM_DIR="$HOME/.jcvm"
eval "$(jcvm shell-init)"

# Auto-switching based on .tool-versions file
# .tool-versions
java 21
node 20.10.0
python 3.11.5
```

## Security Considerations

### 1. Checksum Verification

- All downloads verified against checksums
- SHA256 hashes from official sources

### 2. Sandboxed Installation

- Tools installed in user directory
- No system-wide modifications
- No root privileges required

### 3. Plugin Validation

- Built-in plugins are trusted
- User plugins run in sandboxed environment
- Declarative plugins can't execute arbitrary code

### 4. Network Security

- HTTPS for all downloads
- Certificate pinning for critical APIs
- Rate limiting on API calls

## Performance Optimizations

### 1. Parallel Operations

- Concurrent version downloads
- Parallel detection across paths
- Async API calls

### 2. Caching Strategy

- Downloaded archives cached
- API responses cached (with TTL)
- Version metadata cached

### 3. Lazy Loading

- Plugins loaded on-demand
- Version lists paginated
- Deferred detection

## Future Enhancements

### Phase 1 (Current)

- ✅ Core trait system with thread-safe plugin registry
- ✅ Unified tool manager and quick-switch UX
- ✅ Java, Node.js, and Python plugins verified end-to-end
- 🔄 REST API server

### Phase 2 (Next)

- REST API server
- WebSocket for real-time updates
- Plugin marketplace
- Auto-update mechanism

### Phase 3 (Future)

- Desktop UI (Tauri/Electron)
- Mobile app for remote management
- Cloud sync of configurations
- Team workspace support

### Phase 4 (Advanced)

- AI-powered version recommendations
- Dependency conflict resolution
- Automatic compatibility checking
- Integration with CI/CD pipelines

## Migration Path

### For Existing JCVM Users

1. **Automatic Migration**: Existing Java installations automatically detected
2. **Configuration Preserved**: All settings migrated to new format
3. **Backward Compatible**: Old commands still work
4. **New Features Optional**: Can continue using Java-only mode

### Migration Steps

```bash
# Backup existing setup
cp -r ~/.jcvm ~/.jcvm.backup

# Update to new version
jcvm update

# Migrate installations
jcvm migrate --from java-only --to universal

# Verify migration
jcvm list --all-tools
```

## Testing Strategy

### 1. Unit Tests

- Each plugin has comprehensive unit tests
- Trait implementations tested in isolation
- Mock external APIs

### 2. Integration Tests

- End-to-end installation flows
- Version switching across tools
- Shell integration tests

### 3. Platform Tests

- macOS (Intel + Apple Silicon)
- Linux (various distros)
- Windows (WSL + native)

### 4. UI Tests

- API endpoint tests
- Frontend component tests
- E2E user workflows

## Contributing

See [PLUGIN_DEVELOPMENT.md](PLUGIN_DEVELOPMENT.md) for detailed plugin development guide.

## License

MIT License - See LICENSE file for details.

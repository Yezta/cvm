# GitHub Actions Workflows

This directory contains all GitHub Actions workflows for the JCVM project. These workflows are inspired by and adapted from the excellent CI/CD setup in [Tabby Terminal](https://github.com/Eugeny/tabby).

## 🔄 How This Differs from Tabby

While we've adopted many patterns from Tabby, our workflow architecture differs to better suit a Rust CLI project:

**Tabby's Approach** (TypeScript/Electron):
- Single `build.yml` handles both CI testing AND release builds (triggered on tags)
- Simple `release.yml` just creates GitHub release draft
- Builds TypeScript/Electron with Node.js/yarn/webpack
- Uses Firebase for documentation hosting

**JCVM's Approach** (Rust CLI):
- **Separation of Concerns**: `ci.yml` for testing, `release.yml` for release artifacts
- `ci.yml` runs on every push/PR for fast feedback
- `release.yml` only runs on version tags to build optimized release binaries
- Pure Rust toolchain: cargo, rustup, cross-compilation
- GitHub Pages for documentation (more accessible for open source)
- Additional `nightly.yml` for bleeding-edge builds
- `version-bump.yml` helper for automated version management

**Why We Differ**:
1. **Faster CI**: Test builds don't need release optimizations
2. **Cleaner Separation**: CI testing vs production release building are distinct concerns
3. **Resource Efficiency**: Don't build release artifacts on every commit
4. **Better DX**: Developers get faster feedback from lightweight CI builds

Both approaches are valid - Tabby's works great for Electron apps, ours is optimized for Rust CLI tools.

## 📋 Workflow Overview

### Core Workflows

#### `ci.yml` - Continuous Integration
**Trigger**: Push and Pull Requests to `main`, `develop`, and `feat/*` branches

Comprehensive CI pipeline that includes:
- **Lint Job**: Fast formatting and clippy checks (runs first)
- **Code Quality**: Documentation checks and static analysis
- **Cross-Platform Testing**: 
  - macOS (Apple Silicon + Intel)
  - Linux (x86_64)
  - Windows (x86_64)
- **Security Audit**: Dependency vulnerability scanning
- **Code Coverage**: Test coverage reporting with Codecov
- **Integration Tests**: Platform-specific smoke tests

**Key Features**:
- Parallel job execution for faster feedback
- Smart caching with `Swatinem/rust-cache`
- Separate lint job that runs before tests
- Platform-specific build targets
- Artifact uploads on test failures

---

#### `release.yml` - Release Builds
**Trigger**: Git tags matching `v*.*.*` pattern or manual dispatch

Production release workflow that:
- Creates GitHub releases with auto-generated notes
- Builds binaries for multiple platforms:
  - **macOS**: Apple Silicon (aarch64) + Intel (x86_64)
  - **Linux**: x86_64, ARM64, ARMv7
  - **Windows**: x86_64
- Generates SHA256 checksums for all artifacts
- Includes installation scripts in archives
- Supports cross-compilation for ARM targets

**Artifacts**:
- `.tar.gz` for Unix platforms (includes install.sh)
- `.zip` for Windows (includes install.bat)
- `.sha256` checksum files for verification

---

#### `nightly.yml` - Nightly Builds
**Trigger**: 
- Daily at 2 AM UTC (cron)
- Push to `main` branch
- Manual dispatch

Builds nightly development versions:
- Same platform matrix as releases
- Version format: `{version}-nightly+{sha}`
- Includes `NIGHTLY_INFO.txt` with build details
- Artifacts retained for 30 days
- **Accessible via nightly.link**: https://nightly.link/{owner}/jcvm/workflows/nightly/main

**Warning**: Nightly builds are experimental and may contain bugs!

---

#### `codeql.yml` - Security Analysis
**Trigger**:
- Push and Pull Requests to `main` and `develop`
- Weekly on Mondays at 9 AM UTC
- Manual dispatch

Security scanning with:
- **CodeQL Analysis**: Advanced semantic code analysis
- **Dependency Review**: Checks for vulnerable dependencies in PRs
- **Security Audit**: Runs `cargo audit` for known CVEs
- Automatic security alerts and reporting

**Permissions**: Requires `security-events: write` for alerts

---

#### `docs.yml` - Documentation
**Trigger**:
- Push to `main` branch
- Manual dispatch

Builds and deploys Rust documentation:
- Generates rustdoc with `--document-private-items`
- Deploys to GitHub Pages
- Auto-redirect from root to crate documentation
- URL: `https://{owner}.github.io/jcvm`

---

#### `version-bump.yml` - Version Management
**Trigger**: Manual dispatch with version input

Automates version bumping:
- Updates version in `Cargo.toml`
- Updates `VERSION` file
- Creates commit and tag
- Pushes changes to trigger release workflow

---

## 🏗️ Build Matrix Strategy

Following Tabby's approach, we use comprehensive platform matrices:

### Release & Nightly Builds

| Platform | Architecture | Target Triple | Cross-Compile |
|----------|-------------|---------------|---------------|
| macOS | Apple Silicon | `aarch64-apple-darwin` | No |
| macOS | Intel | `x86_64-apple-darwin` | No |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` | No |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` | Yes |
| Linux | ARMv7 | `armv7-unknown-linux-gnueabihf` | Yes |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | No |

### CI Testing

| Platform | Target | Integration Tests |
|----------|--------------------------|------------------|
| macOS (latest) | `aarch64-apple-darwin` | ✅ |
| macOS (latest) | `x86_64-apple-darwin` | ✅ |
| Linux (ubuntu) | `x86_64-unknown-linux-gnu` | ✅ |
| Windows (latest) | `x86_64-pc-windows-msvc` | ❌ (smoke tests only) |

## 🔧 Key Technologies & Actions

### Rust Tooling
- **dtolnay/rust-toolchain**: Rust toolchain management
- **Swatinem/rust-cache**: Fast, smart Cargo caching
- **taiki-e/install-action**: Cross-compilation tools
- **rustsec/audit-check**: Security auditing

### CI/CD Actions
- **actions/checkout@v4**: Repository checkout
- **actions/upload-artifact@v4**: Artifact management
- **softprops/action-gh-release@v2**: Release creation
- **codecov/codecov-action@v4**: Coverage reporting
- **github/codeql-action@v3**: Security analysis

### Build Tools
- **cross**: Cross-compilation for ARM targets
- **cargo-tarpaulin**: Code coverage collection
- **strip**: Binary size optimization (Unix)

## 🚀 Usage Examples

### Triggering Workflows

**Create a Release**:
```bash
git tag v1.0.0
git push origin v1.0.0
```

**Manual Release** (via GitHub UI):
1. Go to Actions → Release
2. Click "Run workflow"
3. Enter tag (e.g., `v1.0.0`)

**Force Nightly Build**:
```bash
# Push to main triggers nightly
git push origin main

# Or via GitHub Actions UI
```

### Downloading Artifacts

**Latest Release**:
```bash
curl -fsSL https://raw.githubusercontent.com/{owner}/jcvm/main/scripts/install.sh | bash
```

**Nightly Builds** (via nightly.link):
```bash
# Visit: https://nightly.link/{owner}/jcvm/workflows/nightly/main
# Download artifacts for your platform
```

**Manual Download**:
1. Go to Actions → Select workflow run
2. Scroll to "Artifacts" section
3. Download platform-specific archive

## 📊 Workflow Dependencies

```
Lint (fast checks)
  ├─> Code Quality
  └─> Tests (parallel per platform)
        └─> Coverage
              └─> CI Success Gate

Release Creation
  └─> Build Binaries (parallel per platform)
        └─> Upload to Release
              └─> Finalize

Nightly
  └─> Build Artifacts (parallel per platform)
        └─> Upload Artifacts
              └─> Summary

CodeQL Analysis (parallel)
Security Audit (parallel)
Dependency Review (PRs only)
  └─> Security Summary
```

## 🔐 Required Secrets

### Optional (for enhanced features):
- `CODECOV_TOKEN`: Code coverage reporting (optional, public repos work without)

### Future (not currently used):
- Code signing certificates for macOS/Windows
- Package registry tokens
- Deployment credentials

## 🎯 Best Practices Implemented

Following Tabby and industry standards:

1. **Fail-Fast Linting**: Quick feedback on code quality
2. **Parallel Execution**: Independent jobs run concurrently
3. **Smart Caching**: Rust compilation cache across runs
4. **Cross-Platform Support**: Test on all target platforms
5. **Artifact Retention**: 30 days for nightly, permanent for releases
6. **Security First**: Automated vulnerability scanning
7. **Comprehensive Testing**: Unit, integration, and smoke tests
8. **Documentation**: Auto-generated and deployed docs

## 📝 Maintenance Notes

### Updating Workflow Dependencies

Actions are pinned to major versions (e.g., `@v4`). Update regularly:

```bash
# Check for outdated actions
gh actions list-outdated

# Update manually in workflow files
# Or use Dependabot to automate (recommended)
```

### Adding New Platforms

1. Add to matrix in `release.yml`, `nightly.yml`, and `ci.yml`
2. Update platform table in this README
3. Test thoroughly with manual workflow dispatch
4. Update documentation in main README.md

### Modifying Build Process

- Keep `ci.yml` and `release.yml` build steps in sync
- Test changes with `workflow_dispatch` first
- Monitor build times and optimize caching
- Update artifact names if changing versioning scheme

## 🆘 Troubleshooting

### Build Failures

**Rust compilation errors**:
- Check `Cargo.lock` is committed
- Verify dependencies compile on target platform
- Review cache usage (may need cache-busting)

**Cross-compilation issues**:
- Ensure `cross` tool is properly installed
- Check target triple is correct
- Review ARM toolchain availability

### Artifact Upload Failures

**Size limits**:
- GitHub has artifact size limits
- Strip binaries to reduce size
- Compress appropriately

**Permission errors**:
- Verify `contents: write` permission
- Check repository settings allow Actions
- Ensure GitHub token has necessary scopes

### Workflow Sync Issues

**When workflows don't trigger**:
- Verify branch protection rules
- Check workflow file syntax with `yamllint`
- Review repository Actions settings
- Ensure `.github/workflows/` path is correct

## 🔗 Related Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Tabby Terminal Workflows](https://github.com/Eugeny/tabby/tree/master/.github/workflows) (inspiration)
- [Cross-compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [nightly.link Documentation](https://nightly.link/)

---

**Note**: This workflow setup is inspired by and adapts patterns from the [Tabby Terminal](https://github.com/Eugeny/tabby) project, which has one of the most comprehensive CI/CD setups for cross-platform Rust applications. The workflow system has been optimized from 4 separate workflows to 3 focused, efficient workflows.

## 🔄 Workflows Overview

### 1. **CI** (`ci.yml`)

**Triggers:** Push to main/develop/feat/* branches, PRs to main/develop

Comprehensive continuous integration pipeline that validates code quality and functionality.

**Jobs:**

- **checks** (Ubuntu): Fast parallel checks
  - Code formatting (`cargo fmt`)
  - Linting (`cargo clippy`)
  - Documentation checks (`cargo doc`)
  
- **test** (Matrix: Ubuntu, macOS, Windows): Cross-platform testing
  - Build project
  - Run unit tests
  - Build release binary
  - Run integration tests (Unix only)
  
- **security** (Ubuntu, non-blocking): Security audit using `cargo-audit`
  
- **coverage** (Ubuntu, main/develop only, non-blocking): Code coverage with tarpaulin → Codecov

- **ci-success**: Final gate ensuring all required checks pass

**Features:**

- ✅ Modern Rust caching with `Swatinem/rust-cache`
- ✅ Parallel job execution for speed
- ✅ Matrix testing across all major platforms
- ✅ Optional jobs (security, coverage) don't block CI
- ✅ Integration tests run automatically on Unix platforms

### 2. **Version Management** (`version-bump.yml`)

**Triggers:**

- Push to main (automatic)
- Manual workflow dispatch (manual)

Unified version management supporting both automatic and manual version bumps.

**Automatic Mode** (on push to main):

- Analyzes commits using [Conventional Commits](https://www.conventionalcommits.org/)
- Auto-detects bump type:
  - `major`: BREAKING CHANGE or `!:` in commit
  - `minor`: `feat:` commits
  - `patch`: Other changes (`fix:`, `docs:`, etc.)
- Skip with `[skip version]` or `[skip ci]` in commit message

**Manual Mode** (workflow_dispatch):

- Select bump type: major, minor, or patch
- Optionally skip changelog generation
- Choose whether to create git tag

**Process:**

1. Detects/receives version bump type
2. Updates `VERSION`, `Cargo.toml`, `Cargo.lock`
3. Generates changelog from conventional commits
4. Commits changes with `[skip ci]`
5. Creates and pushes git tag (triggers release)

**Changelog Generation:**

- Categorizes commits by type: Breaking Changes, Features, Bug Fixes, Performance, Refactoring, Documentation, Maintenance
- Uses emoji prefixes for visual clarity
- Inserts into `CHANGELOG.md` in proper location

### 3. **Release** (`release.yml`)

**Triggers:**

- Push of tags matching `v*.*.*`
- Manual workflow dispatch with tag input

Builds and publishes release binaries for all supported platforms.

**Jobs:**

- **create-release**: Creates GitHub release with notes from changelog
  
- **build-binaries** (Matrix): Builds for 5 platforms:
  - macOS (Apple Silicon - aarch64-apple-darwin)
  - macOS (Intel - x86_64-apple-darwin)
  - Linux (x86_64 - x86_64-unknown-linux-gnu)
  - Linux (ARM64 - aarch64-unknown-linux-gnu) [cross-compiled]
  - Windows (x86_64 - x86_64-pc-windows-msvc)
  
- **finalize**: Generates release summary

**Artifacts:**

Each platform build produces:

- Compressed archive (`.tar.gz` for Unix, `.zip` for Windows)
- SHA256 checksum file
- Binary + README + LICENSE + CHANGELOG
- Platform-specific install script

**Features:**

- ✅ Cross-compilation support for ARM64 Linux
- ✅ Binary stripping for smaller size
- ✅ Automated install scripts included
- ✅ Comprehensive release notes with installation instructions
- ✅ SHA256 checksums for verification

## 🚀 Usage Examples

### Triggering a Version Bump (Manual)

Go to **Actions → Version Management → Run workflow**

Options:

- **bump_type**: `major`, `minor`, `patch`, or leave empty for auto-detect
- **create_tag**: Check to create tag and trigger release
- **skip_changelog**: Check to skip changelog update

### Creating a Release (Manual)

Go to **Actions → Release → Run workflow**

Input:

- **tag**: The version tag (e.g., `v1.0.0`)

### Automatic Flow

1. Make changes following [Conventional Commits](https://www.conventionalcommits.org/)

   ```bash
   git commit -m "feat: add new plugin system"
   git commit -m "fix: resolve memory leak in installer"
   git commit -m "feat!: redesign CLI interface" # Breaking change
   ```

2. Push to main

   ```bash
   git push origin main
   ```

3. **Version Management** workflow runs:
   - Detects commit types
   - Bumps version appropriately
   - Updates files and changelog
   - Creates and pushes tag

4. **Release** workflow triggers automatically:
   - Builds binaries for all platforms
   - Creates GitHub release
   - Uploads assets

5. Users can install:
   ```bash
   curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/scripts/install.sh | bash
   ```

## 🎯 Key Improvements

### Before (4 workflows)

- ❌ Duplicated version bump logic (2 workflows)
- ❌ Inefficient caching (manual cargo cache setup)
- ❌ Separate integration test jobs
- ❌ Blocking security/coverage jobs
- ❌ ~600+ lines of workflow YAML

### After (3 workflows)

- ✅ Unified version management (auto + manual)
- ✅ Modern Rust caching (`Swatinem/rust-cache`)
- ✅ Integrated tests in matrix
- ✅ Non-blocking optional checks
- ✅ ~400 lines of workflow YAML (~33% reduction)
- ✅ Better organized and maintainable
- ✅ Faster execution with parallel jobs

## 📋 Conventional Commit Format

For automatic version detection to work properly, use this format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Types:**

- `feat`: New feature (triggers minor bump)
- `fix`: Bug fix (triggers patch bump)
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Test updates
- `chore`: Maintenance tasks
- `ci`: CI/CD changes

**Breaking Changes:**

- Add `!` after type: `feat!: redesign API`
- Or add `BREAKING CHANGE:` in footer (triggers major bump)

**Examples:**
```bash
# Minor bump
git commit -m "feat(java): add Java 25 support"

# Patch bump
git commit -m "fix(installer): handle edge case in PATH setup"

# Major bump
git commit -m "feat!: redesign plugin architecture"

# Skip version bump
git commit -m "docs: update README [skip version]"
```

## 🔧 Configuration

### Required Secrets

- `GITHUB_TOKEN`: Automatically provided by GitHub Actions
- `CODECOV_TOKEN`: (Optional) For code coverage uploads

### Branch Protection

Recommended settings for `main` branch:

- ✅ Require status checks before merging
  - CI Success
- ✅ Require branches to be up to date
- ✅ Include administrators

## 📚 References

- [Conventional Commits](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust CI/CD Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)

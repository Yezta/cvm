# GitHub Actions Workflows

This directory contains streamlined CI/CD workflows for JCVM. The workflow system has been optimized from 4 separate workflows to 3 focused, efficient workflows.

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

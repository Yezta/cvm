# Version Management Guide

This document explains the automated version management system for JCVM, which follows [Semantic Versioning](https://semver.org/) principles.

## Table of Contents

- [Version Format](#version-format)
- [Automatic Version Bumping](#automatic-version-bumping)
- [Manual Version Bumping](#manual-version-bumping)
- [Local Development](#local-development)
- [Changelog Management](#changelog-management)
- [Best Practices](#best-practices)

## Version Format

JCVM follows **Semantic Versioning 2.0.0** (SemVer):

```txt
MAJOR.MINOR.PATCH
```

- **MAJOR**: Incremented for incompatible API changes or breaking changes
- **MINOR**: Incremented for new features in a backward-compatible manner
- **PATCH**: Incremented for backward-compatible bug fixes

Examples:

- `1.0.0` → `1.0.1` (patch: bug fix)
- `1.0.1` → `1.1.0` (minor: new feature)
- `1.1.0` → `2.0.0` (major: breaking change)

## Automatic Version Bumping

### Automatic Patch Bumps on Main

Every commit pushed to the `main` branch automatically triggers a **patch version bump**.

**Workflow:** `.github/workflows/version-bump.yml`

**Process:**

1. Detects new commits on `main` branch
2. Reads current version from `VERSION` file
3. Increments the patch number (e.g., `1.0.0` → `1.0.1`)
4. Updates:
   - `VERSION` file
   - `Cargo.toml` version
   - `Cargo.lock`
   - `CHANGELOG.md` with commit messages
5. Creates a new git tag (e.g., `v1.0.1`)
6. Pushes changes and tag back to the repository

**Exclusions:** The workflow ignores commits that only modify:

- `VERSION`
- `Cargo.toml`/`Cargo.lock`
- `CHANGELOG.md`
- Markdown files (`.md`)
- Documentation (`docs/**`)
- GitHub workflows (`.github/**`)

**Commit Message Convention:**
Version bump commits are marked with `[skip ci]` to prevent infinite loops:

```txt
chore: bump version to 1.0.1 [skip ci]
```

## Manual Version Bumping

### Via GitHub Actions (Recommended for Production)

For **major** or **minor** version bumps, use the manual GitHub Actions workflow.

**Workflow:** `.github/workflows/manual-version-bump.yml`

**Steps:**

1. Go to GitHub Actions tab in your repository
2. Select "Manual Version Bump" workflow
3. Click "Run workflow"
4. Choose the bump type:
   - **major** - For breaking changes (1.2.3 → 2.0.0)
   - **minor** - For new features (1.2.3 → 1.3.0)
   - **patch** - For bug fixes (1.2.3 → 1.2.4)
5. Optionally skip tag creation
6. Click "Run workflow"

**What It Does:**

- Updates `VERSION`, `Cargo.toml`, and `Cargo.lock`
- Updates `CHANGELOG.md` with appropriate section:
  - Major → "Breaking Changes"
  - Minor → "Added"
  - Patch → "Fixed"
- Creates and pushes git tag (unless skipped)
- Commits all changes to `main`

### When to Use Each Bump Type

**Major Version Bump (`major`):**

- Breaking API changes
- Removing deprecated features
- Incompatible changes to command-line interface
- Major architectural changes
- Changes that require user action

Examples:

```bash
# Changed CLI command structure
jcvm use <version>  →  jcvm version use <version>  # Breaking!
```

**Minor Version Bump (`minor`):**

- New features (backward-compatible)
- New command-line options
- New plugin support
- Performance improvements (significant)
- New tool support (e.g., adding Python support)

Examples:

```bash
# Added new command
jcvm cache clean  # New feature!

# Added new flag
jcvm install --verify-checksum  # New option!
```

**Patch Version Bump (`patch`):**

- Bug fixes
- Documentation updates
- Performance improvements (minor)
- Internal refactoring
- Security patches

Examples:

```bash
# Fixed bug in version detection
# Fixed download retry logic
# Updated error messages
```

## Local Development

### Using the Bump Script

For local development and testing, use the provided script:

```bash
# Bump patch version (1.2.3 → 1.2.4)
./scripts/bump-version.sh patch

# Bump minor version (1.2.3 → 1.3.0)
./scripts/bump-version.sh minor

# Bump major version (1.2.3 → 2.0.0)
./scripts/bump-version.sh major
```

**Interactive Process:**

1. Shows current version
2. Shows what the new version will be
3. Asks for confirmation
4. Updates all version files
5. Updates `CHANGELOG.md`
6. Shows git diff of changes
7. Optionally commits changes
8. Optionally creates and pushes tag

**What It Updates:**

- `VERSION` file
- `Cargo.toml` version field
- `Cargo.lock` (via `cargo update`)
- `CHANGELOG.md` with recent commits

### Manual Version Update

If you need to update versions manually:

1. **Update VERSION file:**

   ```bash
   echo "1.2.3" > VERSION
   ```

2. **Update Cargo.toml:**

   ```toml
   [package]
   version = "1.2.3"
   ```

3. **Update Cargo.lock:**

   ```bash
   cargo update -p jcvm
   ```

4. **Update CHANGELOG.md:**
   Add entry following the existing format.

5. **Commit and tag:**

   ```bash
   git add VERSION Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore: bump version to 1.2.3"
   git tag -a v1.2.3 -m "Release version 1.2.3"
   git push origin main --tags
   ```

## Changelog Management

### Format

The `CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog

## [Unreleased]

### Added
- New features go here

### Changed
- Changes to existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security fixes

## [1.2.3] - 2025-10-18

### Fixed
- Fixed bug in version detection (a1b2c3d)
- Updated error messages (e4f5g6h)
```

### Automated Updates

The version bump workflows automatically:

- Extract commit messages since last tag
- Format them as bullet points with commit hashes
- Place them under appropriate section based on bump type:
  - **Major** → "Breaking Changes"
  - **Minor** → "Added"
  - **Patch** → "Fixed"
- Insert after `[Unreleased]` section

### Manual Changelog Entries

For more descriptive changelog entries, you can manually edit `CHANGELOG.md` before releasing:

```markdown
## [Unreleased]

### Added
- Python plugin with version detection and installation
- Support for `.python-version` files
- New `jcvm python` command suite

### Fixed
- Fixed bug in Java version detection on Windows
- Improved error messages for network failures
```

## Best Practices

### Development Workflow

1. **Feature Development:**
   - Create feature branch: `git checkout -b feature/my-feature`
   - Make changes and commit with descriptive messages
   - Create Pull Request to `main`

2. **Release Preparation:**
   - Merge PR to `main`
   - Automatic patch bump occurs
   - For minor/major releases, use manual workflow

3. **Breaking Changes:**
   - Document in PR description
   - Update documentation
   - Use manual workflow to bump major version
   - Add migration guide if needed

### Commit Message Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/) for better changelog generation:

```bash
# Features
git commit -m "feat: add Python plugin support"
git commit -m "feat(cli): add --verify-checksum flag"

# Bug Fixes
git commit -m "fix: resolve version detection on Windows"
git commit -m "fix(installer): handle network timeouts"

# Breaking Changes
git commit -m "feat!: redesign CLI command structure

BREAKING CHANGE: Commands now use verb-noun pattern instead of noun-verb"

# Other
git commit -m "docs: update installation guide"
git commit -m "chore: update dependencies"
git commit -m "refactor: simplify download logic"
git commit -m "test: add integration tests for auto-switch"
```

### Version Tagging Strategy

- **All releases are tagged:** `v1.2.3`
- **Tags are immutable:** Never delete or modify existing tags
- **Pre-releases:** Use `-alpha`, `-beta`, `-rc` suffixes:
  - `v2.0.0-alpha.1`
  - `v2.0.0-beta.1`
  - `v2.0.0-rc.1`

### Release Process

1. **Prepare Release:**
   - Ensure all tests pass
   - Update documentation if needed
   - Review and clean up CHANGELOG.md

2. **Bump Version:**
   - Use manual workflow for major/minor
   - Automatic patch bump for hotfixes

3. **Verify Release:**
   - Check that tag was created
   - Verify CHANGELOG.md is updated
   - Ensure CI/CD pipeline runs

4. **Post-Release:**
   - Monitor for issues
   - Update release notes on GitHub
   - Announce release if major/minor

### Handling Hotfixes

For urgent bug fixes in production:

1. **Create hotfix branch from tag:**

   ```bash
   git checkout -b hotfix/critical-bug v1.2.3
   ```

2. **Fix and test:**

   ```bash
   git commit -m "fix: critical security issue"
   ```

3. **Merge to main:**

   ```bash
   git checkout main
   git merge hotfix/critical-bug
   git push origin main
   ```

4. **Automatic patch bump occurs** (1.2.3 → 1.2.4)

### Version Conflicts

If version conflicts occur:

1. **Pull latest changes:**

   ```bash
   git pull origin main
   ```

2. **Check current version:**

   ```bash
   cat VERSION
   ```

3. **Manually adjust if needed** using `./scripts/bump-version.sh`

## Troubleshooting

### Version Mismatch Between FILES

If `VERSION` and `Cargo.toml` are out of sync:

```bash
# Use the script to synchronize
./scripts/bump-version.sh patch

# Or manually update both files to match
```

### Failed Automatic Bump

If the automatic version bump workflow fails:

1. Check GitHub Actions logs
2. Look for permission issues
3. Verify `VERSION` file exists and is valid
4. Manually run: `./scripts/bump-version.sh patch`

### Tag Already Exists

If you get "tag already exists" error:

```bash
# List all tags
git tag

# Delete local tag
git tag -d v1.2.3

# Delete remote tag (use with caution!)
git push origin :refs/tags/v1.2.3

# Re-create tag
git tag -a v1.2.3 -m "Release version 1.2.3"
git push origin v1.2.3
```

## Summary

- **Automatic:** Patch bumps on every `main` commit
- **Manual:** Major/minor bumps via GitHub Actions workflow
- **Local:** Use `./scripts/bump-version.sh` for development
- **Format:** Semantic Versioning (MAJOR.MINOR.PATCH)
- **Changelog:** Automatically updated with commits
- **Tags:** Created for every version

For questions or issues, please open a GitHub issue or discussion.

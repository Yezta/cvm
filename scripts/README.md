# Scripts Directory

This directory contains utility scripts for JCVM development and maintenance.

## Version Management

### `bump-version.sh`

Interactive script for bumping version numbers during local development.

**Usage:**
```bash
./scripts/bump-version.sh [major|minor|patch]
```

**Examples:**
```bash
# Bump patch version: 1.2.3 → 1.2.4
./scripts/bump-version.sh patch

# Bump minor version: 1.2.3 → 1.3.0
./scripts/bump-version.sh minor

# Bump major version: 1.2.3 → 2.0.0
./scripts/bump-version.sh major
```

**What it does:**
1. Validates bump type
2. Reads current version from `VERSION` file
3. Calculates new version
4. Updates `VERSION` file
5. Updates `Cargo.toml` version field
6. Updates `Cargo.lock` via `cargo update`
7. Updates `CHANGELOG.md` with recent commits
8. Shows git diff of changes
9. Optionally commits changes
10. Optionally creates and pushes git tag

**Interactive Features:**
- Color-coded output
- Confirmation prompts at each step
- Preview changes before committing
- Safe defaults (asks before destructive operations)

**Requirements:**
- Must be run from project root directory
- Requires `VERSION` and `Cargo.toml` files
- Requires `cargo` to be installed
- Requires `git` for commit operations

**Cross-Platform:**
- Works on macOS (uses `sed -i ''`)
- Works on Linux (uses `sed -i`)

See [Version Management Guide](../docs/VERSION_MANAGEMENT.md) for more details.

## Future Scripts

This directory will contain additional utility scripts as needed:
- Release preparation scripts
- Testing automation
- Deployment helpers
- Documentation generators

## Contributing

When adding new scripts:
1. Make them executable: `chmod +x scripts/your-script.sh`
2. Add shebang line: `#!/usr/bin/env bash`
3. Include usage documentation
4. Handle errors gracefully with `set -e`
5. Add color-coded output for user-friendliness
6. Document the script in this README

# Contributing to JCVM

Thank you for your interest in contributing to JCVM! This document provides guidelines for contributing.

## How to Contribute

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Test your changes thoroughly
5. Commit your changes (`git commit -m 'Add some amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## Development Setup

1. Clone your fork:
## Getting Started

```bash
git clone https://github.com/Yezta/cvm.git
cd jcvm

2. Build the project:
```bash
cargo build
```

3. Test locally:
```bash
./target/debug/jcvm --help
```

4. Run tests:
```bash
cargo test
```

## Version Management

JCVM follows **Semantic Versioning** with automated version bumping:

- **Automatic Patch Bumps**: Every commit to `main` automatically bumps the patch version
- **Manual Major/Minor Bumps**: Use GitHub Actions workflow for major/minor versions
- **Local Development**: Use `./scripts/bump-version.sh [major|minor|patch]`

See [Version Management Guide](docs/VERSION_MANAGEMENT.md) for detailed information.

### Commit Message Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```bash
feat: add new feature
fix: resolve bug
docs: update documentation
chore: maintenance tasks
refactor: code refactoring
test: add tests
```

For breaking changes:
```bash
feat!: redesign CLI structure

BREAKING CHANGE: Commands now use verb-noun pattern
```

## Code Style

- Use 4 spaces for indentation
- Follow existing code style
- Add comments for complex logic
- Use descriptive variable names

## Testing

Before submitting a PR, please test:

- Installation on fresh system
- All commands work as expected
- Auto-switching with `.java-version` files
- Cross-platform compatibility (macOS, Linux)

## Pull Request Guidelines

- Keep PRs focused on a single feature or fix
- Update documentation if needed
- Add tests if applicable
- Reference any related issues

## Feature Requests

Open an issue with the tag `enhancement` to suggest new features.

## Bug Reports

When reporting bugs, please include:

- Your operating system and version
- Shell type (bash/zsh)
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

## Questions?

Feel free to open an issue with the tag `question`.

Thank you for contributing!

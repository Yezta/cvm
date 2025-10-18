# Testing Guide

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_version_parsing

# Run with output
cargo test -- --nocapture

# Run with backtrace
RUST_BACKTRACE=1 cargo test
```

## Test Coverage

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage

# View coverage
open coverage/index.html
```

## Test Organization

Unit tests are in each module using `#[cfg(test)]`. Integration tests go in the `tests/` directory.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = "21".parse::<Version>().unwrap();
        assert_eq!(v.major, 21);
    }
}
```

## Manual Testing Checklist

### Core Functions

- [ ] Install, use, and uninstall JDK versions
- [ ] Switch between versions
- [ ] Create and use aliases
- [ ] Set local project versions (.java-version)
- [ ] Detect and import existing Java installations

### Shell Integration

- [ ] Install shell hooks for bash/zsh/fish
- [ ] Auto-switch on directory change
- [ ] Verify JAVA_HOME updates correctly

### Error Handling

- [ ] Invalid version format
- [ ] Network errors
- [ ] Permission errors
- [ ] Corrupted downloads

## Best Practices

1. **Isolation**: Each test should be independent
2. **Clear Names**: Test names should describe what they test
3. **Aim for >80% coverage**
4. **Test error paths, not just happy paths**

## Contributing Tests

When contributing:

1. Add tests for new features
2. Update existing tests for changes
3. Ensure all tests pass before submitting PR
4. Document complex test scenarios

# Manual Shell Integration Test for .java-version Auto-Switch

This guide will help you manually test the `.java-version` auto-switch functionality in your shell.

## Prerequisites

1. JCVM must be installed and initialized in your shell
2. At least 2 Java versions must be installed via JCVM
3. Shell integration must be active (run `jcvm init` if not)

## Test Setup

First, ensure your shell integration is active:

```bash
# Check if JCVM is initialized
echo $JCVM_DIR
# Should output: /Users/singhard/personal/code/jcvm (or your installation path)

# Verify auto-switch function exists
type _jcvm_auto_switch
# Should show the function definition
```

## Test 1: Auto-Switch on Directory Change

### Setup Test Directories

```bash
# Create test directories in /tmp
mkdir -p /tmp/test-jcvm-project-a
mkdir -p /tmp/test-jcvm-project-b
mkdir -p /tmp/test-jcvm-project-no-version

# Create .java-version files with different versions
cd /tmp/test-jcvm-project-a
jcvm local 21.0.8

cd /tmp/test-jcvm-project-b
jcvm local 25

cd /tmp/test-jcvm-project-no-version
# No .java-version file
```

### Execute Tests

```bash
# Test 1: Enter project-a and verify version
cd /tmp/test-jcvm-project-a
java -version  # Should show Java 21.0.8
echo $JAVA_HOME  # Should point to JCVM's version 21.0.8

# Test 2: Switch to project-b
cd /tmp/test-jcvm-project-b
java -version  # Should show Java 25
echo $JAVA_HOME  # Should point to JCVM's version 25

# Test 3: Go back to project-a
cd /tmp/test-jcvm-project-a
java -version  # Should automatically switch back to 21.0.8

# Test 4: Go to directory without .java-version
cd /tmp/test-jcvm-project-no-version
java -version  # Should keep the current version (no auto-switch)

# Test 5: Rapid switching
cd /tmp/test-jcvm-project-a && java -version
cd /tmp/test-jcvm-project-b && java -version
cd /tmp/test-jcvm-project-a && java -version
# Should switch correctly each time
```

## Test 2: Shell Startup Auto-Detection

```bash
# Start in a directory with .java-version
cd /tmp/test-jcvm-project-a

# Open a new terminal/tab (or run a subshell)
# In zsh:
zsh

# In bash:
bash

# The new shell should automatically detect and switch to the version
java -version  # Should show Java 21.0.8
echo "from .java-version"
```

## Test 3: Nested Directories

```bash
# Create nested structure
cd /tmp/test-jcvm-project-a
mkdir -p src/main/java
cd src/main/java

# Should still use parent directory's .java-version
java -version  # Should show Java 21.0.8

# Create a nested .java-version that overrides
cd /tmp/test-jcvm-project-a/src
jcvm local 25

cd /tmp/test-jcvm-project-a/src/main
java -version  # Should show Java 25 (from src/.java-version)

cd /tmp/test-jcvm-project-a
java -version  # Should show Java 21.0.8 (from root .java-version)
```

## Test 4: Verify jcvm current Command

```bash
# Test that 'jcvm current' shows the right source
cd /tmp/test-jcvm-project-a
jcvm current
# Should output: 21.0.8 (from .java-version)

cd /tmp/test-jcvm-project-b
jcvm current
# Should output: 25 (from .java-version)

cd /tmp/test-jcvm-project-no-version
jcvm current
# Should output current version (without "from .java-version")
```

## Test 5: Error Handling

```bash
# Test with invalid version
cd /tmp
mkdir -p test-jcvm-invalid
cd test-jcvm-invalid
echo "99.99.99" > .java-version

# Try to switch
cd /tmp/test-jcvm-invalid
java -version
# Should not crash; might show error or keep current version

# Test with empty .java-version
echo "" > .java-version
cd /tmp/test-jcvm-invalid
# Should handle gracefully

# Test with malformed .java-version
echo "not-a-version" > .java-version
cd /tmp/test-jcvm-invalid
# Should handle gracefully
```

## Test 6: Manual Override

```bash
# Verify that manual 'jcvm use' overrides .java-version
cd /tmp/test-jcvm-project-a
# .java-version says 21.0.8

jcvm use 25
java -version  # Should show 25 (manual override)

# But changing directory triggers auto-switch again
cd /tmp
cd /tmp/test-jcvm-project-a
java -version  # Should be back to 21.0.8
```

## Test 7: Performance Test

```bash
# Test that auto-switching doesn't slow down cd
time cd /tmp/test-jcvm-project-a
time cd /tmp/test-jcvm-project-b
time cd /tmp/test-jcvm-project-a
# Should be fast (< 0.1s each)
```

## Expected Results

✅ **PASS Criteria:**

- Version switches automatically when entering directories with `.java-version`
- `java -version` shows correct version after `cd`
- `$JAVA_HOME` points to correct version
- `jcvm current` reports correct source
- No errors with invalid/missing `.java-version` files
- Switching is fast and seamless
- Works across nested directories
- Manual `jcvm use` temporarily overrides, but auto-switch reactivates on next `cd`

❌ **FAIL Indicators:**

- Version doesn't switch when entering directory
- Errors appear during normal `cd` operations
- `$JAVA_HOME` points to wrong location
- Auto-switch is slow (> 0.5s)
- Shell becomes unresponsive

## Cleanup

```bash
# Remove test directories
rm -rf /tmp/test-jcvm-project-a
rm -rf /tmp/test-jcvm-project-b
rm -rf /tmp/test-jcvm-project-no-version
rm -rf /tmp/test-jcvm-invalid
```

## Troubleshooting

If auto-switching doesn't work:

1. **Check initialization:**

   ```bash
   grep -A 20 "JCVM" ~/.zshrc  # or ~/.bashrc
   ```

2. **Reinitialize JCVM:**

   ```bash
   jcvm init
   source ~/.zshrc  # or source ~/.bashrc
   ```

3. **Verify function is loaded:**

   ```bash
   type _jcvm_auto_switch
   ```

4. **Check for conflicts:**

   ```bash
   # Other Java managers might interfere
   which java
   env | grep JAVA
   ```

5. **Enable debug mode:**

   ```bash
   # Add this to your shell config temporarily
   _jcvm_auto_switch() {
       echo "DEBUG: Checking for .java-version in $(pwd)"
       # ... rest of function
   }
   ```

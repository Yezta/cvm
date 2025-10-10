# Testing JCVM Locally

This guide will help you test JCVM on your local machine.

## Prerequisites

- macOS or Linux
- bash or zsh shell
- curl or wget
- tar
- Internet connection

## Quick Test

### 1. Test the script without installation

```bash
cd /Users/singhard/personal/code/jcvm
./test.sh
```

This runs basic tests without downloading any JDKs.

### 2. Manual local testing

```bash
# Source the script directly
export JCVM_DIR="$HOME/.jcvm-test"
source ./jcvm.sh

# Test help
jcvm help

# Test list (should be empty)
jcvm list

# Test list-remote (requires internet)
jcvm list-remote
```

### 3. Clean up test environment

```bash
rm -rf ~/.jcvm-test
unset JCVM_DIR
```

## Full Installation Test

### 1. Test the installer

```bash
# Set test directory
export JCVM_DIR="$HOME/.jcvm-test"

# Run installer
./install.sh

# Source the configuration
source ~/.zshrc  # or ~/.bashrc
```

### 2. Test basic commands

```bash
# Show help
jcvm help

# List remote versions (requires internet)
jcvm list-remote

# Install a version (this will download ~200MB)
jcvm install 21

# List installed versions
jcvm list

# Use the version
jcvm use 21

# Check it works
java -version
echo $JAVA_HOME
```

### 3. Test auto-switching

```bash
# Create a test project
mkdir -p /tmp/test-project
cd /tmp/test-project

# Set local version
jcvm local 21
cat .java-version

# Leave and return
cd ~
cd /tmp/test-project

# Should automatically use JDK 21
jcvm current
```

### 4. Test alias

```bash
# Set default
jcvm alias default 21

# Check alias
ls -la ~/.jcvm-test/alias/
```

### 5. Clean up

```bash
# Remove test installation
rm -rf ~/.jcvm-test

# Remove from shell config (manual)
# Edit ~/.zshrc or ~/.bashrc and remove JCVM lines

# Reload shell
source ~/.zshrc
```

## Testing Checklist

### Installation

- [ ] Installer creates directory structure
- [ ] Script is executable
- [ ] Shell configuration is updated
- [ ] Help command works

### Version Management

- [ ] `list-remote` shows available versions
- [ ] `install` downloads and extracts JDK
- [ ] `list` shows installed versions
- [ ] `use` switches to a version
- [ ] `current` shows active version
- [ ] `uninstall` removes a version

### Auto-Switching

- [ ] `.java-version` file is created with `local`
- [ ] Version switches when entering directory
- [ ] Works in bash
- [ ] Works in zsh

### Aliases

- [ ] `alias default` creates symlink
- [ ] Default version loads on new shell
- [ ] Current alias updates on `use`

### Error Handling

- [ ] Installing non-existent version shows error
- [ ] Using non-installed version shows error
- [ ] Helpful error messages
- [ ] Graceful handling of network errors

### Cross-Platform

- [ ] Works on macOS (Intel)
- [ ] Works on macOS (Apple Silicon)
- [ ] Works on Linux (x86_64)
- [ ] Works on Linux (ARM64)

## Integration Testing

### With Build Tools

#### Maven

```bash
cd /tmp/test-project
jcvm use 21

# Create simple Maven project
cat > pom.xml << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>test</artifactId>
    <version>1.0</version>
    <properties>
        <maven.compiler.source>21</maven.compiler.source>
        <maven.compiler.target>21</maven.compiler.target>
    </properties>
</project>
EOF

# Create source file
mkdir -p src/main/java/com/example
cat > src/main/java/com/example/Main.java << 'EOF'
package com.example;

public class Main {
    public static void main(String[] args) {
        System.out.println("Java version: " + System.getProperty("java.version"));
        System.out.println("JCVM works!");
    }
}
EOF

# Build and run
mvn compile
mvn exec:java -Dexec.mainClass="com.example.Main"
```

#### Gradle

```bash
cd /tmp/test-project
jcvm use 21

# Create simple Gradle project
cat > build.gradle << 'EOF'
plugins {
    id 'java'
    id 'application'
}

group = 'com.example'
version = '1.0'

sourceCompatibility = '21'
targetCompatibility = '21'

application {
    mainClass = 'com.example.Main'
}
EOF

# Use same source file as Maven example above

# Build and run
./gradlew build
./gradlew run
```

### Multiple Terminals

1. Open Terminal 1:
```bash
jcvm use 21
java -version  # Should show 21
```

2. Open Terminal 2:
```bash
jcvm use 17
java -version  # Should show 17
```

3. Check Terminal 1:
```bash
java -version  # Should still show 21
```

### IDE Integration

Test that IDEs can detect JAVA_HOME:

```bash
jcvm use 21
echo $JAVA_HOME

# Open IntelliJ IDEA
idea .

# Or VSCode
code .

# Or Eclipse
eclipse
```

The IDE should detect the JDK from JAVA_HOME.

## Performance Testing

### Installation Speed

```bash
time jcvm install 21
# Should complete in < 2 minutes on good connection
```

### Switching Speed

```bash
time jcvm use 21
# Should be < 1 second
```

### Auto-Switch Speed

```bash
time (cd /tmp/test-project)  # with .java-version file
# Should be < 1 second
```

## Debugging

### Enable Debug Mode

Add to beginning of jcvm.sh:

```bash
set -x  # Print commands as they execute
```

### Check Environment

```bash
# Check all JCVM-related environment
env | grep -i jcvm
env | grep -i java

# Check PATH
echo $PATH | tr ':' '\n' | grep -n .

# Check installed versions
ls -la ~/.jcvm/versions/

# Check aliases
ls -la ~/.jcvm/alias/
```

### Common Issues

**Issue**: Command not found
```bash
# Check if script is sourced
declare -f jcvm

# Check if in PATH
which jcvm

# Re-source
source ~/.zshrc
```

**Issue**: Version not switching
```bash
# Check current setup
jcvm current
echo $JAVA_HOME
which java

# Force switch
jcvm use 21
source ~/.zshrc
```

**Issue**: Download fails
```bash
# Check internet
ping -c 3 adoptium.net

# Check API
curl -I https://api.adoptium.net/v3/info/available_releases

# Try manual download
curl -L -o /tmp/test.tar.gz "https://api.adoptium.net/v3/binary/latest/21/ga/mac/x64/jdk/hotspot/normal/eclipse"
```

## Automated Testing Script

Save as `full-test.sh`:

```bash
#!/usr/bin/env bash
set -e

echo "=== JCVM Full Test Suite ==="

# Setup
export JCVM_DIR="$HOME/.jcvm-test"
source ./jcvm.sh

# Test 1: Installation
echo "Test 1: Installing JDK 21..."
jcvm install 21
echo "✓ Installation complete"

# Test 2: Listing
echo "Test 2: Listing versions..."
jcvm list | grep -q "21"
echo "✓ Version listed"

# Test 3: Using
echo "Test 3: Using version..."
jcvm use 21
java -version 2>&1 | grep -q "21"
echo "✓ Version active"

# Test 4: Local version
echo "Test 4: Local version..."
cd /tmp
mkdir -p jcvm-test-local
cd jcvm-test-local
jcvm local 21
test -f .java-version
echo "✓ Local version set"

# Test 5: Alias
echo "Test 5: Alias..."
jcvm alias default 21
test -L "$JCVM_DIR/alias/default"
echo "✓ Alias created"

# Cleanup
echo "Cleanup..."
rm -rf "$JCVM_DIR"
rm -rf /tmp/jcvm-test-local

echo "=== All tests passed! ==="
```

Make it executable and run:

```bash
chmod +x full-test.sh
./full-test.sh
```

## Report Issues

If you find bugs during testing:

1. Note your environment:
   - OS and version
   - Shell and version
   - JCVM version

2. Provide steps to reproduce

3. Include error messages

4. Open an issue on GitHub

---

**Happy Testing!** 🧪

# Examples

This directory contains example usage scenarios for JCVM.

## Basic Workflow

### 1. Install and Switch Between Versions

```bash
# List available versions
jcvm list-remote

# Install JDK 21 (LTS)
jcvm install 21

# Install JDK 17 (LTS)
jcvm install 17

# List installed versions
jcvm list

# Switch to JDK 21
jcvm use 21

# Check current version
jcvm current
java -version

# Switch to JDK 17
jcvm use 17
```

### 2. Project-Specific Version

```bash
# Navigate to your project
cd ~/projects/my-java-app

# Set JDK version for this project
jcvm local 17

# This creates a .java-version file
cat .java-version  # Shows: 17

# Now whenever you cd into this directory, JDK 17 will be used automatically
cd ..
cd my-java-app  # Automatically switches to JDK 17
```

### 3. Set Global Default

```bash
# Set JDK 21 as the default version for new shells
jcvm alias default 21

# Now every new terminal will use JDK 21 by default
```

## Advanced Examples

### Multi-Project Setup

```bash
# Project A uses JDK 17
cd ~/projects/legacy-app
jcvm local 17

# Project B uses JDK 21
cd ~/projects/modern-app
jcvm local 21

# Switching between projects automatically changes JDK version
cd ~/projects/legacy-app  # Uses JDK 17
cd ~/projects/modern-app  # Uses JDK 21
```

### Testing Against Multiple Versions

```bash
# Test script for multiple JDK versions
for version in 17 21; do
    echo "Testing with JDK $version"
    jcvm use $version
    ./gradlew test
done
```

### CI/CD Integration

```bash
# In your CI script
export JCVM_DIR="$HOME/.jcvm"
curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
source ~/.bashrc

# Install required version
jcvm install 21
jcvm use 21

# Run build
./mvnw clean package
```

### Docker Integration

```dockerfile
FROM ubuntu:22.04

# Install JCVM
RUN apt-get update && apt-get install -y curl
RUN curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash

# Install JDK
RUN . ~/.bashrc && jcvm install 21 && jcvm alias default 21

# Your app
COPY . /app
WORKDIR /app
RUN . ~/.bashrc && ./mvnw package
```

## Shell Configuration Examples

### Zsh Configuration (~/.zshrc)

```bash
# JCVM
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"

# Optional: Show Java version in prompt
autoload -U colors && colors
RPROMPT='%{$fg[cyan]%}$(jcvm current 2>/dev/null | head -1)%{$reset_color%}'
```

### Bash Configuration (~/.bashrc)

```bash
# JCVM
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"

# Optional: Show Java version in prompt
export PS1='\u@\h:\w $(jcvm current 2>/dev/null | head -1)\$ '
```

## Troubleshooting Examples

### Fix PATH Issues

```bash
# If java command is not found after switching
jcvm use 21
echo $JAVA_HOME  # Should show ~/.jcvm/versions/21
echo $PATH | tr ':' '\n' | grep jcvm  # Should show JCVM path

# Reload shell
source ~/.bashrc  # or ~/.zshrc
```

### Clean Install

```bash
# Remove all JCVM data
rm -rf ~/.jcvm

# Reinstall
curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
source ~/.bashrc
```

### List All Versions with Details

```bash
# Custom script to show detailed version info
for version in ~/.jcvm/versions/*; do
    if [ -d "$version" ]; then
        echo "=== $(basename $version) ==="
        $version/bin/java -version 2>&1 | head -3
        echo ""
    fi
done
```

# Quick Start Guide

Get up and running with JCVM in 5 minutes!

## Step 1: Install JCVM

### Option A: Using the install script (recommended)

```bash
curl -o- https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash
```

### Option B: Manual installation

```bash
git clone https://github.com/yourusername/jcvm.git ~/.jcvm
cd ~/.jcvm
chmod +x jcvm.sh
```

Add to your `~/.zshrc` or `~/.bashrc`:

```bash
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"
```

Reload your shell:

```bash
source ~/.zshrc  # or ~/.bashrc
```

## Step 2: Verify Installation

```bash
jcvm help
```

You should see the help menu with all available commands.

## Step 3: Install Your First JDK

List available versions:

```bash
jcvm list-remote
```

Install JDK 21 (latest LTS):

```bash
jcvm install 21
```

This will:
- Download JDK 21 from Eclipse Temurin
- Extract it to `~/.jcvm/versions/21`
- Make it available for use

## Step 4: Use the JDK

Activate JDK 21:

```bash
jcvm use 21
```

Verify it's working:

```bash
java -version
echo $JAVA_HOME
```

## Step 5: Set as Default (Optional)

Make JDK 21 your default version:

```bash
jcvm alias default 21
```

Now every new terminal will use JDK 21 automatically!

## Step 6: Project-Specific Version (Optional)

Navigate to your project and set a specific JDK version:

```bash
cd ~/my-java-project
jcvm local 17
```

This creates a `.java-version` file. Whenever you enter this directory, JCVM will automatically switch to JDK 17!

## Common Commands Cheat Sheet

```bash
# List available versions to install
jcvm list-remote

# Install a version
jcvm install 21
jcvm install 17
jcvm install 11

# List installed versions
jcvm list

# Switch to a version
jcvm use 21

# Show current version
jcvm current

# Set project version
jcvm local 17

# Set default version
jcvm alias default 21

# Uninstall a version
jcvm uninstall 11

# Get help
jcvm help
```

## What's Next?

- Read the full [README.md](README.md) for detailed documentation
- Check out [examples](examples/README.md) for advanced use cases
- Join the community and contribute!

## Troubleshooting

### Command not found

Make sure you've added JCVM to your shell profile and reloaded it:

```bash
source ~/.zshrc  # or ~/.bashrc
```

### Version not switching

Check if JCVM is properly loaded:

```bash
echo $JCVM_DIR  # Should show ~/.jcvm
jcvm current    # Should show active version
```

### Need help?

- Check existing [issues](https://github.com/yourusername/jcvm/issues)
- Open a new issue if needed
- Read [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines

---

**Happy coding with JCVM! 🚀**

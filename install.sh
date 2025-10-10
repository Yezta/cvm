#!/usr/bin/env bash

# JCVM Installation Script

set -e

JCVM_DIR="${JCVM_DIR:-$HOME/.jcvm}"
JCVM_REPO="https://github.com/yourusername/jcvm.git"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo_info() {
    echo -e "${BLUE}$*${NC}"
}

echo_success() {
    echo -e "${GREEN}$*${NC}"
}

echo_error() {
    echo -e "${RED}$*${NC}"
}

echo_warning() {
    echo -e "${YELLOW}$*${NC}"
}

# Check if JCVM is already installed
if [ -d "$JCVM_DIR" ]; then
    echo_warning "JCVM is already installed at $JCVM_DIR"
    read -p "Do you want to reinstall? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo_info "Installation cancelled"
        exit 0
    fi
    rm -rf "$JCVM_DIR"
fi

echo_info "Installing JCVM to $JCVM_DIR..."

# Check if git is available
if command -v git >/dev/null 2>&1; then
    echo_info "Cloning JCVM repository..."
    git clone "$JCVM_REPO" "$JCVM_DIR"
else
    echo_info "Git not found, downloading archive..."
    if command -v curl >/dev/null 2>&1; then
        curl -L -o /tmp/jcvm.tar.gz "https://github.com/yourusername/jcvm/archive/refs/heads/main.tar.gz"
    elif command -v wget >/dev/null 2>&1; then
        wget -O /tmp/jcvm.tar.gz "https://github.com/yourusername/jcvm/archive/refs/heads/main.tar.gz"
    else
        echo_error "Error: Neither git, curl, nor wget found. Please install one of them."
        exit 1
    fi
    
    mkdir -p "$JCVM_DIR"
    tar -xzf /tmp/jcvm.tar.gz -C "$JCVM_DIR" --strip-components=1
    rm /tmp/jcvm.tar.gz
fi

# Make script executable
chmod +x "$JCVM_DIR/jcvm.sh"

# Detect shell
SHELL_PROFILE=""
if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zshrc" ]; then
    SHELL_PROFILE="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ] || [ -f "$HOME/.bashrc" ]; then
    SHELL_PROFILE="$HOME/.bashrc"
elif [ -f "$HOME/.bash_profile" ]; then
    SHELL_PROFILE="$HOME/.bash_profile"
elif [ -f "$HOME/.profile" ]; then
    SHELL_PROFILE="$HOME/.profile"
fi

if [ -z "$SHELL_PROFILE" ]; then
    echo_warning "Could not detect shell profile. Please manually add the following to your shell configuration:"
    echo ""
    echo "export JCVM_DIR=\"$JCVM_DIR\""
    echo "[ -s \"\$JCVM_DIR/jcvm.sh\" ] && \\. \"\$JCVM_DIR/jcvm.sh\""
    echo ""
else
    # Check if already configured
    if grep -q "JCVM_DIR" "$SHELL_PROFILE" 2>/dev/null; then
        echo_warning "JCVM already configured in $SHELL_PROFILE"
    else
        echo_info "Adding JCVM to $SHELL_PROFILE..."
        cat >> "$SHELL_PROFILE" << 'EOF'

# JCVM - Java Configuration & Version Manager
export JCVM_DIR="$HOME/.jcvm"
[ -s "$JCVM_DIR/jcvm.sh" ] && \. "$JCVM_DIR/jcvm.sh"
EOF
        echo_success "✓ Added JCVM configuration to $SHELL_PROFILE"
    fi
fi

echo ""
echo_success "✓ JCVM installation complete!"
echo ""
echo_info "To start using JCVM, run:"
echo "  source $SHELL_PROFILE"
echo ""
echo_info "Or open a new terminal window."
echo ""
echo_info "Quick start:"
echo "  jcvm list-remote    # See available JDK versions"
echo "  jcvm install 21     # Install JDK 21"
echo "  jcvm use 21         # Use JDK 21"
echo ""
echo_info "For help, run:"
echo "  jcvm help"

#!/usr/bin/env bash

# JCVM - Quick Installation Script
# Usage: curl -fsSL https://raw.githubusercontent.com/yourusername/jcvm/main/install.sh | bash

set -e

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO_URL="${REPO_URL:-https://github.com/yourusername/jcvm.git}"
TMP_DIR="/tmp/jcvm-install-$$"

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

echo_info "🦀 Installing JCVM (Java Configuration & Version Manager)..."
echo ""

# Check if Rust is installed
if ! command -v cargo >/dev/null 2>&1; then
    echo_warning "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo_success "✅ Rust installed successfully"
fi

echo_info "Rust version: $(rustc --version)"
echo ""

# Clone repository
echo_info "Cloning repository..."
git clone "$REPO_URL" "$TMP_DIR"
cd "$TMP_DIR"

# Build in release mode
echo_info "Building JCVM (this may take a few minutes)..."
cargo build --release

# Create install directory
mkdir -p "$INSTALL_DIR"

# Install binary
echo_info "Installing to $INSTALL_DIR..."
cp target/release/jcvm "$INSTALL_DIR/jcvm"
chmod +x "$INSTALL_DIR/jcvm"

# Clean up
cd "$HOME"
rm -rf "$TMP_DIR"

echo ""
echo_success "✅ JCVM installed successfully!"
echo ""

# Check if in PATH
if ! echo "$PATH" | grep -q "$INSTALL_DIR"; then
    echo_warning "⚠️  $INSTALL_DIR is not in your PATH"
    echo ""
    echo "Add this to your shell config:"
    echo ""
    echo "  # For Bash (~/.bashrc)"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
    echo "  # For Zsh (~/.zshrc)"
    echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    echo ""
    echo "  # For Fish (~/.config/fish/config.fish)"
    echo "  set -gx PATH $INSTALL_DIR \$PATH"
    echo ""
else
    echo_info "✨ $INSTALL_DIR is already in your PATH"
    echo ""
fi

echo "Next steps:"
echo "  1. Run: jcvm shell-init"
echo "  2. Reload your shell: source ~/.zshrc (or ~/.bashrc)"
echo "  3. Start using: jcvm list-remote"
echo ""
echo "For help: jcvm --help"

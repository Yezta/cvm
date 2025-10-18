#!/usr/bin/env bash

# JCVM - Binary Installation Script
# Downloads and installs pre-built JCVM binaries
# Usage: curl -fsSL https://raw.githubusercontent.com/Yezta/cvm/main/install-binary.sh | bash

set -e

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
GITHUB_REPO="${GITHUB_REPO:-Yezta/cvm}"
VERSION="${VERSION:-latest}"

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

# Detect OS and architecture
detect_platform() {
    local os=""
    local arch=""
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)     os="unknown-linux-gnu" ;;
        Darwin*)    os="apple-darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="pc-windows-msvc" ;;
        *)
            echo_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)   arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        *)
            echo_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac
    
    echo "${arch}-${os}"
}

# Get latest version from GitHub
get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
    
    if [ -z "$version" ]; then
        echo_error "Failed to get latest version"
        exit 1
    fi
    
    echo "$version"
}

# Download and install
install_jcvm() {
    local platform
    local version
    local archive_name
    local download_url
    local tmp_dir
    
    echo_info "🦀 Installing JCVM (Java Configuration & Version Manager)..."
    echo ""
    
    # Detect platform
    platform=$(detect_platform)
    echo_info "Detected platform: $platform"
    
    # Get version
    if [ "$VERSION" = "latest" ]; then
        version=$(get_latest_version)
        echo_info "Latest version: $version"
    else
        version="$VERSION"
        echo_info "Installing version: $version"
    fi
    
    # Determine archive format
    if [[ "$platform" == *"windows"* ]]; then
        archive_name="jcvm-v${version}-${platform}.zip"
    else
        archive_name="jcvm-v${version}-${platform}.tar.gz"
    fi
    
    download_url="https://github.com/${GITHUB_REPO}/releases/download/v${version}/${archive_name}"
    
    echo_info "Downloading from: $download_url"
    echo ""
    
    # Create temporary directory
    tmp_dir=$(mktemp -d)
    cd "$tmp_dir"
    
    # Download
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$archive_name" "$download_url"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$archive_name" "$download_url"
    else
        echo_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi
    
    # Extract
    echo_info "Extracting archive..."
    if [[ "$archive_name" == *.tar.gz ]]; then
        tar xzf "$archive_name"
    elif [[ "$archive_name" == *.zip ]]; then
        unzip -q "$archive_name"
    fi
    
    # Find the extracted directory
    extracted_dir=$(find . -maxdepth 1 -type d -name "jcvm-v${version}-*" | head -n 1)
    
    if [ -z "$extracted_dir" ]; then
        echo_error "Failed to find extracted directory"
        exit 1
    fi
    
    # Install binary
    mkdir -p "$INSTALL_DIR"
    
    if [[ "$platform" == *"windows"* ]]; then
        cp "$extracted_dir/jcvm.exe" "$INSTALL_DIR/jcvm.exe"
        echo_success "✅ JCVM installed to $INSTALL_DIR/jcvm.exe"
    else
        cp "$extracted_dir/jcvm" "$INSTALL_DIR/jcvm"
        chmod +x "$INSTALL_DIR/jcvm"
        echo_success "✅ JCVM installed to $INSTALL_DIR/jcvm"
    fi
    
    # Cleanup
    cd - >/dev/null
    rm -rf "$tmp_dir"
    
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
    
    # Verify installation
    if command -v jcvm >/dev/null 2>&1; then
        echo_success "✅ Installation verified!"
        echo ""
        jcvm --version
    else
        echo_warning "⚠️  jcvm command not found in PATH"
        echo "You may need to reload your shell or add $INSTALL_DIR to your PATH"
    fi
    
    echo ""
    echo "Next steps:"
    echo "  1. Run: jcvm shell-init"
    echo "  2. Reload your shell: source ~/.zshrc (or ~/.bashrc)"
    echo "  3. Check available tools: jcvm list-remote --tool <java|node|python>"
    echo ""
    echo "For help: jcvm --help"
}

# Run installation
install_jcvm

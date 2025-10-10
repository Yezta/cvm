#!/usr/bin/env bash

# JCVM - Java Configuration & Version Manager
# A JDK version manager inspired by NVM

# Exit on error in strict mode (commented out for interactive use)
# set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default directories
export JCVM_DIR="${JCVM_DIR:-$HOME/.jcvm}"
export JCVM_VERSIONS_DIR="$JCVM_DIR/versions"
export JCVM_ALIAS_DIR="$JCVM_DIR/alias"
export JCVM_CACHE_DIR="$JCVM_DIR/cache"

# Adoptium API base URL
ADOPTIUM_API="https://api.adoptium.net/v3"

# Initialize JCVM directories
jcvm_init() {
    mkdir -p "$JCVM_VERSIONS_DIR"
    mkdir -p "$JCVM_ALIAS_DIR"
    mkdir -p "$JCVM_CACHE_DIR"
}

# Print colored message
jcvm_echo() {
    local color="$1"
    shift
    echo -e "${color}$*${NC}"
}

# Get OS type
jcvm_get_os() {
    case "$(uname -s)" in
        Darwin*) echo "mac" ;;
        Linux*)  echo "linux" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *) echo "unknown" ;;
    esac
}

# Get architecture
jcvm_get_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64) echo "x64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) echo "$arch" ;;
    esac
}

# List available remote versions
jcvm_list_remote() {
    local feature_version="${1:-}"
    
    jcvm_echo "$BLUE" "Fetching available JDK versions from Adoptium..."
    
    local url="$ADOPTIUM_API/info/available_releases"
    local response
    
    if command -v curl >/dev/null 2>&1; then
        response=$(curl -sS "$url")
    elif command -v wget >/dev/null 2>&1; then
        response=$(wget -qO- "$url")
    else
        jcvm_echo "$RED" "Error: curl or wget is required"
        return 1
    fi
    
    # Parse available versions (simple parsing, would need jq for better parsing)
    echo ""
    jcvm_echo "$GREEN" "Available JDK Versions:"
    echo "$response" | grep -o '"available_releases":\[[^]]*\]' | grep -o '[0-9]\+' | sort -rn | while read -r version; do
        local marker=""
        # Check if version is LTS
        if [[ "$version" == "21" || "$version" == "17" || "$version" == "11" || "$version" == "8" ]]; then
            marker=" ${GREEN}(LTS)${NC}"
        fi
        echo -e "  $version$marker"
    done
    
    echo ""
    jcvm_echo "$YELLOW" "Usage: jcvm install <version>"
    jcvm_echo "$YELLOW" "Example: jcvm install 21"
}

# Install a JDK version
jcvm_install() {
    local version="$1"
    
    if [[ -z "$version" ]]; then
        jcvm_echo "$RED" "Error: Please specify a version to install"
        jcvm_echo "$YELLOW" "Usage: jcvm install <version>"
        return 1
    fi
    
    local os
    local arch
    os=$(jcvm_get_os)
    arch=$(jcvm_get_arch)
    
    if [[ "$os" == "unknown" ]]; then
        jcvm_echo "$RED" "Error: Unsupported operating system"
        return 1
    fi
    
    jcvm_echo "$BLUE" "Installing JDK $version for $os-$arch..."
    
    # Determine image type based on OS
    local image_type="jdk"
    local file_extension
    case "$os" in
        mac)
            file_extension="tar.gz"
            ;;
        linux)
            file_extension="tar.gz"
            ;;
        windows)
            file_extension="zip"
            ;;
    esac
    
    # Construct download URL
    local api_url="$ADOPTIUM_API/assets/latest/$version/hotspot"
    jcvm_echo "$BLUE" "Querying: $api_url"
    
    local download_url
    local download_info
    
    if command -v curl >/dev/null 2>&1; then
        download_info=$(curl -sS "$api_url")
    else
        download_info=$(wget -qO- "$api_url")
    fi
    
    # Extract download link for the specific OS and architecture
    # This is a simplified version - in production, you'd use jq for proper JSON parsing
    download_url=$(echo "$download_info" | grep -o '"link":"[^"]*"' | head -1 | cut -d'"' -f4)
    
    if [[ -z "$download_url" ]]; then
        jcvm_echo "$RED" "Error: Could not find download URL for JDK $version"
        jcvm_echo "$YELLOW" "Try: jcvm list-remote to see available versions"
        return 1
    fi
    
    # Download
    local filename="${version}-${os}-${arch}.${file_extension}"
    local download_path="$JCVM_CACHE_DIR/$filename"
    
    jcvm_echo "$BLUE" "Downloading from: $download_url"
    
    if command -v curl >/dev/null 2>&1; then
        curl -# -L -o "$download_path" "$download_url"
    else
        wget --show-progress -O "$download_path" "$download_url"
    fi
    
    if [[ ! -f "$download_path" ]]; then
        jcvm_echo "$RED" "Error: Download failed"
        return 1
    fi
    
    # Extract
    local install_dir="$JCVM_VERSIONS_DIR/$version"
    mkdir -p "$install_dir"
    
    jcvm_echo "$BLUE" "Extracting to $install_dir..."
    
    case "$file_extension" in
        tar.gz)
            tar -xzf "$download_path" -C "$install_dir" --strip-components=1
            ;;
        zip)
            unzip -q "$download_path" -d "$install_dir"
            ;;
    esac
    
    if [[ $? -eq 0 ]]; then
        jcvm_echo "$GREEN" "✓ Successfully installed JDK $version"
        jcvm_echo "$YELLOW" "Run 'jcvm use $version' to activate this version"
        
        # Clean up download
        rm -f "$download_path"
    else
        jcvm_echo "$RED" "Error: Extraction failed"
        rm -rf "$install_dir"
        return 1
    fi
}

# List installed versions
jcvm_list() {
    if [[ ! -d "$JCVM_VERSIONS_DIR" ]] || [[ -z "$(ls -A "$JCVM_VERSIONS_DIR" 2>/dev/null)" ]]; then
        jcvm_echo "$YELLOW" "No JDK versions installed"
        jcvm_echo "$YELLOW" "Use 'jcvm install <version>' to install a version"
        return 0
    fi
    
    local current_version
    current_version=$(jcvm_current_version)
    
    jcvm_echo "$GREEN" "Installed JDK versions:"
    
    for version_dir in "$JCVM_VERSIONS_DIR"/*; do
        if [[ -d "$version_dir" ]]; then
            local version
            version=$(basename "$version_dir")
            
            if [[ "$version" == "$current_version" ]]; then
                echo -e "  ${GREEN}* $version${NC} (currently active)"
            else
                echo "  $version"
            fi
        fi
    done
    
    # Show default alias if set
    if [[ -L "$JCVM_ALIAS_DIR/default" ]]; then
        local default_version
        default_version=$(basename "$(readlink "$JCVM_ALIAS_DIR/default")")
        echo ""
        jcvm_echo "$BLUE" "Default version: $default_version"
    fi
}

# Get current version from JAVA_HOME
jcvm_current_version() {
    if [[ -n "${JAVA_HOME:-}" ]] && [[ "$JAVA_HOME" == "$JCVM_VERSIONS_DIR"* ]]; then
        basename "$JAVA_HOME"
    elif [[ -L "$JCVM_ALIAS_DIR/current" ]]; then
        basename "$(readlink "$JCVM_ALIAS_DIR/current")"
    fi
}

# Show current version
jcvm_current() {
    local version
    version=$(jcvm_current_version)
    
    if [[ -n "$version" ]]; then
        jcvm_echo "$GREEN" "Current JDK version: $version"
        if command -v java >/dev/null 2>&1; then
            java -version 2>&1 | head -1
        fi
    else
        jcvm_echo "$YELLOW" "No JCVM version currently active"
        if command -v java >/dev/null 2>&1; then
            jcvm_echo "$YELLOW" "Using system Java:"
            java -version 2>&1 | head -1
        else
            jcvm_echo "$YELLOW" "No Java found in PATH"
        fi
    fi
}

# Use a specific version
jcvm_use() {
    local version="$1"
    
    if [[ -z "$version" ]]; then
        jcvm_echo "$RED" "Error: Please specify a version to use"
        jcvm_echo "$YELLOW" "Usage: jcvm use <version>"
        return 1
    fi
    
    local version_dir="$JCVM_VERSIONS_DIR/$version"
    
    if [[ ! -d "$version_dir" ]]; then
        jcvm_echo "$RED" "Error: JDK version $version is not installed"
        jcvm_echo "$YELLOW" "Use 'jcvm install $version' to install it"
        return 1
    fi
    
    # Set JAVA_HOME
    export JAVA_HOME="$version_dir"
    
    # Update PATH - remove old JCVM paths and add new one
    PATH=$(echo "$PATH" | tr ':' '\n' | grep -v "$JCVM_VERSIONS_DIR" | tr '\n' ':')
    export PATH="$JAVA_HOME/bin:$PATH"
    
    # Create symlink for current version
    ln -sf "$version_dir" "$JCVM_ALIAS_DIR/current"
    
    jcvm_echo "$GREEN" "✓ Now using JDK $version"
    java -version 2>&1 | head -1
}

# Set local version for a project
jcvm_local() {
    local version="$1"
    
    if [[ -z "$version" ]]; then
        if [[ -f ".java-version" ]]; then
            cat .java-version
            return 0
        else
            jcvm_echo "$RED" "Error: Please specify a version"
            jcvm_echo "$YELLOW" "Usage: jcvm local <version>"
            return 1
        fi
    fi
    
    local version_dir="$JCVM_VERSIONS_DIR/$version"
    
    if [[ ! -d "$version_dir" ]]; then
        jcvm_echo "$RED" "Error: JDK version $version is not installed"
        jcvm_echo "$YELLOW" "Use 'jcvm install $version' to install it"
        return 1
    fi
    
    echo "$version" > .java-version
    jcvm_echo "$GREEN" "✓ Set local JDK version to $version"
    jcvm_echo "$YELLOW" "Created .java-version file in current directory"
    
    # Automatically switch to this version
    jcvm_use "$version"
}

# Set alias (like default)
jcvm_alias() {
    local alias_name="$1"
    local version="$2"
    
    if [[ -z "$alias_name" ]] || [[ -z "$version" ]]; then
        jcvm_echo "$RED" "Error: Please specify alias name and version"
        jcvm_echo "$YELLOW" "Usage: jcvm alias <name> <version>"
        jcvm_echo "$YELLOW" "Example: jcvm alias default 21"
        return 1
    fi
    
    local version_dir="$JCVM_VERSIONS_DIR/$version"
    
    if [[ ! -d "$version_dir" ]]; then
        jcvm_echo "$RED" "Error: JDK version $version is not installed"
        return 1
    fi
    
    ln -sf "$version_dir" "$JCVM_ALIAS_DIR/$alias_name"
    jcvm_echo "$GREEN" "✓ Set alias '$alias_name' to JDK $version"
}

# Uninstall a version
jcvm_uninstall() {
    local version="$1"
    
    if [[ -z "$version" ]]; then
        jcvm_echo "$RED" "Error: Please specify a version to uninstall"
        jcvm_echo "$YELLOW" "Usage: jcvm uninstall <version>"
        return 1
    fi
    
    local version_dir="$JCVM_VERSIONS_DIR/$version"
    
    if [[ ! -d "$version_dir" ]]; then
        jcvm_echo "$RED" "Error: JDK version $version is not installed"
        return 1
    fi
    
    # Check if it's the current version
    local current_version
    current_version=$(jcvm_current_version)
    
    if [[ "$version" == "$current_version" ]]; then
        jcvm_echo "$YELLOW" "Warning: This is the currently active version"
    fi
    
    read -p "Are you sure you want to uninstall JDK $version? (y/N) " -n 1 -r
    echo
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$version_dir"
        
        # Remove any aliases pointing to this version
        for alias_file in "$JCVM_ALIAS_DIR"/*; do
            if [[ -L "$alias_file" ]] && [[ "$(readlink "$alias_file")" == "$version_dir" ]]; then
                rm -f "$alias_file"
            fi
        done
        
        jcvm_echo "$GREEN" "✓ Uninstalled JDK $version"
    else
        jcvm_echo "$YELLOW" "Uninstall cancelled"
    fi
}

# Auto-detect and switch version based on .java-version file
jcvm_auto_switch() {
    if [[ -f ".java-version" ]]; then
        local version
        version=$(cat .java-version | tr -d '[:space:]')
        
        if [[ -n "$version" ]]; then
            local current_version
            current_version=$(jcvm_current_version)
            
            if [[ "$version" != "$current_version" ]]; then
                jcvm_use "$version" 2>/dev/null
            fi
        fi
    fi
}

# Show help
jcvm_help() {
    cat << EOF
$(jcvm_echo "$GREEN" "JCVM - Java Configuration & Version Manager")

$(jcvm_echo "$BLUE" "Usage:") jcvm <command> [options]

$(jcvm_echo "$BLUE" "Commands:")
  list-remote              List available JDK versions to install
  install <version>        Install a JDK version
  list                     List installed JDK versions
  use <version>           Use a specific JDK version
  current                  Show currently active version
  local <version>         Set JDK version for current directory
  alias <name> <version>  Create an alias for a version
  uninstall <version>     Uninstall a JDK version
  help                     Show this help message

$(jcvm_echo "$BLUE" "Examples:")
  jcvm list-remote         # Show available versions
  jcvm install 21          # Install JDK 21
  jcvm use 17              # Switch to JDK 17
  jcvm local 21            # Set JDK 21 for current project
  jcvm alias default 21    # Set JDK 21 as default

$(jcvm_echo "$BLUE" "Project Configuration:")
  Create a .java-version file in your project root:
    echo "21" > .java-version
  
  JCVM will automatically switch to this version when you cd into the directory.

$(jcvm_echo "$BLUE" "Supported Distributions:")
  - Eclipse Temurin (Adoptium) - Primary source
  - More distributions coming soon!

For more information, visit: https://github.com/yourusername/jcvm
EOF
}

# Main command dispatcher
jcvm() {
    jcvm_init
    
    local command="${1:-help}"
    shift || true
    
    case "$command" in
        install)
            jcvm_install "$@"
            ;;
        uninstall)
            jcvm_uninstall "$@"
            ;;
        use)
            jcvm_use "$@"
            ;;
        list)
            jcvm_list "$@"
            ;;
        list-remote|ls-remote)
            jcvm_list_remote "$@"
            ;;
        current)
            jcvm_current
            ;;
        local)
            jcvm_local "$@"
            ;;
        alias)
            jcvm_alias "$@"
            ;;
        help|--help|-h)
            jcvm_help
            ;;
        *)
            jcvm_echo "$RED" "Error: Unknown command '$command'"
            jcvm_help
            return 1
            ;;
    esac
}

# Auto-switch on directory change (if using zsh/bash with cd hook)
if [[ -n "${ZSH_VERSION:-}" ]]; then
    autoload -U add-zsh-hook
    jcvm_cd_hook() {
        jcvm_auto_switch
    }
    add-zsh-hook chpwd jcvm_cd_hook
    # Run on shell startup
    jcvm_auto_switch
elif [[ -n "${BASH_VERSION:-}" ]]; then
    jcvm_cd() {
        builtin cd "$@" && jcvm_auto_switch
    }
    alias cd='jcvm_cd'
    # Run on shell startup
    jcvm_auto_switch
fi

# Load default version if set
if [[ -L "$JCVM_ALIAS_DIR/default" ]] && [[ -z "${JAVA_HOME:-}" || ! "$JAVA_HOME" == "$JCVM_VERSIONS_DIR"* ]]; then
    local default_version
    default_version=$(basename "$(readlink "$JCVM_ALIAS_DIR/default")")
    jcvm_use "$default_version" >/dev/null 2>&1
fi

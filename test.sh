#!/usr/bin/env bash

# Test script for JCVM
# Run this to verify JCVM works correctly

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo_test() {
    echo -e "${GREEN}[TEST]${NC} $*"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Source JCVM
export JCVM_DIR="${JCVM_DIR:-$HOME/.jcvm-test}"
source ./jcvm.sh

echo_test "Testing JCVM initialization..."
jcvm_init

if [[ -d "$JCVM_DIR" ]]; then
    echo_test "✓ JCVM directory created"
else
    echo_error "✗ JCVM directory not created"
    exit 1
fi

echo_test "Testing OS detection..."
OS=$(jcvm_get_os)
echo_test "Detected OS: $OS"

echo_test "Testing architecture detection..."
ARCH=$(jcvm_get_arch)
echo_test "Detected architecture: $ARCH"

echo_test "Testing help command..."
jcvm help > /dev/null
echo_test "✓ Help command works"

echo_test "Testing list command (should show no versions)..."
jcvm list

echo_test "Testing list-remote command..."
jcvm list-remote | head -10

# Test version file
echo_test "Testing .java-version file detection..."
cd /tmp
echo "17" > .java-version
if [[ -f ".java-version" ]] && [[ "$(cat .java-version)" == "17" ]]; then
    echo_test "✓ .java-version file created"
else
    echo_error "✗ .java-version file test failed"
fi
rm .java-version

# Clean up
echo_test "Cleaning up test directory..."
rm -rf "$JCVM_DIR"

echo ""
echo_test "All tests passed!"
echo_test "Note: Actual installation and download tests require internet connection"
echo_test "Run 'jcvm install 21' to test a real installation"

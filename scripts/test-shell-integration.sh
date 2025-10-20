#!/bin/bash

# Test script for JCVM shell integration
# This script verifies that the shell wrapper function works correctly

set -e

echo "=== JCVM Shell Integration Test ==="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test 1: Check if jcvm is installed
echo "Test 1: Checking if jcvm is available..."
if command -v jcvm &> /dev/null; then
    echo -e "${GREEN}✓${NC} jcvm is installed"
    jcvm --version
else
    echo -e "${RED}✗${NC} jcvm not found"
    exit 1
fi
echo ""

# Test 2: Check if shell integration is installed
echo "Test 2: Checking shell integration..."
SHELL_CONFIG=""
if [ -n "$ZSH_VERSION" ]; then
    SHELL_CONFIG="$HOME/.zshrc"
elif [ -n "$BASH_VERSION" ]; then
    SHELL_CONFIG="$HOME/.bashrc"
fi

if [ -n "$SHELL_CONFIG" ] && [ -f "$SHELL_CONFIG" ]; then
    if grep -q "JCVM_DIR" "$SHELL_CONFIG"; then
        echo -e "${GREEN}✓${NC} Shell integration found in $SHELL_CONFIG"
    else
        echo -e "${YELLOW}!${NC} Shell integration not found in $SHELL_CONFIG"
        echo "  Run: jcvm shell-init"
    fi
else
    echo -e "${YELLOW}!${NC} Could not detect shell config file"
fi
echo ""

# Test 3: Check if jcvm is a function (indicates wrapper is active)
echo "Test 3: Checking if jcvm wrapper function is active..."
JCVM_TYPE=$(type jcvm 2>/dev/null | head -1)
if echo "$JCVM_TYPE" | grep -q "function"; then
    echo -e "${GREEN}✓${NC} jcvm is a shell function (wrapper is active)"
    echo "  This means 'jcvm use' will update your current shell!"
elif echo "$JCVM_TYPE" | grep -q "alias"; then
    echo -e "${GREEN}✓${NC} jcvm is a shell alias (wrapper is active)"
    echo "  This means 'jcvm use' will update your current shell!"
else
    echo -e "${YELLOW}!${NC} jcvm is not a function (wrapper is NOT active)"
    echo "  You may need to reload your shell: source ~/.zshrc"
    echo "  Or install shell integration: jcvm shell-init"
fi
echo ""

# Test 4: List installed versions
echo "Test 4: Listing installed versions..."
if jcvm list &> /dev/null; then
    echo -e "${GREEN}✓${NC} Can list installed versions"
    jcvm list
else
    echo -e "${RED}✗${NC} Failed to list versions"
fi
echo ""

# Test 5: Check current version
echo "Test 5: Checking current version..."
if jcvm current &> /dev/null; then
    echo -e "${GREEN}✓${NC} Can check current version"
    jcvm current
else
    echo -e "${YELLOW}!${NC} No current version set"
fi
echo ""

# Test 6: Verify JAVA_HOME and PATH
echo "Test 6: Checking JAVA_HOME and PATH..."
if [ -n "$JAVA_HOME" ]; then
    echo -e "${GREEN}✓${NC} JAVA_HOME is set: $JAVA_HOME"
else
    echo -e "${YELLOW}!${NC} JAVA_HOME is not set"
fi

if echo "$PATH" | grep -q "java"; then
    echo -e "${GREEN}✓${NC} PATH contains Java directory"
else
    echo -e "${YELLOW}!${NC} PATH does not contain Java directory"
fi
echo ""

# Summary
echo "=== Test Summary ==="
echo ""
echo "Shell integration status:"
if [ -n "$JCVM_TYPE" ] && (echo "$JCVM_TYPE" | grep -q "function" || echo "$JCVM_TYPE" | grep -q "alias"); then
    echo -e "${GREEN}✓ ACTIVE${NC} - 'jcvm use' will update your current shell seamlessly"
else
    echo -e "${YELLOW}! INACTIVE${NC} - You need to:"
    echo "  1. Install integration: jcvm shell-init"
    echo "  2. Reload shell: source ~/.zshrc (or ~/.bashrc)"
fi
echo ""

echo "To test version switching:"
echo "  jcvm use 21"
echo "  java --version  # Should show Java 21 immediately"
echo ""

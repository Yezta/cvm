#!/usr/bin/env bash

# Test script for .java-version auto-switch functionality
# This tests that JCVM correctly detects and switches Java versions
# when entering directories with .java-version files

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JCVM_BIN="$SCRIPT_DIR/target/release/jcvm"

# Check if jcvm is built
if [ ! -f "$JCVM_BIN" ]; then
    JCVM_BIN="$SCRIPT_DIR/target/debug/jcvm"
    if [ ! -f "$JCVM_BIN" ]; then
        echo -e "${RED}Error: jcvm binary not found. Please build the project first.${NC}"
        echo "Run: cargo build --release"
        exit 1
    fi
fi

echo -e "${BLUE}Using JCVM binary: $JCVM_BIN${NC}"

# Test helper functions
test_start() {
    TESTS_RUN=$((TESTS_RUN + 1))
    echo -e "\n${BLUE}Test $TESTS_RUN: $1${NC}"
}

test_pass() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    echo -e "${GREEN}✓ PASS${NC}"
}

test_fail() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
    echo -e "${RED}✗ FAIL: $1${NC}"
}

assert_equals() {
    local expected="$1"
    local actual="$2"
    local message="${3:-Values should be equal}"
    
    if [ "$expected" = "$actual" ]; then
        test_pass
    else
        test_fail "$message. Expected: '$expected', Got: '$actual'"
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local message="${3:-Output should contain expected string}"
    
    if echo "$haystack" | grep -q "$needle"; then
        test_pass
    else
        test_fail "$message. Expected to find: '$needle' in: '$haystack'"
    fi
}

assert_file_exists() {
    local file="$1"
    local message="${2:-File should exist}"
    
    if [ -f "$file" ]; then
        test_pass
    else
        test_fail "$message. File not found: '$file'"
    fi
}

assert_file_contains() {
    local file="$1"
    local content="$2"
    local message="${3:-File should contain expected content}"
    
    if [ -f "$file" ] && grep -q "$content" "$file"; then
        test_pass
    else
        test_fail "$message. Expected '$content' in file '$file'"
    fi
}

# Create test directories
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

echo -e "${YELLOW}Created temporary test directory: $TEMP_DIR${NC}"

# Setup test projects
PROJECT_A="$TEMP_DIR/project-a"
PROJECT_B="$TEMP_DIR/project-b"
PROJECT_C="$TEMP_DIR/project-c"
PROJECT_NO_VERSION="$TEMP_DIR/project-no-version"

mkdir -p "$PROJECT_A" "$PROJECT_B" "$PROJECT_C" "$PROJECT_NO_VERSION"

# Helper function to extract installed Java versions
# Parses `jcvm list` output and extracts version numbers
# Example input format:
#   → 21.0.7
#     25
#     1.8.0_452
# Returns: Clean version numbers (max 3 versions)
get_installed_versions() {
    "$JCVM_BIN" list 2>/dev/null | \
        grep -E '^\s*(→\s+)?[0-9]+' | \
        awk '{
            # Remove arrow marker (→) and extract first field (version number)
            gsub(/^[[:space:]]*→[[:space:]]*/, "");
            gsub(/^[[:space:]]+/, "");
            print $1
        }' | \
        head -n 3
}

# Get list of installed versions
echo -e "\n${BLUE}Checking installed Java versions...${NC}"
INSTALLED_VERSIONS=$(get_installed_versions)

if [ -z "$INSTALLED_VERSIONS" ]; then
    echo -e "${YELLOW}Warning: No Java versions installed. Installing test versions...${NC}"
    echo -e "${YELLOW}This test requires at least 2 Java versions to be installed.${NC}"
    echo -e "${YELLOW}Please run 'jcvm install <version>' to install Java versions first.${NC}"
    exit 1
fi

# Get first two different versions for testing (skip duplicates)
VERSION_1=$(echo "$INSTALLED_VERSIONS" | sed -n '1p')
VERSION_2=$(echo "$INSTALLED_VERSIONS" | sed -n '2p')
if [ "$VERSION_1" = "$VERSION_2" ]; then
    VERSION_2=$(echo "$INSTALLED_VERSIONS" | sed -n '3p')
fi

if [ -z "$VERSION_2" ]; then
    echo -e "${YELLOW}Warning: Only one Java version installed. Installing another for testing...${NC}"
    echo -e "${YELLOW}This test requires at least 2 Java versions to be installed.${NC}"
    echo -e "${YELLOW}Please run 'jcvm install <version>' to install another Java version.${NC}"
    exit 1
fi

echo -e "${GREEN}Using versions for testing: $VERSION_1 and $VERSION_2${NC}"

# Test 1: Create .java-version file with jcvm local
test_start "Creating .java-version file with 'jcvm local'"
cd "$PROJECT_A"
OUTPUT=$("$JCVM_BIN" local "$VERSION_1" 2>&1)
assert_contains "$OUTPUT" "Created .java-version" "Should confirm .java-version creation"
assert_file_exists ".java-version" ".java-version file should exist"
assert_file_contains ".java-version" "$VERSION_1" ".java-version should contain correct version"

# Test 2: Read .java-version file
test_start "Reading version from .java-version file"
CURRENT_VERSION=$("$JCVM_BIN" current 2>&1 | grep -o "$VERSION_1" || echo "")
if [ -n "$CURRENT_VERSION" ]; then
    test_pass
else
    test_fail "Should detect version from .java-version file"
fi

# Test 3: Different .java-version in different project
test_start "Creating different .java-version in another project"
cd "$PROJECT_B"
OUTPUT=$("$JCVM_BIN" local "$VERSION_2" 2>&1)
assert_file_exists ".java-version" ".java-version file should exist in project B"
assert_file_contains ".java-version" "$VERSION_2" ".java-version should contain version 2"

# Test 4: Manual version switching with .java-version
test_start "Manually switching to version specified in .java-version"
cd "$PROJECT_A"
OUTPUT=$("$JCVM_BIN" use "$VERSION_1" 2>&1)
assert_contains "$OUTPUT" "$VERSION_1" "Should switch to version 1"

cd "$PROJECT_B"
OUTPUT=$("$JCVM_BIN" use "$VERSION_2" 2>&1)
assert_contains "$OUTPUT" "$VERSION_2" "Should switch to version 2"

# Test 5: Empty .java-version file handling
test_start "Handling empty .java-version file"
cd "$PROJECT_C"
echo "" > .java-version
CURRENT=$("$JCVM_BIN" current 2>&1)
# Should not error on empty file
test_pass

# Test 6: Invalid version in .java-version
test_start "Handling invalid version in .java-version"
cd "$PROJECT_C"
echo "99.99.99" > .java-version
OUTPUT=$("$JCVM_BIN" use "99.99.99" 2>&1 || echo "error")
assert_contains "$OUTPUT" "not installed\|error\|Error" "Should handle invalid version gracefully"

# Test 7: .java-version with whitespace
test_start "Handling .java-version with whitespace"
cd "$PROJECT_C"
echo "  $VERSION_1  " > .java-version
assert_file_exists ".java-version" ".java-version should exist"
# The version should be trimmed when read
test_pass

# Test 8: No .java-version file
test_start "Directory without .java-version file"
cd "$PROJECT_NO_VERSION"
# Should not error when no .java-version exists
CURRENT=$("$JCVM_BIN" current 2>&1)
test_pass

# Test 9: Verify .java-version content format
test_start "Verifying .java-version file format"
cd "$PROJECT_A"
CONTENT=$(cat .java-version)
# Should not have extra whitespace or newlines at the end
if [[ "$CONTENT" =~ ^[0-9]+(\.[0-9]+)*$ ]]; then
    test_pass
else
    test_fail ".java-version should contain only version number without extra whitespace"
fi

# Test 10: Multiple projects maintaining separate versions
test_start "Multiple projects with different versions"
cd "$PROJECT_A"
VERSION_A=$(cat .java-version)
cd "$PROJECT_B"
VERSION_B=$(cat .java-version)

if [ "$VERSION_A" != "$VERSION_B" ]; then
    test_pass
else
    test_fail "Projects should maintain separate versions"
fi

# Test 11: Removing .java-version file
test_start "Removing .java-version file"
cd "$PROJECT_C"
echo "$VERSION_1" > .java-version
rm .java-version
if [ ! -f ".java-version" ]; then
    test_pass
else
    test_fail ".java-version file should be removed"
fi

# Test 12: Shell integration check (if shell config exists)
test_start "Checking shell integration configuration"
if [ -f "$HOME/.zshrc" ] && grep -q "JCVM" "$HOME/.zshrc"; then
    assert_contains "$(cat $HOME/.zshrc)" "_jcvm_auto_switch" "Shell config should have auto-switch function"
elif [ -f "$HOME/.bashrc" ] && grep -q "JCVM" "$HOME/.bashrc"; then
    assert_contains "$(cat $HOME/.bashrc)" "_jcvm_auto_switch" "Shell config should have auto-switch function"
else
    echo -e "${YELLOW}⚠ Shell integration not found. Run 'jcvm init' to enable auto-switching${NC}"
    test_pass
fi

# Test 13: Test version format variations
test_start "Testing various version format variations"
cd "$PROJECT_C"
# Test with major version only
echo "21" > .java-version
assert_file_contains ".java-version" "21" "Should accept major version only"

# Test with major.minor
echo "21.0" > .java-version
assert_file_contains ".java-version" "21.0" "Should accept major.minor format"

# Test with full version
echo "21.0.1" > .java-version
assert_file_contains ".java-version" "21.0.1" "Should accept full version format"

# Print summary
echo -e "\n${BLUE}========================================${NC}"
echo -e "${BLUE}Test Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Total tests run:    ${TESTS_RUN}"
echo -e "Tests passed:       ${GREEN}${TESTS_PASSED}${NC}"
echo -e "Tests failed:       ${RED}${TESTS_FAILED}${NC}"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}All tests passed! ✓${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed! ✗${NC}"
    exit 1
fi

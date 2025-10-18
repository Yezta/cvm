#!/usr/bin/env bash
# Version Bump Script for JCVM
# Bumps version numbers in VERSION and Cargo.toml files
# Usage: ./scripts/bump-version.sh [major|minor|patch]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default bump type
BUMP_TYPE="${1:-patch}"

# Validate bump type
if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo -e "${RED}Error: Invalid bump type '$BUMP_TYPE'${NC}"
    echo "Usage: $0 [major|minor|patch]"
    echo ""
    echo "Examples:"
    echo "  $0 patch   # 1.2.3 -> 1.2.4"
    echo "  $0 minor   # 1.2.3 -> 1.3.0"
    echo "  $0 major   # 1.2.3 -> 2.0.0"
    exit 1
fi

# Check if we're in the project root
if [ ! -f "VERSION" ] || [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Must be run from project root directory${NC}"
    exit 1
fi

# Get current version
CURRENT_VERSION=$(cat VERSION | tr -d '[:space:]')
echo -e "${BLUE}Current version: ${CURRENT_VERSION}${NC}"

# Parse semantic version
IFS='.' read -r major minor patch <<< "$CURRENT_VERSION"

# Bump version based on type
case "$BUMP_TYPE" in
    major)
        NEW_MAJOR=$((major + 1))
        NEW_VERSION="${NEW_MAJOR}.0.0"
        ;;
    minor)
        NEW_MINOR=$((minor + 1))
        NEW_VERSION="${major}.${NEW_MINOR}.0"
        ;;
    patch)
        NEW_PATCH=$((patch + 1))
        NEW_VERSION="${major}.${minor}.${NEW_PATCH}"
        ;;
esac

echo -e "${YELLOW}Bumping ${BUMP_TYPE} version: ${CURRENT_VERSION} -> ${NEW_VERSION}${NC}"

# Confirm with user
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Aborted${NC}"
    exit 1
fi

# Update VERSION file
echo "$NEW_VERSION" > VERSION
echo -e "${GREEN}✓ Updated VERSION file${NC}"

# Update Cargo.toml
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
else
    # Linux
    sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
fi
echo -e "${GREEN}✓ Updated Cargo.toml${NC}"

# Update Cargo.lock
cargo update -p jcvm --precise "$NEW_VERSION" 2>/dev/null || cargo update -p jcvm 2>/dev/null || true
echo -e "${GREEN}✓ Updated Cargo.lock${NC}"

# Get commits since last tag for changelog
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [ -z "$LAST_TAG" ]; then
    COMMITS=$(git log --pretty=format:"- %s (%h)" --no-merges -n 10)
else
    COMMITS=$(git log ${LAST_TAG}..HEAD --pretty=format:"- %s (%h)" --no-merges)
fi

# Prepare changelog entry
DATE=$(date +%Y-%m-%d)
SECTION=""
case "$BUMP_TYPE" in
    major)
        SECTION="### Breaking Changes"
        ;;
    minor)
        SECTION="### Added"
        ;;
    patch)
        SECTION="### Fixed"
        ;;
esac

# Create temporary changelog entry
TEMP_ENTRY=$(cat << EOF

## [${NEW_VERSION}] - ${DATE}

${SECTION}

${COMMITS}

EOF
)

# Update CHANGELOG.md
if [ -f "CHANGELOG.md" ]; then
    if grep -q "## \[Unreleased\]" CHANGELOG.md; then
        # Find line number of [Unreleased] section
        LINE_NUM=$(grep -n "## \[Unreleased\]" CHANGELOG.md | head -1 | cut -d: -f1)
        # Find next ## heading after Unreleased
        NEXT_SECTION=$(tail -n +$((LINE_NUM + 1)) CHANGELOG.md | grep -n "^## \[" | head -1 | cut -d: -f1)
        
        if [ -n "$NEXT_SECTION" ]; then
            INSERT_LINE=$((LINE_NUM + NEXT_SECTION))
            # Insert new entry
            {
                head -n $((INSERT_LINE - 1)) CHANGELOG.md
                echo "$TEMP_ENTRY"
                tail -n +${INSERT_LINE} CHANGELOG.md
            } > CHANGELOG.md.tmp
            mv CHANGELOG.md.tmp CHANGELOG.md
        else
            # Append at end
            echo "$TEMP_ENTRY" >> CHANGELOG.md
        fi
    else
        # Prepend after header (first 7 lines)
        {
            head -n 7 CHANGELOG.md
            echo "$TEMP_ENTRY"
            tail -n +8 CHANGELOG.md
        } > CHANGELOG.md.tmp
        mv CHANGELOG.md.tmp CHANGELOG.md
    fi
    echo -e "${GREEN}✓ Updated CHANGELOG.md${NC}"
fi

# Show git diff
echo ""
echo -e "${BLUE}Changes:${NC}"
git diff VERSION Cargo.toml CHANGELOG.md

# Ask to commit
echo ""
read -p "Commit changes? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    git add VERSION Cargo.toml Cargo.lock CHANGELOG.md
    git commit -m "chore: bump $BUMP_TYPE version to $NEW_VERSION"
    echo -e "${GREEN}✓ Changes committed${NC}"
    
    # Ask to create tag
    read -p "Create and push tag v${NEW_VERSION}? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git tag -a "v${NEW_VERSION}" -m "Release version ${NEW_VERSION}"
        git push origin main --tags
        echo -e "${GREEN}✓ Tag v${NEW_VERSION} created and pushed${NC}"
    fi
else
    echo -e "${YELLOW}Changes not committed. You can commit manually or reset changes.${NC}"
fi

echo ""
echo -e "${GREEN}Version bump complete!${NC}"
echo -e "${BLUE}New version: ${NEW_VERSION}${NC}"

#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to update version in a file
update_version() {
    local file=$1
    local pattern=$2
    local replacement=$3
    
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' -E "$pattern" "$file"
    else
        # Linux
        sed -i -E "$pattern" "$file"
    fi
}

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep -E '^version = ".*"$' src-tauri/Cargo.toml | head -1 | cut -d'"' -f2)

echo -e "${YELLOW}Current version: $CURRENT_VERSION${NC}"
echo -e "${YELLOW}Enter new version (e.g., 0.2.0):${NC}"
read NEW_VERSION

# Validate version format
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Invalid version format! Use semantic versioning (e.g., 0.2.0)${NC}"
    exit 1
fi

# Check if we're on a clean working directory
if ! git diff-index --quiet HEAD --; then
    echo -e "${RED}Working directory has uncommitted changes!${NC}"
    echo "Please commit or stash changes before releasing."
    exit 1
fi

# Check if tag already exists
if git rev-parse "v$NEW_VERSION" >/dev/null 2>&1; then
    echo -e "${RED}Tag v$NEW_VERSION already exists!${NC}"
    exit 1
fi

echo -e "${GREEN}Updating version to $NEW_VERSION...${NC}"

# Update version in all necessary files
echo "Updating src-tauri/Cargo.toml..."
update_version "src-tauri/Cargo.toml" "s/^version = \".*\"/version = \"$NEW_VERSION\"/" 

echo "Updating src-tauri/tauri.conf.json..."
update_version "src-tauri/tauri.conf.json" "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/"

# Update version display in app.rs
echo "Updating version in app.rs..."
update_version "src/app.rs" 's/use_signal\(\|\| ".*"\.to_string\(\)\)/use_signal(|| "'$NEW_VERSION'".to_string())/'

# Stage all version changes
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json src/app.rs

# Create commit
echo -e "${GREEN}Creating commit...${NC}"
git commit -m "Release v$NEW_VERSION

- Updated version in Cargo.toml, tauri.conf.json, and app.rs
- Preparing for automated release via GitHub Actions"

# Create tag
echo -e "${GREEN}Creating tag v$NEW_VERSION...${NC}"
git tag -a "v$NEW_VERSION" -m "Release v$NEW_VERSION"

# Show what will be pushed
echo -e "${YELLOW}Ready to push the following:${NC}"
git log --oneline -1
echo "Tag: v$NEW_VERSION"

echo -e "${YELLOW}Push to remote? (y/n)${NC}"
read -r CONFIRM

if [[ "$CONFIRM" == "y" || "$CONFIRM" == "Y" ]]; then
    echo -e "${GREEN}Pushing to origin...${NC}"
    git push origin main
    git push origin "v$NEW_VERSION"
    
    echo -e "${GREEN}✅ Release v$NEW_VERSION pushed successfully!${NC}"
    echo -e "${GREEN}GitHub Actions will now build and create the release.${NC}"
    echo -e "${GREEN}Check the Actions tab on GitHub for build progress.${NC}"
else
    echo -e "${YELLOW}Push cancelled. To push manually:${NC}"
    echo "  git push origin main"
    echo "  git push origin v$NEW_VERSION"
fi
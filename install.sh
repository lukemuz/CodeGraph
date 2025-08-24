#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Repository info
REPO="lucasmuzynoski/CodeGraph"
BINARY_NAME="codegraph"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture names
case $ARCH in
  x86_64) TARGET_ARCH="x86_64" ;;
  arm64|aarch64) TARGET_ARCH="aarch64" ;;
  *) 
    echo -e "${RED}Error: Unsupported architecture $ARCH${NC}"
    exit 1
    ;;
esac

# Map OS names and set target
case $OS in
  linux)
    TARGET="${TARGET_ARCH}-unknown-linux-musl"
    ARCHIVE_EXT="tar.gz"
    ;;
  darwin)
    if [ "$TARGET_ARCH" = "aarch64" ]; then
      TARGET="aarch64-apple-darwin"
    else
      TARGET="x86_64-apple-darwin" 
    fi
    ARCHIVE_EXT="tar.gz"
    ;;
  mingw*|cygwin*|msys*)
    TARGET="x86_64-pc-windows-msvc"
    ARCHIVE_EXT="zip"
    BINARY_NAME="codegraph.exe"
    ;;
  *)
    echo -e "${RED}Error: Unsupported OS $OS${NC}"
    exit 1
    ;;
esac

echo -e "${YELLOW}Detected platform: $TARGET${NC}"

# Get the latest release URL
echo -e "${YELLOW}Fetching latest release...${NC}"
RELEASE_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | \
  grep "browser_download_url.*$TARGET\.$ARCHIVE_EXT" | \
  cut -d '"' -f 4)

if [ -z "$RELEASE_URL" ]; then
  echo -e "${RED}Error: No release found for platform $TARGET${NC}"
  echo -e "${YELLOW}Available releases:${NC}"
  curl -s "https://api.github.com/repos/$REPO/releases/latest" | \
    grep "browser_download_url" | \
    cut -d '"' -f 4 | \
    sed 's/.*\//  - /'
  exit 1
fi

# Create temporary directory
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

echo -e "${YELLOW}Downloading $RELEASE_URL...${NC}"
curl -L -o "archive.$ARCHIVE_EXT" "$RELEASE_URL"

# Extract archive
echo -e "${YELLOW}Extracting archive...${NC}"
if [ "$ARCHIVE_EXT" = "tar.gz" ]; then
  tar -xzf "archive.$ARCHIVE_EXT"
elif [ "$ARCHIVE_EXT" = "zip" ]; then
  unzip "archive.$ARCHIVE_EXT"
fi

# Find the binary
BINARY_PATH=$(find . -name "$BINARY_NAME" -type f | head -1)
if [ -z "$BINARY_PATH" ]; then
  echo -e "${RED}Error: Binary not found in archive${NC}"
  exit 1
fi

# Install binary
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

echo -e "${YELLOW}Installing to $INSTALL_DIR/$BINARY_NAME...${NC}"
cp "$BINARY_PATH" "$INSTALL_DIR/$BINARY_NAME"
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Cleanup
cd /
rm -rf "$TEMP_DIR"

echo -e "${GREEN}✓ CodeGraph installed successfully!${NC}"
echo
echo -e "${YELLOW}Next steps:${NC}"
echo "1. Make sure $INSTALL_DIR is in your PATH"
echo "2. Add to Claude Code:"
echo "   claude mcp add codegraph -- $INSTALL_DIR/$BINARY_NAME mcp"
echo
echo "3. Or add to Cursor (~/.cursor/mcp.json):"
echo '   {'
echo '     "mcpServers": {'
echo '       "codegraph": {'
echo '         "command": "'$INSTALL_DIR/$BINARY_NAME'",'
echo '         "args": ["mcp"]'
echo '       }'
echo '     }'
echo '   }'
echo
echo -e "${GREEN}Happy coding! 🚀${NC}"
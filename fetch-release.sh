#!/bin/bash

DOWNLOAD_DIR="./release/latest"
mkdir -p "$DOWNLOAD_DIR"

# Fetch latest tag name
LATEST_TAG=$(gh release view --repo "$REPO" --json tagName --jq .tagName)

echo "Latest release tag: $LATEST_TAG"

# Download ZIP files from the latest release
gh release download "$LATEST_TAG" --repo "$REPO" --pattern "*.zip" --dir "$DOWNLOAD_DIR"

# Extract all downloaded ZIP files
for zipfile in "$DOWNLOAD_DIR"/*.zip; do
  echo "Extracting $zipfile..."
  unzip -o "$zipfile" -d "$DOWNLOAD_DIR"
done

echo "Done."

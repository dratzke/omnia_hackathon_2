#!/bin/bash

DOWNLOAD_DIR="./release/latest"
mkdir -p "$DOWNLOAD_DIR"

LATEST_TAG=$(gh release view --repo "$REPO" --json tagName --jq .tagName)

echo "Latest release tag: $LATEST_TAG"

LATEST_ZIP="$DOWNLOAD_DIR/$LATEST_TAG.zip"
if [ -f "$LATEST_ZIP" ]; then
  echo "Latest release file already exists: $LATEST_ZIP. Skipping fetch"
else
  gh release download "$LATEST_TAG" --repo "$REPO" --pattern "*.zip" --dir "$DOWNLOAD_DIR"
  # Rename the downloaded file to include the tag name
  for ZIP_FILE in "$DOWNLOAD_DIR"/*.zip; do
    mv "$ZIP_FILE" "$LATEST_ZIP"
    echo "Downloaded and renamed to $LATEST_ZIP"
  done
fi

echo "Extracting $LATEST_ZIP..."
unzip -o "$LATEST_ZIP" -d "$DOWNLOAD_DIR"

# Delete all other ZIP files in the directory
for ZIP_FILE in "$DOWNLOAD_DIR"/*.zip; do
  if [ "$ZIP_FILE" != "$LATEST_ZIP" ]; then
    echo "Deleting old ZIP file: $ZIP_FILE"
    rm "$ZIP_FILE"
  fi
done

echo "Done."
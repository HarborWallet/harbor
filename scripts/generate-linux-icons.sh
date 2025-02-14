#!/bin/bash

set -e # Exit on error

ICON_SOURCE="harbor-ui/assets/harbor_icon.png"
ICON_SIZES=(16 32 48 64 128 256)
OUTPUT_DIR="harbor-ui/assets/linux/icons"

# Create output directories for each size
for size in "${ICON_SIZES[@]}"; do
    mkdir -p "$OUTPUT_DIR/${size}x${size}"
done

# Generate icons for each size
for size in "${ICON_SIZES[@]}"; do
    magick convert "$ICON_SOURCE" -resize "${size}x${size}" "$OUTPUT_DIR/${size}x${size}/harbor.png"
done

echo "✨ Generated Linux icons in $OUTPUT_DIR" 
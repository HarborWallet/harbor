#!/bin/bash

set -e # Exit on error

APP_DIR="$1"
if [ -z "$APP_DIR" ]; then
    echo "Usage: $0 <path/to/Harbor.app>"
    exit 1
fi

BINARY_PATH="$APP_DIR/Contents/MacOS/harbor"
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

# Create Frameworks directory if it doesn't exist
FRAMEWORKS_DIR="$APP_DIR/Contents/Frameworks"
mkdir -p "$FRAMEWORKS_DIR"

# Fix @rpath for all libraries
echo "Setting up @rpath..."
install_name_tool -add_rpath "@executable_path/../Frameworks" "$BINARY_PATH"

# Check for dependencies that are not system libraries
echo "Checking for dynamic library dependencies..."
DEPS=$(otool -L "$BINARY_PATH" | grep -v "/System/" | grep -v "@rpath" | grep -v "@executable_path" | grep -v "/usr/lib/" | awk -F' ' '{print $1}')

if [ -n "$DEPS" ]; then
    echo "Found external dependencies:"
    echo "$DEPS"
    
    for DEP_PATH in $DEPS; do
        # Skip if the path is not a file or doesn't exist
        if [ ! -f "$DEP_PATH" ]; then
            echo "Warning: Dependency $DEP_PATH not found, skipping"
            continue
        fi
        
        DEP_NAME=$(basename "$DEP_PATH")
        echo "Processing $DEP_NAME from $DEP_PATH"
        
        # Copy to Frameworks directory
        cp "$DEP_PATH" "$FRAMEWORKS_DIR/"
        
        # Change the reference in the binary
        install_name_tool -change "$DEP_PATH" "@rpath/$DEP_NAME" "$BINARY_PATH"
        
        # Make sure the library itself has the correct ID
        install_name_tool -id "@rpath/$DEP_NAME" "$FRAMEWORKS_DIR/$DEP_NAME"
        
        echo "Fixed path for $DEP_NAME"
        
        # Also check for dependencies of this library
        SUB_DEPS=$(otool -L "$FRAMEWORKS_DIR/$DEP_NAME" | grep -v "/System/" | grep -v "@rpath" | grep -v "@executable_path" | grep -v "/usr/lib/" | awk -F' ' '{print $1}')
        
        for SUB_DEP_PATH in $SUB_DEPS; do
            # Skip if it's referring to itself, not a file, or doesn't exist
            if [[ "$SUB_DEP_PATH" == *"$DEP_NAME"* ]] || [ ! -f "$SUB_DEP_PATH" ]; then
                continue
            fi
            
            SUB_DEP_NAME=$(basename "$SUB_DEP_PATH")
            echo "Processing sub-dependency $SUB_DEP_NAME from $SUB_DEP_PATH"
            
            # Copy to Frameworks directory if not already there
            if [ ! -f "$FRAMEWORKS_DIR/$SUB_DEP_NAME" ]; then
                cp "$SUB_DEP_PATH" "$FRAMEWORKS_DIR/"
                install_name_tool -id "@rpath/$SUB_DEP_NAME" "$FRAMEWORKS_DIR/$SUB_DEP_NAME"
            fi
            
            # Fix the reference in the parent library
            install_name_tool -change "$SUB_DEP_PATH" "@rpath/$SUB_DEP_NAME" "$FRAMEWORKS_DIR/$DEP_NAME"
            
            echo "Fixed sub-dependency $SUB_DEP_NAME"
        done
    done
else
    echo "No external dependencies found"
fi

echo "Done fixing library paths" 
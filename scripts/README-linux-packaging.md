# Linux Packaging with Nix and Manual Binary Patching

This document explains how the Harbor project packages Linux binaries built with Nix into standard .deb packages.

## The Challenge

Building with Nix provides reproducible builds with consistent dependencies, but creates a distribution challenge: binaries built with Nix link to libraries in Nix-specific paths that don't exist on standard Linux distributions.

## Our Solution

We solve this by:

1. **Building**: We build Harbor with Nix, ensuring consistent dependencies.

2. **Library Bundling**: We copy key libraries (Wayland, GL, etc.) from the Nix environment into `/usr/lib/harbor/libs` in our package.

3. **Direct RPATH Patching**: We manually patch the binary's RPATH using `patchelf` to:
   - Look for libraries in `/usr/lib/harbor/libs` first (our bundled libraries)
   - Fall back to standard system locations (`/usr/lib`, `/usr/lib/x86_64-linux-gnu`, etc.)

4. **LD_LIBRARY_PATH Backup**: We also set `LD_LIBRARY_PATH` in the wrapper script for added reliability.

5. **Minimal Dependencies**: The .deb package has minimal hard dependencies, with non-critical libraries marked as "Recommended".

## Why This Approach Works

- **Direct Control**: We directly set the binary's search paths, ensuring it can find our bundled libraries.
- **Reliability**: Even if RPATH patching isn't perfect, the LD_LIBRARY_PATH backup ensures libraries are found.
- **System Integration**: The application behaves like a normal system package when possible.
- **Fallback Mechanism**: The binary can use system libraries if available, or bundled ones if not.

## Debugging

If there are issues with the packaged application:

1. **Enable debug mode**:
   ```
   HARBOR_DEBUG=1 harbor-ui
   ```
   This will write detailed environment and library information to `/tmp/harbor-env-debug.log`.

2. **Check the RPATH of the binary**:
   ```
   patchelf --print-rpath /usr/lib/harbor/harbor-ui-bin
   ```
   It should include `/usr/lib/harbor/libs` as the first path.

3. **Verify bundled libraries**:
   ```
   ls -la /usr/lib/harbor/libs
   ```
   Should contain key Wayland libraries like `libwayland-client.so`.

4. **Check library dependencies**:
   ```
   ldd /usr/lib/harbor/harbor-ui-bin | grep "not found"
   ```
   This will show any missing libraries.

5. **For Wayland-specific debugging**:
   ```
   WAYLAND_DEBUG=1 harbor-ui
   ```

## Implementation Details

- `package-linux.sh`: Creates the .deb package structure, copies files, and directly patches the binary's RPATH.
- The wrapper script (`/usr/bin/harbor-ui`) sets up environment variables and ensures `LD_LIBRARY_PATH` includes our bundled libraries.
- Both Nix and system library paths are used as fallbacks to maximize compatibility.

## Adding New Dependencies

If you need to add new libraries to bundle:

1. Add the library pattern to both the Nix and fallback sections in `package-linux.sh`.
2. Add any necessary system dependencies to the `Depends:` or `Recommends:` line in the control file.
3. Test with `ldd` to ensure all library dependencies are resolved. 
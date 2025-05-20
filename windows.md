# Windows Release Build Guide

This document explains the setup for building, signing, and packaging the Harbor application for Windows as part of the release process.

## Overview

The Windows build process generates two types of release artifacts:

1.  A `.zip` file containing the `harbor.exe` executable and any other necessary files.
2.  An `.msi` installer created using the WiX Toolset, providing a standard Windows installation experience (Start Menu shortcut, entry in Add/Remove Programs).

This process is orchestrated by the `.github/workflows/release.yml` GitHub Actions workflow, which utilizes several PowerShell scripts located in the `scripts/` directory (`build-windows.ps1`, `sign-windows.ps1`, `package-windows.ps1`) and asset files in `harbor-ui/assets/windows/`.

## Prerequisites

To enable the full Windows release process, including code signing, you will need:

1.  **Windows Code Signing Certificate:** A `.pfx` file containing your code signing certificate and private key, obtained from a Certificate Authority (CA).
2.  **Certificate Password:** The password associated with your `.pfx` file.

## GitHub Secrets

The workflow requires the following secrets to be configured in the GitHub repository (`Settings -> Secrets and variables -> Actions -> New repository secret`):

*   `WINDOWS_CERTIFICATE_PFX_BASE64`: The content of your `.pfx` code signing certificate file, encoded in Base64.
    *   **How to generate:**
        *   On Linux/macOS: `base64 -w 0 <your_certificate.pfx>`
        *   On Windows (PowerShell): `[Convert]::ToBase64String([IO.File]::ReadAllBytes("your_certificate.pfx"))`
*   `WINDOWS_CERTIFICATE_PWD`: The password for your `.pfx` certificate file.

**Note:** If these secrets are not provided, the workflow will still build and package the application, but the `.exe` and `.msi` files will *not* be signed. This might cause warnings from Windows SmartScreen for users.

## Required Asset Files

The following files must exist in the `harbor-ui/assets/windows/` directory:

1.  `harbor.ico`: The application icon in the Windows `.ico` format. This icon is embedded in the `.exe` (if configured in `build.rs`), used in the MSI installer, and for the application shortcut.
2.  `License.rtf`: The End User License Agreement (EULA) in Rich Text Format (`.rtf`). This is displayed during the MSI installation process.

## WiX Installer Configuration (`harbor.wxs`)

The `harbor-ui/assets/windows/harbor.wxs` file defines the structure and behavior of the MSI installer.

**IMPORTANT: GUID Replacements**

This file contains several placeholder GUIDs (Globally Unique Identifiers) that **must** be replaced with actual, unique GUIDs before the installer can be built correctly.

*   Look for all instances of `PUT-A-UNIQUE-GUID-HERE-...`.
*   Generate new, unique GUIDs for each placeholder.
    *   You can use PowerShell: `[guid]::NewGuid()`
    *   Or an online GUID generator.
*   **Crucially:** The GUID assigned to the `UpgradeCode` attribute in the `<Product>` tag **must remain the same** across all future versions of Harbor. This allows Windows to correctly handle upgrades. The other GUIDs (for Components) should be unique per component but can stay the same for that component across versions.

Example placeholder:
```xml
<Product Id="*" ... UpgradeCode="PUT-A-UNIQUE-GUID-HERE-1111-2222-333333333333">
```
```xml
<Component Id="MainExecutable" Guid="PUT-A-UNIQUE-GUID-HERE-4444-5555-666666666666">
```
Replace these with newly generated GUIDs, e.g.:
```xml
<Product Id="*" ... UpgradeCode="{YOUR-PERMANENT-UPGRADE-CODE-GUID}">
```
```xml
<Component Id="MainExecutable" Guid="{UNIQUE-COMPONENT-GUID-1}">
```

Failure to replace these GUIDs will result in build errors. 
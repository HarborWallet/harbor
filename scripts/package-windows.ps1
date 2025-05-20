param(
    [Parameter(Mandatory=$true)]
    [string]$Version, # e.g., 1.0.0

    [string]$TargetName = "harbor-ui",
    [string]$TargetTriple = "x86_64-pc-windows-msvc",
    [string]$BuildDir = "target",
    [string]$AssetsDir = "harbor-ui\assets\windows",
    [string]$Architecture = "x64" # Or "win64" depending on preference
)

$ReleaseDir = Join-Path -Path $BuildDir -ChildPath "release"
$ExeSourcePath = Join-Path -Path $BuildDir -ChildPath "$TargetTriple\release\$TargetName.exe"
$IconSourcePath = Join-Path -Path $PWD -ChildPath "$AssetsDir\harbor.ico" # WiX needs absolute path usually
$WixSourcePath = Join-Path -Path $PWD -ChildPath "$AssetsDir\harbor.wxs"  # WiX needs absolute path usually
$StagingDir = Join-Path -Path $ReleaseDir -ChildPath "windows_staging"
$FinalExeName = "harbor.exe" # Rename for consistency if desired

# --- Create Output Directory ---
if (-not (Test-Path $ReleaseDir)) {
    New-Item -ItemType Directory -Path $ReleaseDir | Out-Null
}
if (Test-Path $StagingDir) {
    Remove-Item -Recurse -Force $StagingDir
}
New-Item -ItemType Directory -Path $StagingDir | Out-Null

# --- Check prerequisites ---
if (-not (Test-Path $ExeSourcePath)) { Write-Error "Executable not found: $ExeSourcePath"; exit 1 }
if (-not (Test-Path $IconSourcePath)) { Write-Error "Icon not found: $IconSourcePath"; exit 1 }
if (-not (Test-Path $WixSourcePath)) { Write-Error "WiX source not found: $WixSourcePath"; exit 1 }

# --- Stage Files ---
Copy-Item -Path $ExeSourcePath -Destination (Join-Path $StagingDir $FinalExeName)
# Copy other assets if needed (e.g., config files, DLLs not bundled by cargo)
# Copy-Item -Path "path/to/asset.dll" -Destination $StagingDir

# --- Create ZIP Archive ---
$ZipFileName = "harbor-$Version-$Architecture-windows.zip"
$ZipFilePath = Join-Path -Path $ReleaseDir -ChildPath $ZipFileName
Compress-Archive -Path "$StagingDir\*" -DestinationPath $ZipFilePath -Force
Write-Host "Created ZIP archive: $ZipFilePath"
echo "zip_path=$ZipFilePath" >> $env:GITHUB_OUTPUT

# --- Create MSI Installer (using WiX) ---
Write-Host "Building MSI installer..."
$MsiFileName = "harbor-$Version-$Architecture.msi"
$MsiFilePath = Join-Path -Path $ReleaseDir -ChildPath $MsiFileName
$WixObjDir = Join-Path -Path $BuildDir -ChildPath "wixobj"

# Find WiX tools (candle.exe, light.exe) - assumes they are in PATH after choco install
$CandlePath = Get-Command candle.exe -ErrorAction SilentlyContinue
$LightPath = Get-Command light.exe -ErrorAction SilentlyContinue

if (-not $CandlePath -or -not $LightPath) {
    Write-Error "WiX tools (candle.exe, light.exe) not found in PATH. Ensure WiX Toolset is installed."
    exit 1
}

# Compile WiX source (candle.exe)
Write-Host "Running Candle..."
& $CandlePath `
    "$WixSourcePath" `
    "-out" "$WixObjDir\" `
    "-arch" $Architecture `
    "-dProductVersion=$Version" `
    "-dHarborExePath=$(Join-Path $StagingDir $FinalExeName)" `
    "-dHarborIconPath=$IconSourcePath"
if ($LASTEXITCODE -ne 0) { Write-Error "WiX Candle compilation failed!"; exit 1 }

# Link WiX objects (light.exe)
Write-Host "Running Light..."
$lightArgs = @(
    "`"$WixObjDir\*.wixobj`"", # Input object files
    "-out", "`"$MsiFilePath`"", # Output MSI file
    "-spdb", # Generate PDB for debugging if needed
    "-ext", "WixUIExtension" # Include if using standard UI like WixUI_Minimal
    # "-ext", "WixUtilExtension" # Example: If using utility extensions
    # "-cultures:en-us" # Specify culture if needed
)
 & $LightPath $lightArgs
 if ($LASTEXITCODE -ne 0) { Write-Error "WiX Light linking failed!"; exit 1 }

Write-Host "Created MSI installer: $MsiFilePath"
echo "msi_path=$MsiFilePath" >> $env:GITHUB_OUTPUT

# Clean up staging
Remove-Item -Recurse -Force $StagingDir | Out-Null
Remove-Item -Recurse -Force $WixObjDir | Out-Null

Write-Host "Windows packaging complete." 
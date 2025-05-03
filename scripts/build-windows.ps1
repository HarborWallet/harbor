param(
    [string]$TargetName = "harbor-ui",
    [string]$TargetTriple = "x86_64-pc-windows-msvc",
    [string]$Features = "vendored" # Add other features if needed, comma-separated
)

Write-Host "Building $TargetName for $TargetTriple..."

# Ensure output directory exists (Cargo might do this, but belt-and-suspenders)
$ReleaseDir = "target\$TargetTriple\release"
New-Item -ItemType Directory -Force -Path $ReleaseDir | Out-Null

# Build the Rust executable
$cargoArgs = @("build", "--release", "--target", $TargetTriple, "-p", $TargetName)
if ($Features) {
    $cargoArgs += "--features", $Features
}

cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "Cargo build failed!"
    exit 1
}

$ExePath = Join-Path -Path $ReleaseDir -ChildPath "$TargetName.exe"
Write-Host "Build successful: $ExePath"

# Output the path for other scripts
Write-Output $ExePath 
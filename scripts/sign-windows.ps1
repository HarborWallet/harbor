param(
    [Parameter(Mandatory=$true)]
    [string]$FilePath,

    [Parameter(Mandatory=$true)]
    [string]$CertificatePfxBase64,

    [Parameter(Mandatory=$true)]
    [string]$CertificatePassword,

    [string]$TimestampServer = "http://timestamp.comodoca.com", # Or use digicert, etc.
    [string]$SignToolPath # Optional: Path to signtool.exe if not in PATH
)

Write-Host "Signing $FilePath..."

# Decode the PFX certificate from Base64
$pfxFileName = "certificate.pfx"
try {
    $pfxBytes = [System.Convert]::FromBase64String($CertificatePfxBase64)
    [System.IO.File]::WriteAllBytes($pfxFileName, $pfxBytes)
} catch {
    Write-Error "Failed to decode PFX certificate from Base64: $($_.Exception.Message)"
    exit 1
}

# Find signtool.exe
if (-not $SignToolPath) {
    # Common location in Windows SDK - Adjust if necessary based on runner setup
    $sdkDirs = @(
        (Get-ItemProperty -Path 'HKLM:\SOFTWARE\Wow6432Node\Microsoft\Microsoft SDKs\Windows\v10.0' -Name 'InstallationFolder' -ErrorAction SilentlyContinue).InstallationFolder,
        (Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Microsoft SDKs\Windows\v10.0' -Name 'InstallationFolder' -ErrorAction SilentlyContinue).InstallationFolder
    )
    $sdkBase = $sdkDirs | Where-Object { $_ } | Select-Object -First 1
    if ($sdkBase) {
       $signtoolPaths = Get-ChildItem -Path "$sdkBase\bin" -Recurse -Filter signtool.exe | Where-Object { $_.FullName -like '*\x64\signtool.exe' } | Select-Object -ExpandProperty FullName -First 1
       if ($signtoolPaths) {
          $SignToolPath = $signtoolPaths
       } else {
           Write-Warning "signtool.exe not found in expected SDK paths. Trying PATH..."
           $SignToolPath = Get-Command signtool.exe -ErrorAction SilentlyContinue
           if (-not $SignToolPath) {
               Write-Error "signtool.exe not found in SDK or PATH. Please install the Windows SDK or provide -SignToolPath."
               Remove-Item $pfxFileName -ErrorAction SilentlyContinue
               exit 1
           }
       }
    } else {
         Write-Error "Windows SDK not found. Cannot locate signtool.exe."
         Remove-Item $pfxFileName -ErrorAction SilentlyContinue
         exit 1
    }
    Write-Host "Using signtool at: $SignToolPath"
}

# Sign the file
$signArgs = @(
    "sign",
    "/f", $pfxFileName,
    "/p", $CertificatePassword,
    "/tr", $TimestampServer,
    "/td", "sha256", # Time digest algorithm
    "/fd", "sha256", # File digest algorithm
    "`"$FilePath`""  # Quote the file path
)

& $SignToolPath $signArgs
$exitCode = $LASTEXITCODE

# Clean up the temporary PFX file
Remove-Item $pfxFileName -ErrorAction SilentlyContinue

if ($exitCode -ne 0) {
    Write-Error "Signtool failed for $FilePath with exit code $exitCode."
    exit 1
}

Write-Host "Successfully signed $FilePath" 
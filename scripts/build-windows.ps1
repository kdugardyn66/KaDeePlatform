<#
.SYNOPSIS
  Build the KaDee Platform (KadeeStudio) for Windows into dist\.

.EXAMPLE
  .\scripts\build-windows.ps1                 # x64 release → portable .zip
  .\scripts\build-windows.ps1 -Msi            # also an .msi installer (needs the WiX CLI)
  .\scripts\build-windows.ps1 -Arch arm64     # Windows on ARM
  .\scripts\build-windows.ps1 -Run            # build and start the app

.NOTES
  Needs Rust with the MSVC toolchain (rustup default stable-msvc) and the
  "Desktop development with C++" workload of Visual Studio Build Tools
  (GPUI draws with DirectX 11; the Windows SDK it needs comes with that workload).
  For -Msi:  dotnet tool install --global wix
             wix extension add --global WixToolset.UI.wixext
  Optional code signing: set KADEE_SIGN_CERT (path to .pfx) and KADEE_SIGN_PASSWORD.
#>
[CmdletBinding()]
param(
  [ValidateSet('x64', 'arm64')] [string] $Arch = 'x64',
  [switch] $Msi,
  [switch] $Run,
  [switch] $Clean
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root
$AppName = 'KaDeePlatform'   # file names of the packages
$ExeName = 'KadeeStudio'     # the installed program
$BinName = 'kadee-studio'    # cargo package and binary
$Dist = Join-Path $Root 'dist'

function Step($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Die($msg) { Write-Host "error: $msg" -ForegroundColor Red; exit 1 }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { Die 'Rust is not installed: https://rustup.rs' }

$Version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches.Groups[1].Value
if (-not $Version) { Die 'could not read the version from Cargo.toml' }

$Target = if ($Arch -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }

if ($Clean) { Step 'cargo clean'; cargo clean }

Step "Building $Target (release)"
if (Get-Command rustup -ErrorAction SilentlyContinue) { rustup target add $Target | Out-Null }
cargo build --release --locked -p $BinName --target $Target
if ($LASTEXITCODE -ne 0) { Die 'cargo build failed' }
$Exe = Join-Path $Root "target\$Target\release\$BinName.exe"

# Optional Authenticode signing
if ($env:KADEE_SIGN_CERT) {
  Step 'Signing the executable'
  $signtool = Get-Command signtool -ErrorAction SilentlyContinue
  if (-not $signtool) { Die 'signtool.exe not found (install the Windows SDK)' }
  & $signtool.Source sign /f $env:KADEE_SIGN_CERT /p $env:KADEE_SIGN_PASSWORD /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $Exe
  if ($LASTEXITCODE -ne 0) { Die 'signing failed' }
}

New-Item -ItemType Directory -Force -Path $Dist | Out-Null

# Portable zip
$Base = "$AppName-$Version-windows-$Arch"
Step "Creating $Base.zip"
$Stage = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
$Dir = Join-Path $Stage $Base
New-Item -ItemType Directory -Force -Path $Dir | Out-Null
Copy-Item $Exe (Join-Path $Dir "$ExeName.exe")
Copy-Item README.md, CHANGELOG.md, LICENSE-MIT, LICENSE-APACHE $Dir
$Zip = Join-Path $Dist "$Base.zip"
if (Test-Path $Zip) { Remove-Item $Zip }
Compress-Archive -Path $Dir -DestinationPath $Zip
Remove-Item -Recurse -Force $Stage

# MSI installer (WiX v4 or later)
if ($Msi) {
  $wix = Get-Command wix -ErrorAction SilentlyContinue
  if (-not $wix) { Die 'WiX CLI not found: dotnet tool install --global wix' }
  $MsiPath = Join-Path $Dist "$Base.msi"
  Step "Creating $(Split-Path -Leaf $MsiPath)"
  # MSI versions are numeric only: strip any pre-release suffix (0.2.0-beta.1 → 0.2.0).
  $MsiVersion = ($Version -split '-')[0]
  & $wix.Source build -arch $Arch -ext WixToolset.UI.wixext `
    -d "Version=$MsiVersion" `
    -d "Exe=$Exe" `
    -d "Icon=$Root\assets\app-icon\kadeestudio.ico" `
    -d "License=$Root\packaging\windows\license.rtf" `
    -o $MsiPath "$Root\packaging\windows\kadee-platform.wxs"
  if ($LASTEXITCODE -ne 0) { Die 'wix build failed' }
  if ($env:KADEE_SIGN_CERT) {
    & (Get-Command signtool).Source sign /f $env:KADEE_SIGN_CERT /p $env:KADEE_SIGN_PASSWORD /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $MsiPath
  }
}

# Checksums
Get-ChildItem $Dist -File | Where-Object { $_.Name -like "$Base*" -and $_.Extension -ne '.sha256' } | ForEach-Object {
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$h  $($_.Name)" | Set-Content -Encoding ascii "$($_.FullName).sha256"
}

Step 'Done'
Get-ChildItem $Dist -File | Where-Object { $_.Name -like "$Base*" -and $_.Extension -ne '.sha256' } | ForEach-Object { "  $($_.Name)" }

if ($Run) { & $Exe }

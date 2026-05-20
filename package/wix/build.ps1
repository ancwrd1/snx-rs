# Build the Windows MSI installer using the dotnet `wix` tool. Must be run on
# a Windows host: WiX v7's bind phase needs msi.dll, unavailable on Linux.
#
# Inputs:
#   - Release binaries must already exist under target/<triple>/<profile>/.
#     Build with e.g.: cargo build --target x86_64-pc-windows-msvc --profile lto
#   - `wix` v4+ on PATH: dotnet tool install --global wix
#
# Env overrides: $env:TARGET, $env:PROFILE, $env:VERSION.
# Output: target/snx-rs-<version>-x64.msi
#
# Requires the WixToolset.UI.wixext extension (for the WixUI_InstallDir dialog
# set). The script auto-installs it via `wix extension add -g` — that's
# idempotent and a no-op if it's already present.

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$basedir = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$target  = Join-Path $basedir 'target'
$triple  = if ($env:TARGET)  { $env:TARGET }  else { 'x86_64-pc-windows-msvc' }
$profile = if ($env:PROFILE) { $env:PROFILE } else { 'lto' }
$bindir  = Join-Path $target (Join-Path $triple $profile)

if ($env:VERSION) {
    $version = $env:VERSION
} else {
    $version = (& git -C $basedir describe --tags --always).Trim() -replace '^v',''
}
# WiX wants Major.Minor.Build.Revision (numeric only). MSI ignores the 4th
# field for upgrade detection, but we need *some* difference between dev
# builds, so encode `git describe`'s commits-since-tag into the revision
# field. Combined with AllowSameVersionUpgrades="yes" in the wxs, this keeps
# upgrades working between tagged releases AND between untagged dev builds
# off the same tag.
if ($version -match '^(\d+\.\d+\.\d+)(?:-(\d+)-g[0-9a-f]+)?') {
    $rev = if ($matches[2]) { [int]$matches[2] } else { 0 }
    $msiVersion = "$($matches[1]).$rev"
} else {
    $msiVersion = ($version -split '-')[0]
}

$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("snx-rs-wix-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage -Force | Out-Null

try {
    foreach ($app in 'snx-rs.exe','snxctl.exe','snx-rs-gui.exe') {
        $src = Join-Path $bindir $app
        if (-not (Test-Path -LiteralPath $src)) {
            throw "missing binary: $src`nbuild with: cargo build --target $triple --profile $profile"
        }
        Copy-Item -LiteralPath $src -Destination $stage
    }

    $registry = Join-Path $env:USERPROFILE '.cargo\registry\src'
    $wintunSrc = Get-ChildItem -Path $registry -Recurse -Filter 'wintun.dll' -ErrorAction SilentlyContinue |
                 Where-Object { $_.FullName -match 'wintun-bindings-[^\\/]+[\\/]wintun[\\/]bin[\\/]amd64[\\/]wintun\.dll$' } |
                 Select-Object -First 1
    if (-not $wintunSrc) {
        throw "wintun.dll not found under $registry; run 'cargo fetch' or build first"
    }
    Copy-Item -LiteralPath $wintunSrc.FullName -Destination (Join-Path $stage 'wintun.dll')

    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'snx-rs.png') -Destination $stage

    $wixVersionRaw = (& wix --version | Select-Object -First 1).Trim()
    if ($wixVersionRaw -notmatch '(\d+\.\d+\.\d+(?:\.\d+)?)') {
        throw "could not parse wix version from '$wixVersionRaw'"
    }
    $wixVersion = $matches[1]
    & wix extension add -g "WixToolset.UI.wixext/$wixVersion" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "failed to install WixToolset.UI.wixext/$wixVersion" }

    $out = Join-Path $target ("snx-rs-$version-x64.msi")
    & wix build `
        -arch x64 `
        -ext "WixToolset.UI.wixext/$wixVersion" `
        -d "Version=$msiVersion" `
        -d "StageDir=$stage" `
        -d "LicenseRtf=$(Join-Path $PSScriptRoot 'license.rtf')" `
        -o $out `
        (Join-Path $PSScriptRoot 'snx-rs.wxs')
    if ($LASTEXITCODE -ne 0) { throw "wix build failed (exit $LASTEXITCODE)" }

    Write-Host "Built: $out"
}
finally {
    Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
}

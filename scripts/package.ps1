# Builds the download: target\package\idiff-<version>-x64.zip (idiff.exe, LICENSE, README.md)
# and SHA256SUMS.txt beside it.
#
#   pwsh -File scripts\package.ps1 -Version 1.0.0
param([Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$')][string]$Version)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$crate = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version = "(.+)"$').Matches[0].Groups[1].Value
if ($crate -ne $Version) { throw "Cargo.toml says $crate, but the release is $Version" }

Push-Location $root
try {
    cargo build --release --locked -p idiff
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

    $out = Join-Path $root 'target\package'
    $stage = Join-Path $out "idiff-$Version-x64"
    Remove-Item -Recurse -Force $out -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force $stage | Out-Null
    Copy-Item 'target\release\idiff.exe', 'LICENSE', 'README.md' -Destination $stage

    # The program must start and report this version before it ships.
    $reported = & (Join-Path $stage 'idiff.exe') --version
    if ($reported -notmatch [regex]::Escape($Version)) { throw "idiff.exe reports '$reported', not $Version" }

    $zip = Join-Path $out "idiff-$Version-x64.zip"
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip
    Remove-Item -Recurse -Force $stage

    $hash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
    # sha256sum's format, so the publish job can check it with `sha256sum -c`.
    Set-Content -Path (Join-Path $out 'SHA256SUMS.txt') -Value "$hash  $(Split-Path $zip -Leaf)" -Encoding ascii -NoNewline
    Write-Host "$zip`n$hash"
}
finally { Pop-Location }

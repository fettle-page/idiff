# Makes winget work on a GitHub windows-2025 runner, which doesn't ship it. Used by the CI smoke job.
# Repair-WinGetPackageManager -AllUsers fails there (0x80073CF1: a dependency package isn't found),
# so: the module's repair for this user first, then winget's own release with its dependencies.
$ErrorActionPreference = 'Stop'

function Test-Winget {
    try { $v = & winget --version 2>$null; if ($LASTEXITCODE -eq 0 -and $v) { Write-Host "winget $v"; return $true } } catch {}
    $false
}

function Add-WindowsAppsToPath {
    $apps = Join-Path $env:LOCALAPPDATA 'Microsoft\WindowsApps'
    if ($env:PATH -notlike "*$apps*") { $env:PATH = "$apps;$env:PATH" }
    if ($env:GITHUB_PATH) { Add-Content -Path $env:GITHUB_PATH -Value $apps }
}

if (Test-Winget) { exit 0 }

Write-Host '== Repair-WinGetPackageManager (this user)'
try {
    Install-Module Microsoft.WinGet.Client -Force -Scope CurrentUser
    Repair-WinGetPackageManager -Latest -Force
    Add-WindowsAppsToPath
    if (Test-Winget) { exit 0 }
}
catch { Write-Warning "Repair failed: $_" }

Write-Host '== winget release with its dependencies'
$dir = Join-Path $env:RUNNER_TEMP 'winget'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
$headers = @{ 'User-Agent' = 'idiff-ci' }
if ($env:GH_TOKEN) { $headers.Authorization = "Bearer $env:GH_TOKEN" }
$release = Invoke-RestMethod -Uri 'https://api.github.com/repos/microsoft/winget-cli/releases/latest' -Headers $headers
function Get-Asset([string]$Pattern) {
    $asset = $release.assets | Where-Object name -Like $Pattern | Select-Object -First 1
    if (-not $asset) { throw "winget release $($release.tag_name) has no asset like $Pattern" }
    $path = Join-Path $dir $asset.name
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $path -Headers @{ 'User-Agent' = 'idiff-ci' }
    $path
}
$bundle = Get-Asset 'Microsoft.DesktopAppInstaller_8wekyb3d8bbwe.msixbundle'
$deps = Get-Asset 'DesktopAppInstaller_Dependencies.zip'
$license = Get-Asset '*License1.xml'
Expand-Archive -Path $deps -DestinationPath (Join-Path $dir 'deps') -Force
$depFiles = Get-ChildItem -Path (Join-Path $dir 'deps') -Recurse -File -Include *.appx, *.msix |
    Where-Object { $_.FullName -match '\\x64\\' -or $_.Name -match '_x64_' }
Write-Host "release $($release.tag_name): $($depFiles.Count) dependencies"
foreach ($f in $depFiles) {
    try { Add-AppxPackage -Path $f.FullName } catch { Write-Warning "$($f.Name): $_" }
}
Add-AppxProvisionedPackage -Online -PackagePath $bundle -DependencyPackagePath $depFiles.FullName -LicensePath $license | Out-Null
Add-AppxPackage -Path $bundle -DependencyPath $depFiles.FullName
Add-WindowsAppsToPath
if (Test-Winget) { exit 0 }
throw 'winget is still not available'

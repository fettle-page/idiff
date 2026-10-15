# One Receipts Library job, on a fresh GitHub runner: install the app with `idiff run`,
# uninstall it with `idiff leftovers`, and leave everything in <Out>/<id>/ for the library
# generator. Never run this on your own PC: it installs and uninstalls software.
#
#   pwsh -File scripts/catalog-run.ps1 -Id VideoLAN.VLC -Name VLC -Out results [-Idiff target\release\idiff.exe]
#
# <Out>/<id>/ gets:
#   meta.json       written first and updated after each step. `failed` names the step that
#                   didn't finish, so a job killed mid-install still says where it stopped.
#   before.json     the session's before snapshot, copied as soon as idiff writes it.
#   receipt.json    the install receipt; leftovers.json the uninstall one. Not redacted here:
#                   runners hold no personal data, and the library generator redacts anyway.
#   run.log, leftovers.log   idiff's output.
# Always exits 0, so the workflow uploads whatever there is.
param(
    [Parameter(Mandatory)][ValidatePattern('^[A-Za-z0-9][A-Za-z0-9._+-]*$')][string]$Id,
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$Out,
    [string]$Idiff = 'idiff'
)

$dir = Join-Path ([IO.Path]::GetFullPath($Out, (Get-Location).Path)) $Id
New-Item -ItemType Directory -Force -Path $dir | Out-Null
# A home of its own, so the session folder is known and holds nothing else.
$env:IDIFF_HOME = Join-Path $dir 'idiff-home'

$runUrl = if ($env:GITHUB_RUN_ID) { "$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID" }
$meta = [ordered]@{
    id            = $Id
    name          = $Name
    version       = $null
    date          = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
    idiff_version = $null
    runner_image  = "$env:ImageOS $env:ImageVersion".Trim()
    run_url       = $runUrl
    failed        = 'start: didn''t finish'
}

function Save-Meta {
    $meta | ConvertTo-Json | Set-Content -Path (Join-Path $dir 'meta.json') -Encoding utf8NoBOM
}

function Find-SessionFile([string]$File) {
    Get-ChildItem -Path (Join-Path $env:IDIFF_HOME 'sessions') -Filter $File -Recurse -File -ErrorAction SilentlyContinue |
        Select-Object -First 1
}

# Runs idiff with a deadline, copying before.json out of the session as soon as it appears.
# Returns the exit code, or $null if it had to be stopped.
function Invoke-Idiff([string[]]$IdiffArgs, [string]$Log, [int]$Minutes) {
    $p = Start-Process -FilePath $Idiff -ArgumentList $IdiffArgs -NoNewWindow -PassThru `
        -RedirectStandardOutput (Join-Path $dir $Log) -RedirectStandardError (Join-Path $dir "$Log.err")
    $null = $p.Handle # keeps ExitCode readable after the process ends
    $deadline = (Get-Date).AddMinutes($Minutes)
    $copied = Test-Path (Join-Path $dir 'before.json')
    while (-not $p.HasExited -and (Get-Date) -lt $deadline) {
        if (-not $copied -and ($before = Find-SessionFile 'before.json')) {
            Copy-Item $before.FullName (Join-Path $dir 'before.json')
            $copied = $true
        }
        Start-Sleep -Seconds 2
    }
    if (-not $p.HasExited) {
        taskkill /PID $p.Id /T /F 2>&1 | Out-Null
        return $null
    }
    if (-not $copied -and ($before = Find-SessionFile 'before.json')) {
        Copy-Item $before.FullName (Join-Path $dir 'before.json')
    }
    $p.ExitCode
}

# Both receipts as idiff exports them, split into the library's two files.
function Save-Receipts {
    $export = Join-Path $dir 'export.json'
    & $Idiff export last --format json --no-redact -o $export 2>> (Join-Path $dir 'export.log')
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $export)) { return }
    $both = Get-Content $export -Raw | ConvertFrom-Json
    $both.install | ConvertTo-Json -Depth 32 | Set-Content (Join-Path $dir 'receipt.json') -Encoding utf8NoBOM
    if ($both.leftovers) {
        $both.leftovers | ConvertTo-Json -Depth 32 | Set-Content (Join-Path $dir 'leftovers.json') -Encoding utf8NoBOM
    }
    Remove-Item $export
}

# The installed version, found the way idiff finds the app (spec §6): the biggest new program
# entry that isn't a system component, else a new Store package. Only after an install that
# exited 0: a failed one installed nothing, and anything new then is something else. Store
# packages published by Microsoft only count for Microsoft's own apps, since winget refreshes its
# own source package (Microsoft.Winget.Source) during runs.
function Get-InstalledVersion([object]$ExitCode, [string]$AppId) {
    if ($ExitCode -ne 0) { return $null }
    $beforeFile, $afterFile = (Find-SessionFile 'before.json'), (Find-SessionFile 'after.json')
    if (-not $beforeFile -or -not $afterFile) { return $null }
    $before = Get-Content $beforeFile.FullName -Raw | ConvertFrom-Json -AsHashtable
    $after = Get-Content $afterFile.FullName -Raw | ConvertFrom-Json -AsHashtable
    $microsoftApp = $AppId -like 'Microsoft.*'
    foreach ($source in 'Programs', 'Packaged') {
        $old = @{}
        foreach ($i in $before.sources[$source].items) { $old[$i.key] = $true }
        $new = @($after.sources[$source].items | Where-Object {
                $publisher = "$($_.props.publisher)"
                $byMicrosoft = $publisher -like 'Microsoft*' -or $publisher -like '*CN=Microsoft Corporation*'
                -not $old.ContainsKey($_.key) -and $_.props.system_component -ne '1' -and $_.props.version -and
                -not ($source -eq 'Packaged' -and $byMicrosoft -and -not $microsoftApp)
            })
        if ($new) {
            return ($new | Sort-Object { [long]($_.props.size_kb ?? 0) } -Descending | Select-Object -First 1).props.version
        }
    }
    $null
}

try {
    Save-Meta
    # What the runner image changes by itself (its preinstalled Firefox) is hidden, and counted,
    # in these receipts only: idiff reads a noise.local.toml from its home.
    New-Item -ItemType Directory -Force -Path $env:IDIFF_HOME | Out-Null
    Copy-Item -Path (Join-Path $PSScriptRoot '..\catalog\runner-noise.toml') `
        -Destination (Join-Path $env:IDIFF_HOME 'noise.local.toml') -ErrorAction Stop
    $meta.idiff_version = ((& $Idiff --version) -split ' ')[-1]

    # 1. Install.
    $meta.failed = 'install: didn''t finish'
    Save-Meta
    $code = Invoke-Idiff -Log 'run.log' -Minutes 25 -IdiffArgs @(
        'run', '--yes', '--settle', '30', '--timeout', '20', '--app', "`"$Name`"", '--',
        'winget', 'install', '--id', $Id, '--exact', '--silent',
        '--accept-package-agreements', '--accept-source-agreements', '--disable-interactivity')
    $meta.version = Get-InstalledVersion -ExitCode $code -AppId $Id
    Save-Receipts
    # Exit 2 means the installer failed but the receipt was still made; 1 means idiff itself failed.
    $installed = $null -ne $code -and $code -ne 1 -and (Find-SessionFile 'after.json')
    $meta.failed = if ($null -eq $code) { 'install: stopped after 25 minutes' } elseif ($code -ne 0) { "install: exit $code" } else { $null }
    Save-Meta

    # 2. Uninstall, if there's an install to compare against.
    if ($installed) {
        $installFailed = $meta.failed
        $meta.failed = $installFailed ?? 'leftovers: didn''t finish'
        Save-Meta
        # Uninstallers that work finish in a minute or two; on runners many wait for a dialog nobody
        # can click, forever. Five minutes tells them apart without spending Actions minutes on waiting.
        $code = Invoke-Idiff -Log 'leftovers.log' -Minutes 8 -IdiffArgs @(
            'leftovers', 'last', '--yes', '--settle', '30', '--timeout', '5', '--',
            'winget', 'uninstall', '--id', $Id, '--exact', '--silent',
            '--accept-source-agreements', '--disable-interactivity')
        Save-Receipts
        $leftoversFailed = if ($null -eq $code) { 'leftovers: stopped after 8 minutes' } elseif ($code -ne 0) { "leftovers: exit $code" }
        # The first step that went wrong is the one to report.
        $meta.failed = $installFailed ?? $leftoversFailed
        Save-Meta
    }
}
catch {
    $meta.failed = "$($meta.failed ?? 'script'): $_"
    Save-Meta
}

exit 0

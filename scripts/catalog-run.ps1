# One Receipts Library job, on a fresh GitHub runner: install the app with `idiff run`,
# uninstall it with `idiff leftovers`, and leave everything in <Out>/<id>/ for the library
# generator. Never run this on your own PC: it installs and uninstalls software.
#
#   pwsh -File scripts/catalog-run.ps1 -Id VideoLAN.VLC -Name VLC -Out results [-UninstallArgs /S] [-Idiff target\release\idiff.exe]
#
# -UninstallArgs is the app's uninstall_args from catalog/apps.toml. Without it the uninstall is
# `winget uninstall --silent`, which runs the app's QuietUninstallString if it has one and its
# plain UninstallString if not, adding no switches; many exe uninstallers then wait for a dialog.
# With it, the uninstall runs the UninstallString of the app's Uninstall entry with these
# arguments added after its own, and falls back to winget if there's no such entry.
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
    # The same characters the workflow allows: these reach a command line unquoted.
    [ValidatePattern('^[A-Za-z0-9 /_=:.-]*$')][string]$UninstallArgs = '',
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

# The windows of processes started since $Since, in the job log: the dialog an uninstaller that
# ran out of time was waiting on. Only the job log; the receipts stay as idiff made them.
function Write-OpenWindows([datetime]$Since) {
    $open = @(Get-Process | Where-Object {
            $p = $_
            # StartTime can't be read for some system processes; none of those is the uninstaller.
            $p.MainWindowTitle -and $(try { $p.StartTime -ge $Since } catch { $false })
        })
    if (-not $open) {
        Write-Host 'No process started since the uninstall has a window open.'
        return
    }
    foreach ($p in $open) {
        Write-Host "Window still open: `"$($p.MainWindowTitle)`" from $($p.ProcessName) (pid $($p.Id), $($p.Path))"
    }
}

# Runs idiff with a deadline, copying before.json out of the session as soon as it appears.
# Returns the exit code, or $null if it had to be stopped. With -ShowWindows, a run that didn't
# succeed writes the windows still open to the job log, before anything is stopped.
function Invoke-Idiff([string[]]$IdiffArgs, [string]$Log, [int]$Minutes, [switch]$ShowWindows) {
    $started = Get-Date
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
        if ($ShowWindows) { Write-OpenWindows $started }
        taskkill /PID $p.Id /T /F 2>&1 | Out-Null
        return $null
    }
    if (-not $copied -and ($before = Find-SessionFile 'before.json')) {
        Copy-Item $before.FullName (Join-Path $dir 'before.json')
    }
    # idiff leaves an uninstaller that outlived its --timeout running, so its window is still there.
    if ($ShowWindows -and $p.ExitCode -ne 0) { Write-OpenWindows $started }
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
            # idiff was given --app, so the entry named like the app wins over a bigger runtime that
            # came along (HandBrake's .NET Desktop Runtime would otherwise give it version 10.0.13).
            $named = @($new | Where-Object { "$($_.label)".IndexOf($Name, [StringComparison]::OrdinalIgnoreCase) -ge 0 })
            if ($named) { $new = $named }
            return ($new | Sort-Object { [long]($_.props.size_kb ?? 0) } -Descending | Select-Object -First 1).props.version
        }
    }
    $null
}

# The app's Uninstall entry: of the program entries the install added, the one named like the app
# (named exactly first, then the biggest), else any entry in the registry whose DisplayName has the
# app's name in it. Returns the key's values, or $null if none has an UninstallString. Only reads.
function Find-UninstallEntry {
    $roots = @{
        'hklm\native'      = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
        'hklm\wow6432node' = 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
        'hkcu\native'      = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
    }
    $hasName = { param($label) "$label".IndexOf($Name, [StringComparison]::OrdinalIgnoreCase) -ge 0 }
    $paths = @()
    $beforeFile, $afterFile = (Find-SessionFile 'before.json'), (Find-SessionFile 'after.json')
    if ($beforeFile -and $afterFile) {
        $before = Get-Content $beforeFile.FullName -Raw | ConvertFrom-Json -AsHashtable
        $after = Get-Content $afterFile.FullName -Raw | ConvertFrom-Json -AsHashtable
        $old = @{}
        foreach ($i in $before.sources['Programs'].items) { $old[$i.key] = $true }
        $paths = @($after.sources['Programs'].items |
                Where-Object { -not $old.ContainsKey($_.key) -and $_.props.system_component -ne '1' -and (& $hasName $_.label) } |
                Sort-Object @{ Expression = { "$($_.label)" -eq $Name }; Descending = $true },
                @{ Expression = { [long]($_.props.size_kb ?? 0) }; Descending = $true } |
                ForEach-Object {
                    # Keys are "<hive>\<view>\<subkey>", as idiff's Programs source writes them.
                    $hive, $view, $subkey = $_.key -split '\\', 3
                    $root = $roots["$hive\$view"]
                    if ($root -and $subkey) { Join-Path $root $subkey }
                })
    }
    if (-not $paths) {
        $paths = @(foreach ($root in $roots.Values) {
                Get-ChildItem -Path $root -ErrorAction SilentlyContinue |
                    Where-Object { (& $hasName $_.GetValue('DisplayName')) -and $_.GetValue('SystemComponent') -ne 1 } |
                    ForEach-Object PSPath
            })
    }
    foreach ($path in $paths) {
        $entry = Get-ItemProperty -LiteralPath $path -ErrorAction SilentlyContinue
        if ($entry -and "$($entry.UninstallString)".Trim()) { return $entry }
    }
    $null
}

# An UninstallString split into the program and the arguments it already has. Most quote the
# program; some don't though its path has spaces (WinRAR's), so an unquoted one ends at ".exe".
function Split-CommandLine([string]$Line) {
    $Line = $Line.Trim()
    if ($Line -match '^"([^"]+)"\s*(.*)$') { return $Matches[1], $Matches[2] }
    if ($Line -match '^(.+?\.exe)(?:\s+(.*))?$') { return $Matches[1], "$($Matches[2])" }
    $Line, ''
}

# What `idiff leftovers` runs after the `--`: the app's own uninstaller with -UninstallArgs when
# there are some and its entry has an UninstallString, else winget. Says which in the job log.
function Get-UninstallCommand {
    $winget = @('winget', 'uninstall', '--id', $Id, '--exact', '--silent',
        '--accept-source-agreements', '--disable-interactivity')
    $entry = Find-UninstallEntry
    if ($entry) {
        Write-Host "Uninstall entry $($entry.PSChildName) ($($entry.DisplayName)): UninstallString $($entry.UninstallString); QuietUninstallString $($entry.QuietUninstallString ?? '(none)')"
    }
    if (-not $UninstallArgs.Trim()) { return $winget }
    if (-not $entry) {
        Write-Host "No Uninstall entry named like `"$Name`"; uninstalling with winget instead."
        return $winget
    }
    $program, $own = Split-CommandLine $entry.UninstallString
    # An MSI's entry runs msiexec /I or /X with its product code; winget already uninstalls
    # those quietly, and switches added after /I would repair it instead.
    if ((Split-Path -Leaf $program) -match '^msiexec(\.exe)?$' -or -not (Test-Path -LiteralPath $program -PathType Leaf)) {
        Write-Host "UninstallString doesn't run an uninstaller to add arguments to ($program); uninstalling with winget instead."
        return $winget
    }
    Write-Host "Uninstalling with `"$program`" $own $UninstallArgs"
    # Start-Process joins these with spaces as they are: the program is quoted here, and the
    # UninstallString's own arguments stay as the installer wrote them.
    @("`"$program`"", $own, $UninstallArgs) | Where-Object { $_ }
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
        # can click, forever. Five minutes tells them apart without spending Actions minutes on waiting,
        # and the job log then names the windows still open.
        $code = Invoke-Idiff -Log 'leftovers.log' -Minutes 8 -ShowWindows -IdiffArgs (@(
                'leftovers', 'last', '--yes', '--settle', '30', '--timeout', '5', '--') + @(Get-UninstallCommand))
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

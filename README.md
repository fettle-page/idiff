# Install diff

Shows, in plain words, what an app's installer did to your Windows PC, and what it left behind after you uninstalled it.

```
> idiff run -- winget install Discord.Discord

Discord 1.0.9170: installed
⚠ 1 scheduled task: runs an updater every day
⚠ Starts with Windows
✓ No services, drivers or firewall rules
✓ Files only in your user folder (412 MB)
· 3 Windows background changes hidden (idiff show --all)
```

```
> idiff leftovers last -- winget uninstall Discord.Discord

Discord 1.0.9170: after uninstall, 2 leftovers
✗ Scheduled task "DiscordUpdate" is still there
✗ 340 MB in AppData\Local\Discord
```

Installers can do far more than copy files: add background services with full system rights, schedule daily updaters, start with Windows, install drivers, open the firewall, add right-click menu entries. Windows doesn't tell you, and uninstalling often leaves some of it behind. idiff looks at exactly those places before and after, filters out what Windows changed on its own in the meantime, and prints a receipt.

Windows 10 and 11, 64-bit. Free and open source (MIT).

## Install

Download `idiff-<version>-x64.zip` from [Releases](../../releases), unzip it, and put `idiff.exe` somewhere on your PATH. Check the download against `SHA256SUMS.txt` and its [build attestation](https://docs.github.com/actions/security-for-github-actions/using-artifact-attestations). The program isn't code-signed yet, so Windows SmartScreen may warn the first time.

## Use

```
idiff run [--app NAME] [--yes] [--no-elevate] -- <install command>
idiff leftovers last [--yes] [-- <uninstall command>]
idiff show last [--all] [--why] [--json]
idiff export last --format md|html|json [-o FILE] [--no-redact]
idiff list
idiff snapshot [-o FILE]
idiff diff <before.json> <after.json>
idiff calibrate --minutes N [--save]
```

`run` takes a snapshot, runs your install command, asks you to open the app once (many apps add themselves to startup on first launch, not during the install), takes a second snapshot and prints the receipt. Every run is saved, so `show`, `export` and `leftovers` can come later. `--yes` skips the question and waits a few seconds instead.

Exit codes: 0 when there's a receipt, 1 when idiff itself failed, 2 when the install command failed (the receipt still prints).

## What it looks at

| | Where |
|---|---|
| Installed programs | the Uninstall lists (machine, 32-bit and user) |
| Services and drivers | `HKLM\SYSTEM\CurrentControlSet\Services` |
| Scheduled tasks | Task Scheduler, hidden tasks included |
| Startup | Run/RunOnce keys, Startup folders, Task Manager's enabled flags |
| Firewall rules | Windows Firewall's rule list |
| Right-click menu | shell extensions and menu commands for files, folders and drives |
| PATH | the user and machine PATH |
| Folders and shortcuts | new top-level folders in Program Files, ProgramData and AppData; desktop and Start menu shortcuts |
| Store apps | MSIX packages, their startup tasks and menu extensions |
| Trusted root certificates | the machine and user Root stores |
| Antivirus exclusions | Microsoft Defender's exclusion lists |
| Network settings | the hosts file, the proxy, Winsock providers |

Not the whole disk, and not the whole registry. A snapshot takes under a second.

## It only reads

idiff never changes, cleans, removes or "fixes" anything. `run` and `leftovers` run the one command you give them, and nothing else. Scheduled tasks and antivirus exclusions are only fully visible to an administrator, so idiff asks once through Windows' own prompt (UAC) to read those two, and only those two. Say no, or pass `--no-elevate`, and the receipt says what it couldn't check. Your own startup entries and folders are always read as you, never as the administrator.

## How it decides what's the app's

Between the two snapshots, Windows keeps working: Defender updates itself, Windows Update runs, services change state. idiff sorts every change:

1. **Windows' own background changes**, from a built-in list (`crates/idiff-core/noise.toml`), are hidden but counted on the receipt. `idiff show --all` lists them.
2. **The app's**, if what it runs sits in the app's install folder, is signed by the app's publisher, or lives in a folder named after the app.
3. **Windows', by signature:** a changed Windows item, or a new one signed exactly "Microsoft Windows" or "Microsoft Corporation". A file's location under `C:\Windows` is never enough on its own: some apps install their drivers and services there.
4. **Everything else** is listed under "Also changed while installing (couldn't tell if it was this app)". Nothing is hidden without being counted.

`idiff show --why` gives the reason for every line. `idiff calibrate --minutes 3` shows what your own PC changes by itself; `--save` adds those to your personal noise list.

## Privacy

Nothing leaves your PC. Sessions are stored in `%LOCALAPPDATA%\idiff`. `export` removes your user and PC names, profile folders, account IDs and addresses from the file before writing it; `--no-redact` keeps them and warns you.

## Receipts Library

Receipts for popular apps, made automatically on fresh test machines: coming with 1.0.

## About

Made by candygoogy. A sister project of [Fettle](https://github.com/fettle-page/fettle), a plain-words check-up for Windows PCs.

MIT licence.

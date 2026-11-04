# Bitwarden 2026.10.3

## Bitwarden 2026.10.3: installed

- ✓ No services, drivers or firewall rules
- ✓ Files only in your user folder \(903 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Local\\Programs"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Bitwarden 2026.10.3: after uninstall, 1 leftover

- ✗ 164 MB in AppData\\Local\\bitwarden-updater

## Changed since the previous version

- \+ Files only in your user folder \(903 MB\)
- \- Files only in your user folder \(599 MB\)

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [2026.9.1](2026.9.1.md) (2026-10-15)

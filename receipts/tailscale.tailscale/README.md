# Tailscale 1.104.7

## Tailscale 1.104.7: installed

- ⚠ Adds 1 background service: Tailscale \(starts with Windows\) \(full system rights\)
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for tailscaled.exe
- ✓ No drivers
- · Opens tailscale:// links
- · Adds to PATH: C:\\Program Files\\Tailscale\\
- · Files in Program Files \(71 MB\)
- · Files in ProgramData \(7 KB\)
- · Files in your user folder \(1 KB\)
- · Adds a Start menu entry
- · 5 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "wintun"
- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

### Tailscale 1.104.7: after uninstall, 3 leftovers

- ✗ Firewall rule "Tailscale-Process" is still there
- ✗ 32 KB in ProgramData\\Tailscale
- ✗ 1 KB in AppData\\Local\\Tailscale

## Changed since the previous version

- \+ Files in ProgramData \(7 KB\)
- \- Files in ProgramData \(5 KB\)

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261103.298.1) by idiff 1.1.0 on 2026-11-16, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/43852972131). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [1.104.5](1.104.5.md) (2026-11-04)

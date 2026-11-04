# Steam 2.10.91.91

## Steam 2.10.91.91: installed

- ⚠ Adds 1 background service: Steam Client Service \(full system rights\)
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for steam.exe
- ✓ No drivers
- · Opens steam://, steamlink:// links
- · Files in Program Files \(7 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 5 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

### Steam 2.10.91.91: after uninstall, 6 leftovers

- ✗ Firewall rule "Steam" is still there \(2 rules\)
- ✗ Link handler "steam" is still there \(2 items\)
- ✗ Link handler "steamlink" is still there \(2 items\)

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

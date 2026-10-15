# Steam 2.10.91.91

## Steam 2.10.91.91: installed

- ⚠ Adds 1 background service: Steam Client Service \(full system rights\)
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for steam.exe
- ✓ No drivers
- · Files in Program Files \(7 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Steam 2.10.91.91: after uninstall, 8 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Steam" is still there
- ✗ Service "Steam Client Service" is still there
- ✗ Startup entry "Steam" is still there
- ✗ Firewall rule "Steam" is still there \(2 rules\)
- ✗ 7 MB in Program Files \(x86\)\\Steam
- ✗ Shortcut "Start menu \(all users\)\\Steam\\Steam.lnk" is still there
- ✗ Shortcut "Public Desktop\\Steam.lnk" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

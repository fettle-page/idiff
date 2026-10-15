# qBittorrent 5.2.4

## qBittorrent 5.2.4: installed

- ⚠ Allows incoming connections for qbittorrent.exe
- ✓ No services or drivers
- · Files in Program Files \(209 MB\)
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### qBittorrent 5.2.4: after uninstall, 6 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "qBittorrent" is still there
- ✗ Firewall rule "qBittorrent" is still there \(2 rules\)
- ✗ 209 MB in Program Files\\qBittorrent
- ✗ Shortcut "Start menu \(all users\)\\qBittorrent\\qBittorrent.lnk" is still there
- ✗ Shortcut "Start menu \(all users\)\\qBittorrent\\Uninstall qBittorrent.lnk" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

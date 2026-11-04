# LibreOffice 26.8.2.2

## LibreOffice 26.8.2.2: installed

- ⚠ Adds 1 background service: LibreOffice Maintenance Service \(full system rights\)
- ✓ No drivers or firewall rules
- · Opens vnd.libreoffice.command:// links
- · Files in Program Files \(700 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### LibreOffice 26.8.2.2: after uninstall, 2 leftovers

- ✗ Service "LibreOffice Maintenance Service" is still there
- ✗ 1 KB in Program Files\\LibreOffice

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

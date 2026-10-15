# TeamViewer 15.82.6.0

## TeamViewer 15.82.6.0: installed

- ⚠ Adds 1 background service: TeamViewer \(starts with Windows\) \(full system rights\)
- ⚠ Allows incoming connections for teamviewer.exe, teamviewer\_service.exe
- ✓ No drivers
- · Files in Program Files \(217 MB\)
- · Files in your user folder \(4 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### TeamViewer 15.82.6.0: after uninstall, 2 leftovers

- ✗ 57 KB in Program Files\\TeamViewer
- ✗ 4 MB in AppData\\Local\\TeamViewer

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

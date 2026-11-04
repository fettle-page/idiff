# TightVNC 2.8.89.0

## TightVNC 2.8.89.0: installed

- ⚠ Adds 1 background service: TightVNC Server \(starts with Windows\) \(full system rights\)
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for tvnserver.exe
- ✓ No drivers
- · Files in Program Files \(3 MB\)
- · Files in ProgramData
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### TightVNC 2.8.89.0: after uninstall, no leftovers

- ✓ Nothing left behind

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

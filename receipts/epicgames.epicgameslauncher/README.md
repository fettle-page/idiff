# Epic Games Launcher 1.3.210.0

## Epic Games Launcher 1.3.210.0: installed

- ⚠ Adds 1 background service: Epic Games Updater \(full system rights\)
- ✓ No drivers or firewall rules
- · Files in Program Files \(555 MB\)
- · Files in ProgramData \(729 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · Also installed: Microsoft GameInput
- · 1 Windows background change hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added service "GameInput Redist Service"
- · Added folder "Program Files\\Microsoft GameInput"

### Hidden: Windows background changes

- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Epic Games Launcher 1.3.210.0: after uninstall, 1 leftover

- ✗ Folder "ProgramData\\Epic" is still there \(empty\)
- · Left installed: Microsoft GameInput. It came with the app, and other apps may use it.

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

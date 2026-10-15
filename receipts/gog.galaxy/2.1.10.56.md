# GOG Galaxy 2.1.10.56

## GOG Galaxy 2.1.10.56: installed

- ⚠ Adds 1 background service: GalaxyCommunication \(full system rights\)
- ⚠ Starts with Windows
- ✓ No drivers or firewall rules
- · Files in Program Files \(367 MB\)
- · Files in ProgramData \(291 MB\)
- · Files in your user folder \(3 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added service "GalaxyClientService"
- · Added startup entry "GalaxyClient"

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### GOG Galaxy 2.1.10.56: after uninstall, 1 leftover

- ✗ Folder "AppData\\Local\\GOG.com" is still there \(empty\)

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

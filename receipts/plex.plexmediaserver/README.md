# Plex Media Server 1.43.5.11079

## Plex Media Server 1.43.5.11079: installed

- ⚠ Adds 1 background service: Plex Update Service \(starts with Windows\) \(full system rights\)
- ⚠ Allows incoming connections for plexscripthost.exe, plex media server.exe, plex dlna server.exe, plex tuner service.exe
- ✓ No drivers
- · Files in Program Files \(227 MB\)
- · Files in your user folder \(3 MB\)
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Added folder "AppData\\Local\\PeerDistRepub"
- · Added folder "AppData\\Local\\Programs"
- · Added folder "AppData\\Local\\Programs\\Common"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Plex Media Server 1.43.5.11079: after uninstall, 2 leftovers

- ✗ Folder "Program Files\\Plex" is still there \(empty\)
- ✗ 4 MB in AppData\\Local\\Plex Media Server

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

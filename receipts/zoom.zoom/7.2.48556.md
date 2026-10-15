# Zoom Workplace 7.2.48556

## Zoom Workplace 7.2.48556: installed

- ⚠ Adds 1 background service: Zoom Sharing Service \(starts with Windows\) \(full system rights\)
- · Runs once at the next sign-in
- ⚠ Allows incoming connections for airhost.exe, zoom.exe, zoomhybridconf.exe
- ✓ No drivers
- · Files in Program Files \(561 MB\)
- · Files in your user folder
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Zoom Workplace 7.2.48556: after uninstall, 4 leftovers

- ✗ Firewall rule "Airhost service for Zoom Video Meetings" is still there \(2 rules\)
- ✗ Firewall rule "Zoom Video Meeting" is still there
- ✗ Firewall rule "Hybrid Conference for Zoom Video Meetings" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

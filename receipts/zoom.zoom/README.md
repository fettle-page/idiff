# Zoom Workplace 7.2.51704

## Zoom Workplace 7.2.51704: installed

- ⚠ Adds 1 background service: Zoom Sharing Service \(starts with Windows\) \(full system rights\)
- · Runs once at the next sign-in
- ⚠ Allows incoming connections for zoom.exe, airhost.exe, zoomhybridconf.exe
- ✓ No drivers
- · Opens zoompbx.zoomphonecall://, zoomphonecall://, zoommtg://, zoomus:// links
- · Files in Program Files \(563 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Zoom Workplace 7.2.51704: after uninstall, 6 leftovers

- ✗ Firewall rule "Zoom Video Meeting" is still there
- ✗ Firewall rule "Airhost service for Zoom Video Meetings" is still there \(2 rules\)
- ✗ Firewall rule "Hybrid Conference for Zoom Video Meetings" is still there
- ✗ Link handler "ZoomPhoneCall" is still there \(2 items\)

## Changed since the previous version

- \+ Allows incoming connections for zoom.exe, airhost.exe, zoomhybridconf.exe
- \+ Opens zoompbx.zoomphonecall://, zoomphonecall://, zoommtg://, zoomus:// links
- \+ Files in Program Files \(563 MB\)
- \- Allows incoming connections for airhost.exe, zoom.exe, zoomhybridconf.exe
- \- Files in Program Files \(561 MB\)
- \- Files in your user folder

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [7.2.48556](7.2.48556.md) (2026-10-15)

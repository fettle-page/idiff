# NordVPN 8.14.4.0

## NordVPN 8.14.4.0: installed

- ⚠ Installs 1 driver: NordVPN Divert Driver
- ⚠ Adds 3 background services: nordsec-threatprotection-service, NordSec Update Service, nordvpn-service \(starts with Windows\) \(full system rights\)
- ✓ No firewall rules
- · Opens nordvpn://, nordvpn.notification:// links
- · Files in Program Files \(401 MB\)
- · Files in ProgramData \(564 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · Also installed: Microsoft Windows Desktop Runtime - 8.0.33 \(x64\), NordUpdater
- · 5 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "ovpn-dco"
- · Added driver "WireGuard"

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### NordVPN 8.14.4.0: after uninstall, no leftovers

- ✓ Nothing left behind
- · Left installed: Microsoft .NET Host FX Resolver - 8.0.33 \(x64\), Microsoft .NET Host - 8.0.33 \(x64\), Microsoft Windows Desktop Runtime - 8.0.33 \(x64\). They came with the app, and other apps may use them.

## Changed since the previous version

- \+ Opens nordvpn://, nordvpn.notification:// links
- \+ Files in Program Files \(401 MB\)
- \+ Files in ProgramData \(564 KB\)
- \+ Also installed: Microsoft Windows Desktop Runtime - 8.0.33 \(x64\), NordUpdater
- \- Files in Program Files \(405 MB\)
- \- Files in ProgramData \(571 KB\)
- \- Also installed: Microsoft Windows Desktop Runtime - 8.0.32 \(x64\), NordUpdater

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [8.13.1.0](8.13.1.0.md) (2026-10-15)

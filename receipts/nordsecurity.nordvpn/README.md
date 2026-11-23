# NordVPN 8.16.1.0

## NordVPN 8.16.1.0: installed

- ⚠ Installs 1 driver: NordVPN Divert Driver
- ⚠ Adds 3 background services: nordsec-threatprotection-service, NordSec Update Service, nordvpn-service \(starts with Windows\) \(full system rights\)
- ✓ No firewall rules
- · Opens nordvpn://, nordvpn.notification:// links
- · Files in Program Files \(434 MB\)
- · Files in ProgramData \(605 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · Also installed: Microsoft Windows Desktop Runtime - 8.0.34 \(x64\), NordUpdater
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "ovpn-dco"
- · Added driver "WireGuard"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### NordVPN 8.16.1.0: after uninstall, no leftovers

- ✓ Nothing left behind
- · Left installed: Microsoft .NET Host - 8.0.34 \(x64\), Microsoft .NET Host FX Resolver - 8.0.34 \(x64\), Microsoft Windows Desktop Runtime - 8.0.34 \(x64\). They came with the app, and other apps may use them.

## Changed since the previous version

- \+ Files in Program Files \(434 MB\)
- \+ Files in ProgramData \(605 KB\)
- \+ Also installed: Microsoft Windows Desktop Runtime - 8.0.34 \(x64\), NordUpdater
- \- Files in Program Files \(401 MB\)
- \- Files in ProgramData \(564 KB\)
- \- Also installed: Microsoft Windows Desktop Runtime - 8.0.33 \(x64\), NordUpdater

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261116.315.1) by idiff 1.1.0 on 2026-11-23, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/45171434798). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [8.14.4.0](8.14.4.0.md) (2026-11-04)
- [8.13.1.0](8.13.1.0.md) (2026-10-15)

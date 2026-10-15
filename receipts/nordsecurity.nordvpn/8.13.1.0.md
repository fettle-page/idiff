# NordVPN 8.13.1.0

## NordVPN 8.13.1.0: installed

- ⚠ Installs 1 driver: NordVPN Divert Driver
- ⚠ Adds 3 background services: nordsec-threatprotection-service, NordSec Update Service, nordvpn-service \(starts with Windows\) \(full system rights\)
- ✓ No firewall rules
- · Files in Program Files \(405 MB\)
- · Files in ProgramData \(571 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · Also installed: Microsoft Windows Desktop Runtime - 8.0.32 \(x64\), NordUpdater
- · 2 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "ovpn-dco"
- · Added driver "WireGuard"

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### NordVPN 8.13.1.0: after uninstall, no leftovers

- ✓ Nothing left behind
- · Left installed: Microsoft .NET Host FX Resolver - 8.0.32 \(x64\), Microsoft .NET Runtime - 8.0.32 \(x64\), Microsoft Windows Desktop Runtime - 8.0.32 \(x64\), Microsoft .NET Host - 8.0.32 \(x64\). They came with the app, and other apps may use them.

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

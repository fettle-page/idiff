# Mullvad VPN 2026.5

## Mullvad VPN 2026.5: installed

- ⚠ Installs 1 driver: Mullvad Split Tunnel Service
- ⚠ Adds 1 background service: Mullvad VPN Service \(starts with Windows\) \(full system rights\)
- ✓ No firewall rules
- · Adds to PATH: C:\\Program Files\\Mullvad VPN\\resources
- · Files in Program Files \(439 MB\)
- · Files in ProgramData \(2 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 2 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Mullvad VPN 2026.5: after uninstall, no leftovers

- ✓ Nothing left behind

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261103.298.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

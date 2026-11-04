# ExpressVPN 14.3.1+15429

## ExpressVPN 14.3.1+15429: installed

- ⚠ Installs 3 drivers: expressvpn-ovpn-dco, expressvpn-pkf, ExpressVPN WFP Callout
- ⚠ Adds 2 background services: ExpressVPN Service, ExpressVPN WireGuard Service \(starts with Windows\) \(full system rights\)
- ⚠ Starts with Windows
- ✓ No firewall rules
- · Opens expressvpn://, expressvpn-desktop:// links
- · Files in Program Files \(197 MB\)
- · Files in your user folder \(5 MB\)
- · Adds a Start menu entry
- · 3 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### ExpressVPN 14.3.1+15429: after uninstall, 1 leftover

- ✗ Driver "expressvpn-ovpn-dco" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

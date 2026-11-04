# Surfshark 6.21.0999

## Surfshark 6.21.0999: installed

- ⚠ Installs 1 driver: SharkDivert
- ⚠ Adds 1 background service: Surfshark Service \(starts with Windows\) \(full system rights\)
- ✓ No firewall rules
- · Opens surfshark:// links
- · Files in Program Files \(400 MB\)
- · Files in ProgramData \(544 KB\)
- · Files in your user folder \(613 KB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "ovpn-dco"
- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Added folder "AppData\\Local\\PeerDistRepub"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Surfshark 6.21.0999: after uninstall, 11 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Surfshark" is still there \(2 items\)
- ✗ Service "Surfshark Service" is still there
- ✗ Driver "SharkDivert" is still there
- ✗ 400 MB in Program Files\\Surfshark
- ✗ Shortcut "Start menu \(all users\)\\Surfshark\\Surfshark\\Surfshark.lnk" is still there
- ✗ 544 KB in ProgramData\\Surfshark
- ✗ Shortcut "Public Desktop\\Surfshark.lnk" is still there
- ✗ 1 KB in AppData\\Local\\Surfshark
- ✗ 616 KB in AppData\\Roaming\\Surfshark
- ✗ Link handler "Surfshark" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

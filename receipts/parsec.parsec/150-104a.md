# Parsec 150-104a

## Parsec 150-104a: installed

- ⚠ Adds 1 background service: Parsec \(starts with Windows\) \(full system rights\)
- ⚠ Allows incoming connections for parsecd.exe
- ✓ No drivers
- · Opens parsec://, parsecd:// links
- · Files in Program Files \(10 MB\)
- · Adds a Start menu entry
- · Also installed: Parsec Virtual USB Adapter Driver
- · 3 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added driver "parsecvirtualds"
- · Added driver "parsecvusba"

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Added folder "AppData\\Local\\PeerDistRepub"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Parsec 150-104a: after uninstall, 9 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Parsec Virtual USB Adapter Driver" is still there
- ✗ Program "Parsec" is still there
- ✗ Service "Parsec" is still there
- ✗ Firewall rule "Parsec" is still there
- ✗ 9 MB in Program Files\\Parsec
- ✗ 2 MB in Program Files\\Parsec Virtual USB Adapter Driver
- ✗ Shortcut "Start menu \(all users\)\\Parsec\\Parsec.lnk" is still there
- ✗ Link handler "parsec" is still there
- ✗ Link handler "parsecd" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

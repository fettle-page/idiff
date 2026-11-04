# Playnite 10.58

## Playnite 10.58: installed

- ✓ No services, drivers or firewall rules
- · Opens playnite:// links
- ✓ Files only in your user folder \(462 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 7 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Added folder "AppData\\Local\\Programs"
- · Added folder "AppData\\Local\\Programs\\Common"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Playnite 10.58: after uninstall, 8 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Playnite" is still there
- ✗ 454 MB in AppData\\Local\\Playnite
- ✗ Shortcut "Start menu\\Playnite\\Playnite.lnk" is still there
- ✗ Shortcut "Start menu\\Playnite\\Safe Mode.lnk" is still there
- ✗ Shortcut "Start menu\\Playnite\\Uninstall.lnk" is still there
- ✗ 8 MB in AppData\\Roaming\\Playnite
- ✗ Shortcut "Desktop\\Playnite.lnk" is still there
- ✗ Link handler "Playnite" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

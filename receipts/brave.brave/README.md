# Brave 155.1.97.56

## Brave 155.1.97.56: installed

- ⚠ 2 scheduled tasks: runs an updater every day
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for brave.exe
- ✓ No services or drivers
- ✓ Files only in your user folder \(1.4 GB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added store app "WindowsAppRuntime.2"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Brave 155.1.97.56: after uninstall, 8 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Brave" is still there
- ✗ Scheduled task "BraveSoftwareUpdateTaskUserS-1-5-21-4239583824-265399229-1323058055-500Core{C9416284-FEBC-42BF-885D-3869D5E2F25A}" is still there
- ✗ Scheduled task "BraveSoftwareUpdateTaskUserS-1-5-21-4239583824-265399229-1323058055-500UA{F856713E-5F6F-465F-A791-69B36400DC45}" is still there
- ✗ Startup entry "BraveSoftware Update" is still there
- ✗ Firewall rule "Brave \(mDNS-In\)" is still there
- ✗ 1.2 GB in AppData\\Local\\BraveSoftware
- ✗ Shortcut "Start menu\\Brave.lnk" is still there
- ✗ Shortcut "Desktop\\Brave.lnk" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

# Dropbox 262.4.3183

## Dropbox 262.4.3183: installed

- ⚠ Adds 4 background services: Dropbox Update Service \(dbupdate\), Dropbox Update Service \(dbupdatem\), DbxSvc, Dropbox Elevation Service \(DropboxElevationService\) \(starts with Windows\) \(full system rights\)
- ⚠ 2 scheduled tasks: runs an updater at sign-in, as SYSTEM; runs an updater every day, as SYSTEM
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for dropbox.exe
- ✓ No drivers
- · Adds right-click menu entries
- · Files in Program Files \(1.7 GB\)
- · Files in ProgramData \(290 KB\)
- · Files in your user folder \(21 MB\)
- · Adds a Start menu entry
- · 7 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Local\\PeerDistRepub"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### Dropbox 262.4.3183: after uninstall, 20 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "Dropbox" is still there
- ✗ Program "Dropbox Update Helper" is still there
- ✗ Service "Dropbox Update Service \(dbupdate\)" is still there
- ✗ Service "Dropbox Update Service \(dbupdatem\)" is still there
- ✗ Service "Dropbox Elevation Service \(DropboxElevationService\)" is still there
- ✗ Scheduled task "DropboxUpdateTaskMachineCore" is still there
- ✗ Scheduled task "DropboxUpdateTaskMachineUA" is still there
- ✗ Startup entry "Dropbox" is still there
- ✗ Firewall rule "Dropbox" is still there \(2 rules\)
- ✗ Right-click menu entry "DropboxExt" is still there \(3 items\)
- ✗ 898 MB in Program Files \(x86\)\\Dropbox
- ✗ 346 KB in ProgramData\\Dropbox
- ✗ Shortcut "Start menu \(all users\)\\Dropbox\\Dropbox.lnk" is still there
- ✗ 18 MB in AppData\\Local\\Dropbox
- ✗ Folder "AppData\\Local\\DropboxUpdate" is still there \(empty\)
- ✗ 4 MB in AppData\\Roaming\\Dropbox
- ✗ Store app "Dropbox" is still there
- · The uninstaller added folder "Program Files\\Dropbox"

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

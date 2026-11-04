# WPS Office 12.2.0.23196

## WPS Office 12.2.0.23196: installed

- ⚠ Adds 1 background service: WPS Office Cloud Service \(full system rights\)
- ⚠ 4 scheduled tasks: runs wpscloudsvr.exe once; runs wpscloudsvr.exe at sign-in; runs an updater once; runs ksolaunch.exe at sign-in
- ⚠ Allows incoming connections for wpspdf.exe, et.exe, wpscloudsvr.exe, wpp.exe, promecefpluginhost.exe, wps.exe
- ✓ No drivers
- · Adds right-click menu entries
- · Opens ksoqing:// links
- · Files in Program Files \(1 MB\)
- · Files in ProgramData \(30 KB\)
- · Files in your user folder \(1.1 GB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 5 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"
- · Added store app "wpsappext1"
- · Added store app "wpsappext2"
- · Added store app "wpsappext3"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### WPS Office 12.2.0.23196: after uninstall, 28 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "WPS Office \(12.2.0.23196\)" is still there
- ✗ Service "WPS Office Cloud Service" is still there
- ✗ Scheduled task "WpsExternal\_runneradmin\_20261104141230" is still there
- ✗ Scheduled task "WpsExternal\_runneradmin\_startup" is still there
- ✗ Scheduled task "WpsUpdateTask\_runneradmin" is still there
- ✗ Scheduled task "WpsWakeWnsLogonTask" is still there
- ✗ Firewall rule "wpspdf.exe" is still there
- ✗ Firewall rule "et.exe" is still there
- ✗ Firewall rule "wpscloudsvr.exe" is still there
- ✗ Firewall rule "wpp.exe" is still there
- ✗ Firewall rule "promecefpluginhost.exe" is still there
- ✗ Firewall rule "wps.exe" is still there
- ✗ Right-click menu entry "kwpsshellext" is still there \(2 items\)
- ✗ 1 MB in Program Files \(x86\)\\Kingsoft
- ✗ Folder "Program Files \(x86\)\\Microsoft Office" is still there \(empty\)
- ✗ 30 KB in ProgramData\\kingsoft
- ✗ 1014 MB in AppData\\Local\\kingsoft
- ✗ 99 MB in AppData\\Roaming\\kingsoft
- ✗ Shortcut "Start menu\\WPS Office.lnk" is still there
- ✗ Shortcut "Start menu\\WPS Office\\WPS Office Tools\\WPS Office Configuration Tools.lnk" is still there
- ✗ Shortcut "Start menu\\WPS PDF.lnk" is still there
- ✗ Shortcut "Start menu\\WPS Sheets.lnk" is still there
- ✗ Shortcut "Start menu\\WPS Slides.lnk" is still there
- ✗ Shortcut "Start menu\\WPS Docs.lnk" is still there
- ✗ Shortcut "Desktop\\WPS Office.lnk" is still there
- ✗ Shortcut "Desktop\\WPS PDF.lnk" is still there
- ✗ Link handler "ksoqing" is still there

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

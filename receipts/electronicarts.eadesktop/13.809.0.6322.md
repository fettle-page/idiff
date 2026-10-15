# EA app 13.809.0.6322

## EA app 13.809.0.6322: installed

- ⚠ Adds 1 background service: EABackgroundService \(full system rights\)
- ⚠ Starts with Windows
- ⚠ Allows incoming connections for eadesktop.exe, ealocalhostsvc.exe, eabackgroundservice.exe, eagep.exe, eaconnect\_microsoft.exe
- · Adds outbound firewall rules for ealaunchhelper.exe
- ✓ No drivers
- · Files in Program Files \(605 MB\)
- · Files in ProgramData \(1 MB\)
- · Files in your user folder \(8 MB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · Also installed: Microsoft Visual C++ 2012 Redistributable \(x86\) - 11.0.61030, Microsoft Visual C++ 2013 Redistributable \(x86\) - 12.0.40664, Microsoft Visual C++ 2010 x86 Redistributable - 10.0.40219
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added folder "Program Files\\EA Games"
- · Added folder "AppData\\Local\\cache"
- · Added folder "AppData\\Local\\Origin"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### EA app 13.809.0.6322: after uninstall, 12 leftovers

- ⚠ The app is still listed in Installed programs, so the uninstall may not have run or finished; this list may include things it would have removed
- ✗ Program "EA app" is still there \(2 items\)
- ✗ Service "EABackgroundService" is still there
- ✗ 608 MB in Program Files\\Electronic Arts
- ✗ 2 MB in ProgramData\\EA Desktop
- ✗ Shortcut "Start menu \(all users\)\\EA\\App Recovery.lnk" is still there
- ✗ Shortcut "Start menu \(all users\)\\EA\\EA app Updater.lnk" is still there
- ✗ Shortcut "Start menu \(all users\)\\EA\\EA Error Reporter.lnk" is still there
- ✗ Shortcut "Start menu \(all users\)\\EA\\EA.lnk" is still there
- ✗ Shortcut "Public Desktop\\EA.lnk" is still there
- ✗ 795 KB in AppData\\Local\\EADesktop
- ✗ 8 MB in AppData\\Local\\Electronic Arts
- · The uninstaller added firewall rule "EA app \(EADesktop\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EABackgroundService\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EALocalHostSvc\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EAConnect\_microsoft\)"
- · The uninstaller added firewall rule "EA app \(EAConnect\_microsoft\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EAGEP\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EADesktop\)"
- · The uninstaller added firewall rule "EA app \(EAGEP\)"
- · The uninstaller added firewall rule "EA app \(EALaunchHelper\) \(Outbound\)"
- · The uninstaller added firewall rule "EA app \(EALocalHostSvc\)"
- · The uninstaller added firewall rule "EA app \(EABackgroundService\)"
- · Left installed: Microsoft Visual C++ 2012 Redistributable \(x86\) - 11.0.61030, Microsoft Visual C++ 2013 x86 Minimum Runtime - 12.0.40664, Microsoft Visual C++ 2013 Redistributable \(x86\) - 12.0.40664, Microsoft Visual C++ 2012 x86 Additional Runtime - 11.0.61030, Microsoft Visual C++ 2012 x86 Minimum Runtime - 11.0.61030, Microsoft Visual C++ 2013 x86 Additional Runtime - 12.0.40664, Microsoft Visual C++ 2010 x86 Redistributable - 10.0.40219. They came with the app, and other apps may use them.

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

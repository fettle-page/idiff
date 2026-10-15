# HandBrake 1.11.2

## HandBrake 1.11.2: installed

- ✓ No services, drivers or firewall rules
- · Files in Program Files \(106 MB\)
- · Adds a Start menu entry
- · Also installed: Microsoft Windows Desktop Runtime 10.0.13 \(x64\)
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Removed program "Microsoft .NET Host - 10.0.12 \(x64\)"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

The uninstall didn't finish on the test machine, so these may include things it would have removed.

### HandBrake 1.11.2: after uninstall, 4 leftovers

- ⚠ The uninstall was still running after 5 minutes, so this list may include things it would have removed
- ✗ Program "HandBrake 1.11.2" is still there
- ✗ 106 MB in Program Files\\HandBrake
- ✗ Shortcut "Start menu \(all users\)\\HandBrake\\HandBrake.lnk" is still there
- ✗ Shortcut "Start menu \(all users\)\\HandBrake\\Uninstall.lnk" is still there
- · Left installed: Microsoft .NET Runtime - 10.0.13 \(x64\), Microsoft Windows Desktop Runtime - 10.0.13 \(x64\), Microsoft .NET Host - 10.0.13 \(x64\), Microsoft .NET Host FX Resolver - 10.0.13 \(x64\), Microsoft Windows Desktop Runtime 10.0.13 \(x64\). They came with the app, and other apps may use them.

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

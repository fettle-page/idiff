# Adobe Acrobat Reader 26.002.22031

## Adobe Acrobat Reader 26.002.22031: installed

- ⚠ Adds 1 background service: Adobe Acrobat Update Service \(starts with Windows\) \(full system rights\)
- ⚠ 1 scheduled task: runs adobearm.exe at sign-in
- ✓ No drivers or firewall rules
- · Adds right-click menu entries
- · Files in Program Files \(1.2 GB\)
- · Adds a desktop shortcut
- · Adds a Start menu entry
- · 5 Windows background changes hidden \(listed below\)

### Hidden: Windows background changes

- · Changed service "PcaSvc"
- · Changed scheduled task "PrintJobCleanupTask"
- · Added folder "AppData\\Local\\Mozilla"
- · Added folder "AppData\\Roaming\\Mozilla"
- · Added store app "Windows Package Manager Source \(winget\) V2"

## After uninstall

### Adobe Acrobat Reader 26.002.22031: after uninstall, no leftovers

- ✓ Nothing left behind

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261007.264.1) by idiff 1.0.0 on 2026-10-15, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/38780384255). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

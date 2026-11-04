# PowerToys 0.102.3001.0

## PowerToys 0.102.3001.0: installed

- ⚠ Allows incoming connections for powertoys.mousewithoutborders.exe
- · Adds outbound firewall rules for PowerToys.SparseApp
- ✓ No services or drivers
- · Adds right-click menu entries
- · Opens powertoys:// links
- · Adds to PATH: C:\\Users\\runneradmin\\AppData\\Local\\PowerToys\\bin\\, C:\\Users\\runneradmin\\AppData\\Local\\PowerToys\\DSCModules\\
- ✓ Files only in your user folder \(1013 MB\)
- · Adds a Start menu entry
- · 6 Windows background changes hidden \(listed below\)

### Also changed while installing \(couldn't tell if it was this app\)

- · Added store app "Command Palette"
- · Added store app "Microsoft Visual C++ 2015 UWP Runtime Package"
- · Added link handler "x-cmdpal"

### Hidden: Windows background changes

- · Changed service "BITS"
- · Changed service "PcaSvc"
- · Added folder "AppData\\Local\\Package Cache"
- · Changed store app "Windows Security"
- · Added store app "Windows Package Manager Source \(winget\) V2"
- · Added link handler "windowsdefender"

## After uninstall

### PowerToys 0.102.3001.0: after uninstall, no leftovers

- ✓ Nothing left behind

## Changed since the previous version

- \+ Opens powertoys:// links
- \+ Adds to PATH: C:\\Users\\runneradmin\\AppData\\Local\\PowerToys\\bin\\, C:\\Users\\runneradmin\\AppData\\Local\\PowerToys\\DSCModules\\
- \+ Files only in your user folder \(1013 MB\)
- \- 1 scheduled task: runs powertoys.exe at sign-in
- \- Adds to PATH: C:\\Users\\runneradmin\\AppData\\Local\\PowerToys\\DSCModules\\
- \- Files only in your user folder \(985 MB\)

## How this was made

Installed silently with winget on a fresh GitHub Actions runner (win25-vs2026 20261030.294.1) by idiff 1.1.0 on 2026-11-04, then uninstalled silently. The app itself was never started. [Workflow run](https://github.com/fettle-page/idiff/actions/runs/41841853441). idiff only reads; see the README for how it decides what belongs to the app.

[How idiff decides what's the app's](../../README.md#how-it-decides-whats-the-apps)

## Older versions

- [0.101.2362.0](0.101.2362.0.md) (2026-10-15)

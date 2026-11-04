# Changelog

## 1.1.0 (unreleased)

- New: link handlers. The receipt says which links an app now opens (`· Opens discord:// links`), and warns when it takes over links another program opened before.
- New: system hooks. The receipt warns about an Image File Execution Options debugger, a change to what Windows runs at sign-in (Winlogon `Shell` and `Userinit`), and DLLs added to `AppInit_DLLs`.
- New: an ARM64 download, `idiff-<version>-arm64.zip`, built and tested on Windows 11 on ARM.
- `idiff leftovers` counts exit code 19 from Chromium's `setup.exe --uninstall` (Brave, Vivaldi, Opera) as success, since that's the code it uses for "uninstall successful".
- A session saved by 1.0 still works: compared with a newer snapshot, the two new sources say they couldn't be checked instead of listing everything as new.
- Receipts Library: 98 apps (was 49), a weekly run, and each app's own silent uninstall switches where `winget uninstall --silent` waits for a dialog.

## 1.0.0 (2026-10-15)

First release.

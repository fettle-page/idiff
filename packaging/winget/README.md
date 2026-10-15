# winget

idiff is submitted to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) as `Fettle.InstallDiff`. Once a version is merged there, anyone can install it with:

```
winget install Fettle.InstallDiff
```

That unzips the release's `idiff-<version>-x64.zip` and puts `idiff` on PATH (a portable install: no installer, nothing outside winget's own folders).

## What's here

`Fettle.InstallDiff/<version>/` holds the three manifest files for each version, exactly as they go into winget-pkgs under `manifests/f/Fettle/InstallDiff/<version>/`:

- `Fettle.InstallDiff.yaml`: the version file
- `Fettle.InstallDiff.installer.yaml`: the release zip's URL and SHA-256, and `idiff.exe` with its `idiff` alias
- `Fettle.InstallDiff.locale.en-US.yaml`: name, publisher, description, license

The package identifier `Fettle.InstallDiff` is permanent: users install and upgrade by it.

## Adding a version

1. Publish the release (tag `v<version>`; the release workflow builds the zip and `SHA256SUMS.txt`).
2. Copy the previous version's folder to `Fettle.InstallDiff/<version>/`. In all three files, change `PackageVersion`. In the installer file, change `InstallerUrl`, `InstallerSha256` (from `SHA256SUMS.txt`, in upper case) and `ReleaseDate`. In the locale file, change `ReleaseNotesUrl`.
3. Commit, push, and check it on a GitHub runner (never on your own PC):

   ```
   gh workflow run winget.yml -f version=<version>
   ```

   The workflow validates the manifest, installs idiff from it on a fresh windows-2025 runner, runs `idiff --version` through the alias, and uninstalls it.
4. Submit the folder to winget-pkgs as a pull request from your fork, under `manifests/f/Fettle/InstallDiff/<version>/`, titled `New version: Fettle.InstallDiff version <version>` (`New package: …` for the first one). Your first PR there asks you to accept Microsoft's CLA with a comment.
5. winget-pkgs runs its own validation (an install on a test machine and a malware scan), then a moderator reviews it. That usually takes a few days. Answer any questions on the PR.

## Notes

- The downloads aren't signed yet. winget accepts unsigned packages, but its scan can be slower to clear a new unsigned file.
- OpenImageIO also has a command called `idiff`. With both on PATH, whichever comes first wins; the README says to run this one by its full path in that case.

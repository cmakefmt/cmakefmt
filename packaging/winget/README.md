# winget packaging

This directory contains the winget manifest for
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).

## Submitting to winget-pkgs

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
2. Create the directory `manifests/c/cmakefmt/cmakefmt/0.2.0/`
3. Copy the three YAML files from this directory into it
4. Open a PR — the winget bot will validate the manifest automatically

## Updating for a new release

- Bump `PackageVersion` in all three YAML files
- Update `InstallerUrl` and `InstallerSha256` in the installer manifest
- Update `ReleaseNotesUrl` in the locale manifest

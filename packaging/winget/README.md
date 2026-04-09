# winget packaging

This directory contains the winget manifest for
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs).

## Before submitting

Read the [winget-pkgs contributing guide](https://github.com/microsoft/winget-pkgs/blob/master/CONTRIBUTING.md)
before opening a PR. Key points: use the PR template checklist, sign the
[CLA](https://cla.opensource.microsoft.com/microsoft/winget-pkgs), use the
latest manifest schema (currently 1.12.0), keep the `yaml-language-server`
schema header comments at the top of each manifest (the validator requires
them), and validate locally with `winget validate --manifest <path>` on
Windows before submitting.

## Submitting to winget-pkgs

1. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)
2. Create the directory `manifests/c/cmakefmt/cmakefmt/0.4.0/`
3. Copy the three YAML files from this directory into it
4. Open a PR — the winget bot will validate the manifest automatically

## Updating for a new release

- Bump `PackageVersion` in all three YAML files
- Update `InstallerUrl` and `InstallerSha256` in the installer manifest
- Update `ReleaseNotesUrl` in the locale manifest

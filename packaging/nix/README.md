# Nix packaging

`packaging/nix/cmakefmt.nix` is the nixpkgs derivation for
[NixOS/nixpkgs](https://github.com/NixOS/nixpkgs).

The derivation uses pre-built musl binaries for Linux (static, no patching
needed) and universal binaries for macOS. The aarch64-linux build uses the
glibc binary since a musl build is not available for that target.

## Submitting to nixpkgs

1. Fork [NixOS/nixpkgs](https://github.com/NixOS/nixpkgs)
2. Copy `cmakefmt.nix` to `pkgs/by-name/cm/cmakefmt/package.nix`
3. Add an entry to `pkgs/top-level/all-packages.nix`:
   ```nix
   cmakefmt = callPackage ../by-name/cm/cmakefmt/package.nix { };
   ```
4. Build and test: `nix build -f . cmakefmt`
5. Open a PR following the nixpkgs contribution guidelines

## Updating for a new release

- Bump `version`
- Update the `hash` values for each platform (use `nix-prefetch-url` or
  `nix store prefetch-file` to compute the SRI hash)

```bash
nix store prefetch-file \
  https://github.com/cmakefmt/cmakefmt/releases/download/vVERSION/cmakefmt-VERSION-x86_64-unknown-linux-musl.tar.gz
```

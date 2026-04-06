# Homebrew Packaging

This directory contains the Homebrew formula template used for `cmakefmt`
release packaging.

## Files

- `cmakefmt.rb.in`
  - formula template with `@VERSION@` and `@SHA256@` placeholders

## How It Is Used

During the tagged release workflow:

1. `release.yml` downloads the GitHub source tarball for the release tag
2. computes its SHA-256
3. runs `scripts/render-homebrew-formula.sh`
4. publishes the rendered `cmakefmt.rb` file as a GitHub Release artifact
5. updates the `cmakefmt/homebrew-cmakefmt` tap repository when
   `HOMEBREW_TAP_TOKEN` is configured

That gives us a release-matched Homebrew formula without hard-coding versioned
formula files in the main repository.

## Manual Rendering

You can render the formula locally with:

```bash
bash scripts/render-homebrew-formula.sh <version> <source-tarball-sha256>
```

Example:

```bash
bash scripts/render-homebrew-formula.sh 0.1.0 <sha256>
```

## Notes

- the formula uses the GitHub source tarball for the tagged release
- the rendered formula is also pushed to the `cmakefmt/homebrew-cmakefmt` tap
  when release automation has access to `HOMEBREW_TAP_TOKEN`
- `_cmakefmt` is the conventional zsh completion filename; do not rename it to
  add a `.zsh` suffix

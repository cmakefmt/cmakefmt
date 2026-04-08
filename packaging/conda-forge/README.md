# conda-forge packaging

This directory contains the conda-forge recipe for
[conda-forge/staged-recipes](https://github.com/conda-forge/staged-recipes).

The recipe builds cmakefmt from source using `cargo install`.

## Before submitting

Read the [conda-forge contributing guide](https://conda-forge.org/docs/maintainer/adding_pkgs/)
before opening a PR. Key points: the recipe must build on all default
platforms (linux-64, osx-64, win-64), and `conda smithy lint` must pass.

## Submitting to conda-forge

1. Fork [conda-forge/staged-recipes](https://github.com/conda-forge/staged-recipes)
2. Create `recipes/cmakefmt/` and copy the recipe files into it
3. Open a PR — CI will build and test on all platforms

Once the PR is merged, conda-forge automatically creates a
`cmakefmt-feedstock` repository. After that, this directory can be removed
— the canonical recipe lives in the feedstock.

## Updating for a new release

- Bump `version` in `meta.yaml`
- Update the `sha256` (from the GitHub source tarball, not the release binary)
- The feedstock's autotick bot usually handles this automatically

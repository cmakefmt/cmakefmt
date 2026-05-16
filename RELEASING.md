<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Releasing `cmakefmt`

This is the maintainer-facing release procedure. It captures the
information a successor would need to ship a release without prior
context. The longer working reference (architecture decisions,
historical rationale, future plans) lives in the
`cmakefmt/strategy` companion repo; this file holds only the parts
that are load-bearing for actually cutting a release.

## At a glance

| Step | Where | Who triggers |
|---|---|---|
| Bump version + tag | `prepare-release.yml` workflow | Manually, via `workflow_dispatch` |
| Build artefacts, publish to GitHub Releases / crates.io / PyPI / GHCR | `release.yml` workflow | Auto, on tag push |
| Submit to winget-pkgs | `publish-winget.yml` workflow | Auto, on GitHub Release publish |
| Bump Homebrew tap formula | `release.yml` job `publish-homebrew` | Auto, on tag push |
| Bump VS Code extension | `release.yml` job `publish-vscode-extension` | Auto, on tag push |

## Step-by-step

1. Confirm `## Unreleased` in `CHANGELOG.md` has entries for everything
   that should be in the release. Anything not in `Unreleased` will
   not appear in the release notes.
2. Pick the version bump (patch / minor / major). See "Choosing
   the bump" below.
3. Trigger **Prepare Release** workflow:
   - GitHub UI → Actions → "Prepare Release" → Run workflow.
   - `version`: the target version, no leading `v` (e.g. `1.5.0`).
   - `branch`: usually `main`.
4. The workflow validates the version shape, runs `bump-my-version`,
   refreshes `Cargo.lock`, smoke-tests the bumped tree
   (`cargo test --locked --workspace`), stamps the changelog,
   publishes the versioned JSON schema under
   `docs/public/schemas/v<version>/`, commits as `Release <version>`,
   tags `v<version>`, and pushes both. A compile or test failure on
   the bumped tree blocks the tag.
5. The tag push triggers **Release** (`release.yml`), which builds
   release artefacts, uploads them to a GitHub Release, publishes
   to crates.io / PyPI / GHCR / the Homebrew tap / the VS Code
   marketplace, and generates the SBOM.
6. The GitHub Release publication triggers **Publish to winget**
   (`publish-winget.yml`), which submits a winget-pkgs manifest
   via `winget-releaser`.

If any step fails, the tag stays but the published artefacts will
be incomplete. crates.io and PyPI refuse to re-publish a version
once it has been yanked, so recovering from a partial publish
usually means bumping to the next patch and re-cutting.

## Choosing the bump

- **Patch (`1.4.X`)** — bug fixes that change formatter output only
  for inputs that hit the bug, plus internal hygiene with no
  user-visible effect. The bar is "would a user be surprised?"
  Silent-correctness fixes (where the previous output was
  semantically wrong) belong here too.
- **Minor (`1.X.0`)** — new config options, new spec affordances,
  CLI additions, anything that broadens what cmakefmt can do
  without breaking existing usage. Output may legitimately change
  for users who opt into the new feature.
- **Major (`X.0.0`)** — breaking changes to the CLI surface, the
  config schema, or the published library API. Reserved for
  deliberate deprecations after one or more minor releases of
  warning.

When in doubt, lean **patch**. The release cadence is cheap; one
bad bump call is more confusing than two patches in a week.

## Required secrets

These secrets must be configured on the `cmakefmt/cmakefmt` repo
for the release pipeline to work end-to-end. Rotate annually or
on personnel change. Missing secrets cause specific jobs to fail
with `*: secret not configured` errors.

| Secret | Used by | Scope |
|---|---|---|
| `CRATES_IO_API_TOKEN` | `release.yml` → `publish-crate` | crates.io API publish |
| `RELEASE_WORKFLOW_TOKEN` | `prepare-release.yml` | Push commit+tag from the bot |
| `HOMEBREW_TAP_TOKEN` | `release.yml` → `publish-homebrew` | Push to `cmakefmt/homebrew-cmakefmt` |
| `VSCODE_CMAKEFMT_TOKEN` | `release.yml` → `publish-vscode-extension` | Push to `cmakefmt/vscode-cmakefmt` + Marketplace |
| `WINGET_PUBLISH_TOKEN` | `publish-winget.yml` | Push manifest PR to the winget-pkgs fork |
| `GITHUB_TOKEN` | Built-in | GHCR push, GitHub Release upload |

PyPI publishing uses **OIDC trusted publishing**, not a token —
configured on PyPI's side against the `publish-pypi` GitHub
Environment. Re-binding requires PyPI account access.

## Required environments

GitHub Environments provide deployment gates and secret scoping.
Both should be configured with no protection rules unless you
want manual approval before each publish.

- `publish-crate` — binds to `CRATES_IO_API_TOKEN`. The
  `publish-crate` job in `release.yml` references this
  environment.
- `publish-pypi` — bound to PyPI's OIDC trusted publishing config.
  No secret stored; the binding itself is the credential.

## Sibling repositories

The release pipeline writes to four additional repos under the
`cmakefmt` organisation. A successor needs write access to all
of them (and to `cmakefmt/cmakefmt` itself).

| Repo | What lives there | Updated by |
|---|---|---|
| `cmakefmt/cmakefmt` | This codebase | `prepare-release.yml` + manual edits |
| `cmakefmt/cmakefmt-action` | The GitHub Action wrapper | Versioned independently; floating major tag (`v2`) auto-tracked |
| `cmakefmt/homebrew-cmakefmt` | Homebrew tap formula | `release.yml` job `publish-homebrew` |
| `cmakefmt/vscode-cmakefmt` | VS Code extension | `release.yml` job `publish-vscode-extension` |
| `cmakefmt-org/winget-pkgs` (fork of microsoft/winget-pkgs) | winget manifest PRs | `publish-winget.yml` via `winget-releaser` |

## Local pre-release checks

Run these locally before triggering Prepare Release if you want
extra confidence:

```bash
cargo test --locked --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo +1.85 check --locked --all-features --all-targets  # MSRV gate
python3 scripts/fetch-real-world-corpus.py && cargo test --test idempotency
```

The CI matrix runs the equivalent on every PR, but running locally
shortens the feedback loop when the release is time-sensitive.

## Recovering from a failed release

- **Compile error after bump** — the new `cargo test --locked` smoke
  step in `prepare-release.yml` should catch this before the tag is
  pushed. If somehow it slipped through: delete the tag locally and
  remotely (`git tag -d v<version>` and `git push origin
  :refs/tags/v<version>`), revert the `Release <version>` commit on
  `main`, and bump to the next patch.
- **crates.io or PyPI publish failed mid-flight** — both refuse
  re-uploads. Bump to the next patch and re-cut; record the
  skipped version in the changelog as a `<version>` placeholder
  with "(unreleased — partial publish)" note.
- **Homebrew / VS Code / winget publish failed** — these can be
  re-run manually via `workflow_dispatch` without retagging. The
  GitHub Release itself stays valid.

## See also

- `strategy/docs/RELEASING.md` — longer working reference with
  architectural rationale and historical context.
- `.bumpversion.toml` — the authoritative list of files where the
  version string is rewritten.
- `CHANGELOG.md` — release-notes source of truth.

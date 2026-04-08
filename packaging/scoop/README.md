# Scoop packaging

`packaging/scoop/cmakefmt.json` is the Scoop manifest for the
[ScoopInstaller/Extras](https://github.com/ScoopInstaller/Extras) bucket.

> **Note:** ScoopInstaller/Extras requires the project to have 50–100+ GitHub
> stars before a manifest is accepted. Revisit this once the repo reaches that
> threshold.

## Before submitting

Read the [Scoop bucket contribution guidelines](https://github.com/ScoopInstaller/.github/blob/main/.github/CONTRIBUTING.md#for-scoop-buckets)
before opening a PR. Key points: PR title must be `cmakefmt: Add version {version}`,
indentation must be 4 spaces, and add a `/verify` comment after the PR is raised.

## Submitting to Extras

1. Fork [ScoopInstaller/Extras](https://github.com/ScoopInstaller/Extras)
2. Copy `cmakefmt.json` into `bucket/cmakefmt.json`
3. Open a PR following the Extras contribution guidelines

## Updating for a new release

Update the `version` field and the `hash` under `architecture.64bit`. The
`autoupdate` block handles future updates automatically once the manifest is
accepted into the bucket.

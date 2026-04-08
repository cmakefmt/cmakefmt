# Scoop packaging

`packaging/scoop/cmakefmt.json` is the Scoop manifest for the
[ScoopInstaller/Extras](https://github.com/ScoopInstaller/Extras) bucket.

## Submitting to Extras

1. Fork [ScoopInstaller/Extras](https://github.com/ScoopInstaller/Extras)
2. Copy `cmakefmt.json` into `bucket/cmakefmt.json`
3. Open a PR following the Extras contribution guidelines

## Updating for a new release

Update the `version` field and the `hash` under `architecture.64bit`. The
`autoupdate` block handles future updates automatically once the manifest is
accepted into the bucket.

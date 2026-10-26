#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

required_files=(
  ".mise.toml"
  "scripts/stamp-changelog.py"
  "scripts/sync-changelog.py"
  "README.md"
  "SECURITY.md"
  "CONTRIBUTING.md"
  "CHANGELOG.md"
  "docs/README.md"
  "src/README.md"
  "tests/README.md"
  "tests/fixtures/README.md"
  "tests/snapshots.rs"
  "benches/README.md"
  "docs/astro.config.mjs"
  "docs/package-lock.json"
  "docs/package.json"
  "docs/tsconfig.json"
  "docs/src/content.config.ts"
  "docs/public/robots.txt"
  "docs/src/content/docs/index.mdx"
  "docs/src/content/docs/getting-started.md"
  "docs/src/content/docs/playground.mdx"
  "docs/src/content/docs/installation.md"
  "docs/src/content/docs/coverage.md"
  "docs/src/content/docs/release.md"
  "docs/src/content/docs/cli.md"
  "docs/src/content/docs/config.md"
  "docs/src/content/docs/behavior.md"
  "docs/src/content/docs/migration.md"
  "docs/src/content/docs/performance.mdx"
  "docs/src/content/docs/troubleshooting.md"
  "docs/src/content/docs/api.md"
  "docs/src/content/docs/architecture.md"
  "docs/src/content/docs/contributing.md"
  "docs/src/content/docs/changelog.md"
  "docs/src/content/docs/editors.md"
  "docs/src/content/docs/ci.md"
  "docs/src/content/docs/comparison.md"
  "docs/src/content/docs/badge.md"
  "docs/src/content/docs/users.md"
  "docs/src/content/docs/stability.md"
)

for file in "${required_files[@]}"; do
  if [[ ! -f "$file" ]]; then
    echo "missing required docs file: $file" >&2
    exit 1
  fi
done

grep -q "Documentation" README.md || {
  echo "README.md is missing a Documentation section" >&2
  exit 1
}

grep -q "## Unreleased" CHANGELOG.md || {
  echo "CHANGELOG.md is missing an Unreleased section" >&2
  exit 1
}

grep -q "https://cmakefmt.dev" README.md || {
  echo "README.md does not link to the docs landing page" >&2
  exit 1
}

# The release workflow owns changelog generation. Check published entries
# without changing files; working Unreleased notes may differ until release.
python3 - <<'PY'
from pathlib import Path
import re

def released_entries(path):
    text = Path(path).read_text()
    first_release = re.search(r"^## \d+\.\d+\.\d+\b", text, re.MULTILINE)
    if first_release is None:
        raise SystemExit(f"no released changelog entries in {path}")
    return text[first_release.start():].split("\n## Release Process", 1)[0].strip()

if released_entries("CHANGELOG.md") != released_entries("docs/src/content/docs/changelog.md"):
    raise SystemExit("published docs changelog entries differ from CHANGELOG.md")
PY

if command -v npm >/dev/null 2>&1 && [[ -d docs/node_modules ]]; then
  # Run the build and link check directly: npm's prebuild hook synchronizes
  # the changelog, which is intentionally left to the release workflow.
  (cd docs && npm exec -- astro build >/dev/null)
  python3 scripts/check-doc-links.py docs/dist
fi

echo "docs checks passed"

#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

required_files=(
  "README.md"
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
  "docs/src/content/docs/install.md"
  "docs/src/content/docs/coverage.md"
  "docs/src/content/docs/release.md"
  "docs/src/content/docs/cli.md"
  "docs/src/content/docs/config.md"
  "docs/src/content/docs/behavior.md"
  "docs/src/content/docs/migration.md"
  "docs/src/content/docs/performance.md"
  "docs/src/content/docs/troubleshooting.md"
  "docs/src/content/docs/api.md"
  "docs/src/content/docs/architecture.md"
  "docs/src/content/docs/changelog.md"
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

if command -v npm >/dev/null 2>&1 && [[ -f docs/package-lock.json ]]; then
  (cd docs && npm run build >/dev/null)
fi

echo "docs checks passed"

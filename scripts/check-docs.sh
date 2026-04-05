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
  "docs/book.toml"
  "docs/src/SUMMARY.md"
  "docs/src/README.md"
  "docs/src/install.md"
  "docs/src/cli.md"
  "docs/src/config.md"
  "docs/src/behavior.md"
  "docs/src/migration.md"
  "docs/src/api.md"
  "docs/src/architecture.md"
  "docs/src/changelog.md"
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

if command -v mdbook >/dev/null 2>&1; then
  mdbook build docs >/dev/null
fi

echo "docs checks passed"

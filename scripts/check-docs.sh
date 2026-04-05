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
  "site/README.md"
  "site/book.toml"
  "site/src/SUMMARY.md"
  "site/src/README.md"
  "site/src/install.md"
  "site/src/cli.md"
  "site/src/config.md"
  "site/src/behavior.md"
  "site/src/migration.md"
  "site/src/api.md"
  "site/src/architecture.md"
  "site/src/changelog.md"
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

grep -q "site/src/README.md" README.md || {
  echo "README.md does not link to the docs landing page" >&2
  exit 1
}

if command -v mdbook >/dev/null 2>&1; then
  mdbook build site >/dev/null
fi

echo "docs checks passed"

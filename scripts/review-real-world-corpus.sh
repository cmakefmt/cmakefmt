#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
corpus_dir="${CMAKEFMT_REAL_WORLD_DIR:-$repo_root/target/real-world-corpus}"
review_dir="${CMAKEFMT_REAL_WORLD_REVIEW_DIR:-$repo_root/target/real-world-review}"
fmt_bin="${CMAKEFMT_BIN:-$repo_root/target/release/cmakefmt}"

python3 "$repo_root/scripts/fetch-real-world-corpus.py" --dest "$corpus_dir"

if [[ ! -x "$fmt_bin" ]]; then
  cargo build --release --manifest-path "$repo_root/Cargo.toml" >/dev/null
fi

rm -rf "$review_dir"
mkdir -p "$review_dir"

while IFS='|' read -r name relative_path; do
  if [[ -z "$name" ]]; then
    continue
  fi

  input="$corpus_dir/$relative_path"
  out_dir="$review_dir/$name"
  mkdir -p "$out_dir"

  cp "$input" "$out_dir/original.cmake"
  "$fmt_bin" "$input" >"$out_dir/formatted.cmake"
  if diff -u "$out_dir/original.cmake" "$out_dir/formatted.cmake" >"$out_dir/diff.patch"; then
    rm "$out_dir/diff.patch"
    printf '%s unchanged\n' "$name"
  else
    printf '%s changed\n' "$name"
  fi
done < <(
  python3 - <<'PY'
import tomllib
from pathlib import Path

manifest = tomllib.loads(Path("tests/fixtures/real_world/manifest.toml").read_text(encoding="utf-8"))
for fixture in manifest["fixture"]:
    print(f"{fixture['name']}|{fixture['relative_path']}")
PY
)

printf 'review artefacts written to %s\n' "$review_dir"

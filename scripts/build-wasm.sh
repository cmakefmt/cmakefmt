#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# Build the cmakefmt WASM module for the browser playground.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

OUT_DIR="docs/public/wasm"

# Ensure wasm-pack uses the rustup toolchain (not Homebrew's rustc).
RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
if [[ -d "$RUSTUP_HOME" ]]; then
  RUSTUP_TOOLCHAIN="$(rustup show active-toolchain | cut -d' ' -f1)"
  SYSROOT="$(rustup run "$RUSTUP_TOOLCHAIN" rustc --print sysroot)"
  export PATH="$SYSROOT/bin:$PATH"
fi

wasm-pack build \
  --target web \
  --out-dir "$OUT_DIR" \
  --out-name cmakefmt \
  --no-default-features

# Remove files not needed for the playground.
rm -f "$OUT_DIR/.gitignore" "$OUT_DIR/package.json" "$OUT_DIR/README.md"

echo "WASM build complete: $(du -h "$OUT_DIR/cmakefmt_bg.wasm" | cut -f1)"

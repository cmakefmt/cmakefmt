#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

output_dir="${1:-dist}"
mkdir -p "${output_dir}"

cargo build --release --locked --target wasm32-unknown-unknown \
  --no-default-features --features dprint-plugin

cp target/wasm32-unknown-unknown/release/cmakefmt.wasm \
  "${output_dir}/dprint-cmakefmt.wasm"
cp packaging/dprint-cmakefmt.schema.json \
  "${output_dir}/dprint-cmakefmt.schema.json"

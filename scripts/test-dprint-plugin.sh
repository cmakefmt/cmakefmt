#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
plugin_path="${1:-${repo_root}/dist/dprint-cmakefmt.wasm}"
plugin_path="$(cd "$(dirname "${plugin_path}")" && pwd)/$(basename "${plugin_path}")"
test_dir="$(mktemp -d)"
trap 'rm -r "${test_dir}"' EXIT

# Keep host configuration and fixtures isolated from the caller's project.
cd "${test_dir}"
cat > dprint.json <<EOF
{
  "plugins": ["${plugin_path}"],
  "includes": ["**/*"],
  "incremental": false,
  "indentWidth": 4,
  "newLineKind": "crlf",
  "cmakefmt": {"preserveArgumentComments": true}
}
EOF

for file in CMakeLists.txt CMakeLists.txt.in example.cmake; do
  printf 'CUSTOM_COMMAND(\n first\n # keep this comment\n second)\n' > "${file}"
done
printf 'CUSTOM_COMMAND(first second)\n' > untouched.txt
printf 'custom_command(\r\n    first\r\n    # keep this comment\r\n    second)\r\n' > expected.txt

"${DPRINT:-dprint}" fmt --config dprint.json
for file in CMakeLists.txt CMakeLists.txt.in example.cmake; do
  diff -u expected.txt "${file}"
done
printf 'CUSTOM_COMMAND(first second)\n' | cmp - untouched.txt
"${DPRINT:-dprint}" check --config dprint.json
echo "dprint plugin smoke test passed"

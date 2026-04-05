#!/usr/bin/env bash

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <version> <source-tarball-sha256>" >&2
  exit 1
fi

version="$1"
sha256="$2"

template="packaging/homebrew/cmakefmt.rb.in"

if [[ ! -f "$template" ]]; then
  echo "missing template: $template" >&2
  exit 1
fi

sed -e "s/@VERSION@/${version}/g" -e "s/@SHA256@/${sha256}/g" "$template"

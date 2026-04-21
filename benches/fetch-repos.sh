#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Fetch benchmark repositories using sparse checkout (CMake files only).
# Usage: ./benches/fetch-repos.sh

set -euo pipefail

REPOS_DIR="$(cd "$(dirname "$0")" && pwd)/repos"
mkdir -p "$REPOS_DIR"

# repo_url  directory_name
REPOS=(
  "https://github.com/google/googletest.git         googletest"
  "https://github.com/catchorg/Catch2.git           catch2"
  "https://github.com/fmtlib/fmt.git                fmt"
  "https://github.com/gabime/spdlog.git             spdlog"
  "https://github.com/nlohmann/json.git             nlohmann_json"
  "https://github.com/opencv/opencv.git             opencv"
  "https://github.com/protocolbuffers/protobuf.git  protobuf"
  "https://github.com/llvm/llvm-project.git         llvm"
  "https://github.com/KhronosGroup/Vulkan-Hpp.git   vulkan_hpp"
  "https://github.com/Kitware/CMake.git             cmake"
  "https://github.com/grpc/grpc.git                 grpc"
  "https://github.com/blender/blender.git           blender"
  "https://github.com/bulletphysics/bullet3.git     bullet3"
  "https://github.com/oomph-lib/oomph-lib.git       oomph-lib"
)

clone_sparse() {
  local url="$1" name="$2"
  local dest="$REPOS_DIR/$name"

  if [[ -d "$dest" ]]; then
    echo "  skip: $name (already exists)"
    return
  fi

  echo "  clone: $name"
  git clone --depth 1 --filter=blob:none --sparse "$url" "$dest" 2>/dev/null
  cd "$dest"
  git sparse-checkout init --cone
  git sparse-checkout set --no-cone '/**/CMakeLists.txt' '/**/*.cmake'
  cd - > /dev/null
}

echo "Fetching benchmark repositories into $REPOS_DIR ..."
echo ""

for entry in "${REPOS[@]}"; do
  read -r url name <<< "$entry"
  clone_sparse "$url" "$name"
done

echo ""
echo "Done. Repos:"
for entry in "${REPOS[@]}"; do
  read -r _ name <<< "$entry"
  dest="$REPOS_DIR/$name"
  if [[ -d "$dest" ]]; then
    count=$(find "$dest" \( -name 'CMakeLists.txt' -o -name '*.cmake' \) -not -path '*/.git/*' | wc -l | tr -d ' ')
    echo "  $name: $count CMake files"
  fi
done

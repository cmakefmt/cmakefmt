#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Quick cmakefmt-only benchmarks (no cmake-format comparison).
# Measures scaling across file sizes and repository sizes.
#
# Usage: ./benches/run-cmakefmt-only.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CMAKEFMT="$REPO_ROOT/target/release/cmakefmt"
RESULTS_DIR="$SCRIPT_DIR/results/cmakefmt-only"
OOMPH_DIR="/Users/PuneetMatharu/Dropbox/programming/oomph-lib/oomph-lib-repos/forked-oomph-lib"

if [[ ! -x "$CMAKEFMT" ]]; then
  echo "error: release binary not found. Run: cargo build --release"
  exit 1
fi

rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR/per-file" "$RESULTS_DIR/whole-repo"

echo "cmakefmt: $($CMAKEFMT --version 2>&1 | head -1)"
echo "Binary:   $(ls -lh "$CMAKEFMT" | awk '{print $5}')"
echo ""

# ── Per-file: scaling with line count ───────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Per-File: scaling with line count"
echo "═══════════════════════════════════════════════════════════════"
echo ""

PF_FILES=(
  "$REPO_ROOT/target/real-world-corpus/opencv_flann/CMakeLists.txt:opencv_flann:100:500"
  "$REPO_ROOT/target/real-world-corpus/googletest/CMakeLists.txt:googletest:10:50"
  "$REPO_ROOT/target/real-world-corpus/llvm_tablegen/CMakeLists.txt:llvm_tablegen:10:50"
  "$REPO_ROOT/target/real-world-corpus/abseil/CMakeLists.txt:abseil:10:50"
  "$REPO_ROOT/target/real-world-corpus/spdlog/CMakeLists.txt:spdlog:10:50"
  "$REPO_ROOT/target/real-world-corpus/mariadb_server/CMakeLists.txt:mariadb_server:10:50"
  "$REPO_ROOT/target/real-world-corpus/xnnpack/CMakeLists.txt:xnnpack:10:50"
  "$REPO_ROOT/target/real-world-corpus/opencv_root/CMakeLists.txt:opencv_root:5:20"
  "$REPO_ROOT/target/real-world-corpus/blender_root/CMakeLists.txt:blender_root:5:20"
  "$REPO_ROOT/target/real-world-corpus/llvm_libc_math/CMakeLists.txt:llvm_libc_math:5:20"
  "$REPO_ROOT/target/real-world-corpus/grpc_root/CMakeLists.txt:grpc_root:3:10"
)

for entry in "${PF_FILES[@]}"; do
  IFS=':' read -r filepath name warmup runs <<< "$entry"
  [[ ! -f "$filepath" ]] && echo "  SKIP: $name" && continue
  lines=$(wc -l < "$filepath" | tr -d ' ')
  echo "  $name ($lines lines)..."
  hyperfine \
    --warmup "$warmup" \
    --runs "$runs" \
    --export-json "$RESULTS_DIR/per-file/$name.json" \
    --command-name "cmakefmt" "$CMAKEFMT $filepath" \
    --ignore-failure \
    2>&1 | grep "Time (mean"
  echo ""
done

# ── Whole-repo: serial vs parallel ──────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Whole-Repo: serial vs parallel"
echo "═══════════════════════════════════════════════════════════════"
echo ""

REPOS=(
  blender bullet3 catch2 cmake fmt googletest grpc
  llvm nlohmann_json opencv protobuf spdlog vulkan_hpp
)

for name in "${REPOS[@]}"; do
  repo="$REPO_ROOT/benches/repos/$name"
  count=$("$CMAKEFMT" --list-input-files "$repo" 2>/dev/null | wc -l | tr -d ' ')
  [[ "$count" -eq 0 ]] && echo "  SKIP: $name" && continue

  echo "  $name ($count files)..."
  hyperfine \
    --warmup 3 \
    --runs 10 \
    --export-json "$RESULTS_DIR/whole-repo/$name.json" \
    --command-name "parallel" "$CMAKEFMT --check $repo" \
    --command-name "serial" "$CMAKEFMT --check --parallel 1 $repo" \
    --ignore-failure \
    2>&1 | grep "Time (mean"
  echo ""
done

if [[ -d "$OOMPH_DIR" ]]; then
  count=$("$CMAKEFMT" --list-input-files "$OOMPH_DIR" 2>/dev/null | wc -l | tr -d ' ')
  echo "  oomph-lib ($count files)..."
  hyperfine \
    --warmup 3 \
    --runs 10 \
    --export-json "$RESULTS_DIR/whole-repo/oomph-lib.json" \
    --command-name "parallel" "$CMAKEFMT --check $OOMPH_DIR" \
    --command-name "serial" "$CMAKEFMT --check --parallel 1 $OOMPH_DIR" \
    --ignore-failure \
    2>&1 | grep "Time (mean"
  echo ""
fi

# ── Summary ─────────────────────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Summary"
echo "═══════════════════════════════════════════════════════════════"
echo ""

python3 - "$RESULTS_DIR" "$REPO_ROOT" << 'PYEOF'
import json, os, sys

results_dir, repo_root = sys.argv[1:3]

file_paths = {
    "opencv_flann":  f"{repo_root}/target/real-world-corpus/opencv_flann/CMakeLists.txt",
    "googletest":    f"{repo_root}/target/real-world-corpus/googletest/CMakeLists.txt",
    "llvm_tablegen": f"{repo_root}/target/real-world-corpus/llvm_tablegen/CMakeLists.txt",
    "abseil":        f"{repo_root}/target/real-world-corpus/abseil/CMakeLists.txt",
    "spdlog":        f"{repo_root}/target/real-world-corpus/spdlog/CMakeLists.txt",
    "mariadb_server":f"{repo_root}/target/real-world-corpus/mariadb_server/CMakeLists.txt",
    "xnnpack":       f"{repo_root}/target/real-world-corpus/xnnpack/CMakeLists.txt",
    "opencv_root":   f"{repo_root}/target/real-world-corpus/opencv_root/CMakeLists.txt",
    "blender_root":  f"{repo_root}/target/real-world-corpus/blender_root/CMakeLists.txt",
    "llvm_libc_math":f"{repo_root}/target/real-world-corpus/llvm_libc_math/CMakeLists.txt",
    "grpc_root":     f"{repo_root}/target/real-world-corpus/grpc_root/CMakeLists.txt",
}

print("Per-file (sorted by lines):")
print(f"{'File':<20s} {'Lines':>8s} {'Time (ms)':>10s}")
print("-" * 42)

pf_dir = os.path.join(results_dir, "per-file")
rows = []
for fname in sorted(os.listdir(pf_dir)):
    if not fname.endswith(".json"): continue
    name = fname.replace(".json", "")
    data = json.load(open(os.path.join(pf_dir, fname)))
    path = file_paths.get(name, "")
    lines = sum(1 for _ in open(path)) if path and os.path.exists(path) else 0
    ms = data["results"][0]["mean"] * 1000
    rows.append((lines, name, ms))

rows.sort()
for lines, name, ms in rows:
    print(f"{name:<20s} {lines:>8,d} {ms:>10.1f}")

print()
print("Whole-repo (sorted by files):")
print(f"{'Repo':<20s} {'Files':>8s} {'Serial (ms)':>12s} {'Parallel (ms)':>14s} {'Speedup':>8s}")
print("-" * 66)

wr_dir = os.path.join(results_dir, "whole-repo")
wr_rows = []
for fname in sorted(os.listdir(wr_dir)):
    if not fname.endswith(".json"): continue
    name = fname.replace(".json", "")
    data = json.load(open(os.path.join(wr_dir, fname)))
    r = data["results"]
    if len(r) < 2: continue
    par = r[0]["mean"] * 1000
    ser = r[1]["mean"] * 1000
    # Estimate file count from the parallel command name
    wr_rows.append((name, par, ser))

# Sort by serial time as proxy for file count
wr_rows.sort(key=lambda x: x[2])
for name, par, ser in wr_rows:
    speedup = ser / par
    print(f"{name:<20s} {'':>8s} {ser:>12.0f} {par:>14.0f} {speedup:>7.1f}x")
PYEOF

echo ""
echo "Done. Results in: $RESULTS_DIR/"

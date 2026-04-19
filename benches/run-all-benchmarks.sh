#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Run the complete cmakefmt benchmark suite:
#   Phase 1: Per-file benchmarks (11 representative files, 2–55K lines)
#   Phase 2: Whole-repo benchmarks (14 repos, 4–11K files)
#   Phase 3: Compile results into a Markdown summary
#
# Prerequisites:
#   cargo build --release
#   pip install cmakelang  (provides cmake-format)
#   brew install hyperfine
#   ./benches/fetch-repos.sh
#   python3 scripts/fetch-real-world-corpus.py
#
# Usage:
#   ./benches/run-all-benchmarks.sh
#
# Output:
#   benches/results/per-file/*.json
#   benches/results/whole-repo/*.json
#   benches/results/summary.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CMAKEFMT="$REPO_ROOT/target/release/cmakefmt"
PF_DIR="$SCRIPT_DIR/results/per-file"
WR_DIR="$SCRIPT_DIR/results/whole-repo"
FL_DIR="$WR_DIR/filelists"
OOMPH_DIR="/Users/PuneetMatharu/Dropbox/programming/oomph-lib/oomph-lib-repos/forked-oomph-lib"
SUMMARY="$SCRIPT_DIR/results/summary.md"

# ── Preflight checks ────────────────────────────────────────────────────

for cmd in "$CMAKEFMT" cmake-format hyperfine python3; do
  if ! command -v "$cmd" &>/dev/null && [[ ! -x "$cmd" ]]; then
    echo "error: $cmd not found"
    exit 1
  fi
done

# ── Clean previous results ──────────────────────────────────────────────

rm -rf "$PF_DIR" "$WR_DIR"
mkdir -p "$PF_DIR" "$WR_DIR" "$FL_DIR"

echo "cmakefmt:     $($CMAKEFMT --version 2>&1 | head -1)"
echo "cmake-format: $(cmake-format --version 2>&1 | head -1)"
echo "hyperfine:    $(hyperfine --version 2>&1)"
echo ""

# ── Phase 1: Per-file benchmarks ────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 1: Per-File Benchmarks"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Format: filepath:name:warmup:runs
PF_FILES=(
  "$REPO_ROOT/target/real-world-corpus/opencv_flann/CMakeLists.txt:opencv_flann:10:50"
  "$REPO_ROOT/target/real-world-corpus/googletest/CMakeLists.txt:googletest:10:50"
  "$REPO_ROOT/target/real-world-corpus/llvm_tablegen/CMakeLists.txt:llvm_tablegen:10:50"
  "$REPO_ROOT/target/real-world-corpus/abseil/CMakeLists.txt:abseil:10:50"
  "$REPO_ROOT/target/real-world-corpus/spdlog/CMakeLists.txt:spdlog:10:50"
  "$REPO_ROOT/target/real-world-corpus/mariadb_server/CMakeLists.txt:mariadb_server:10:50"
  "$REPO_ROOT/target/real-world-corpus/xnnpack/CMakeLists.txt:xnnpack:10:50"
  "$REPO_ROOT/target/real-world-corpus/opencv_root/CMakeLists.txt:opencv_root:5:20"
  "$REPO_ROOT/target/real-world-corpus/blender_root/CMakeLists.txt:blender_root:5:20"
  "$REPO_ROOT/target/real-world-corpus/llvm_libc_math/CMakeLists.txt:llvm_libc_math:5:20"
  "$REPO_ROOT/target/real-world-corpus/grpc_root/CMakeLists.txt:grpc_root:3:5"
)

for entry in "${PF_FILES[@]}"; do
  IFS=':' read -r filepath name warmup runs <<< "$entry"

  if [[ ! -f "$filepath" ]]; then
    echo "  SKIP: $name ($filepath not found)"
    continue
  fi

  lines=$(wc -l < "$filepath" | tr -d ' ')
  echo "  $name ($lines lines, $warmup warmup, $runs runs)..."

  hyperfine \
    --warmup "$warmup" \
    --runs "$runs" \
    --export-json "$PF_DIR/$name.json" \
    --command-name "cmakefmt" "$CMAKEFMT $filepath" \
    --command-name "cmake-format" "cmake-format $filepath" \
    --ignore-failure

  echo ""
done

# ── Phase 2: Whole-repo benchmarks ──────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 2: Whole-Repository Benchmarks"
echo "═══════════════════════════════════════════════════════════════"
echo ""

REPOS=(
  blender bullet3 catch2 cmake fmt googletest grpc
  llvm nlohmann_json opencv protobuf spdlog vulkan_hpp
)

for name in "${REPOS[@]}"; do
  repo="$REPO_ROOT/benches/repos/$name"
  filelist="$FL_DIR/$name.txt"

  "$CMAKEFMT" --list-input-files "$repo" > "$filelist" 2>/dev/null || true
  count=$(wc -l < "$filelist" | tr -d ' ')

  if [[ "$count" -eq 0 ]]; then
    echo "  SKIP: $name (no CMake files)"
    continue
  fi

  echo "  $name ($count files)..."

  hyperfine \
    --warmup 3 \
    --runs 10 \
    --export-json "$WR_DIR/$name.json" \
    --command-name "cmakefmt-parallel" "$CMAKEFMT --check $repo" \
    --command-name "cmakefmt-serial" "$CMAKEFMT --check --parallel 1 $repo" \
    --command-name "cmake-format" "xargs cmake-format --check < $filelist" \
    --ignore-failure

  echo ""
done

# oomph-lib (external repo)
if [[ -d "$OOMPH_DIR" ]]; then
  filelist="$FL_DIR/oomph-lib.txt"
  "$CMAKEFMT" --list-input-files "$OOMPH_DIR" > "$filelist" 2>/dev/null || true
  count=$(wc -l < "$filelist" | tr -d ' ')
  echo "  oomph-lib ($count files)..."
  hyperfine \
    --warmup 3 \
    --runs 10 \
    --export-json "$WR_DIR/oomph-lib.json" \
    --command-name "cmakefmt-parallel" "$CMAKEFMT --check $OOMPH_DIR" \
    --command-name "cmakefmt-serial" "$CMAKEFMT --check --parallel 1 $OOMPH_DIR" \
    --command-name "cmake-format" "xargs cmake-format --check < $filelist" \
    --ignore-failure
  echo ""
fi

# ── Phase 3: Compile results ────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 3: Compiling Results"
echo "═══════════════════════════════════════════════════════════════"
echo ""

python3 - "$PF_DIR" "$WR_DIR" "$SUMMARY" "$REPO_ROOT" << 'PYEOF'
import json, os, sys, math
from pathlib import Path

pf_dir, wr_dir, summary_path, repo_root = sys.argv[1:5]

# ── Per-file ────────────────────────────────────────────────────────────

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

labels = {
    "opencv_flann":  "OpenCV (flann)",
    "googletest":    "googletest",
    "llvm_tablegen": "LLVM (TableGen)",
    "abseil":        "Abseil",
    "spdlog":        "spdlog",
    "mariadb_server":"MariaDB",
    "xnnpack":       "XNNPACK",
    "opencv_root":   "OpenCV (root)",
    "blender_root":  "Blender",
    "llvm_libc_math":"LLVM (libc math)",
    "grpc_root":     "gRPC",
}

out = []
out.append("# Benchmark Results\n")
out.append("## Per-File Benchmarks\n")
out.append("| Project | Lines | `cmakefmt` (ms) | `cmake-format` (ms) | Speedup |")
out.append("|---|---:|---:|---:|---:|")

pf_rows = []
for fname in sorted(os.listdir(pf_dir)):
    if not fname.endswith(".json"):
        continue
    name = fname.replace(".json", "")
    fpath = os.path.join(pf_dir, fname)
    if os.path.getsize(fpath) == 0:
        continue
    data = json.load(open(fpath))
    results = data["results"]
    if len(results) < 2:
        continue

    path = file_paths.get(name, "")
    lines = sum(1 for _ in open(path)) if path and os.path.exists(path) else 0
    cmf = results[0]["mean"] * 1000
    cf = results[1]["mean"] * 1000
    speedup = cf / cmf
    label = labels.get(name, name)
    pf_rows.append((lines, label, cmf, cf, speedup))

pf_rows.sort()
pf_speedups = []
for lines, label, cmf, cf, sp in pf_rows:
    out.append(f"| {label} | {lines:,} | {cmf:.1f} | {cf:.1f} | {sp:.1f}x |")
    pf_speedups.append(sp)

if pf_speedups:
    geo = math.exp(sum(math.log(s) for s in pf_speedups) / len(pf_speedups))
    out.append(f"\n**Geometric-mean speedup: {geo:.1f}x**\n")

# ── Whole-repo ──────────────────────────────────────────────────────────

out.append("## Whole-Repository Benchmarks\n")
out.append("| Repository | Files | `cmakefmt` (parallel) | `cmakefmt` (serial) | `cmake-format` | Speedup (serial) | Speedup (parallel) |")
out.append("|---|---:|---:|---:|---:|---:|---:|")

wr_rows = []
for fname in sorted(os.listdir(wr_dir)):
    if not fname.endswith(".json"):
        continue
    name = fname.replace(".json", "")
    fpath = os.path.join(wr_dir, fname)
    if os.path.getsize(fpath) == 0:
        continue
    data = json.load(open(fpath))
    results = data["results"]
    if len(results) < 3:
        continue

    fl_path = os.path.join(wr_dir, "filelists", f"{name}.txt")
    files = sum(1 for _ in open(fl_path)) if os.path.exists(fl_path) else 0

    par = results[0]["mean"] * 1000
    ser = results[1]["mean"] * 1000
    cf = results[2]["mean"] * 1000
    sp_ser = cf / ser
    sp_par = cf / par
    wr_rows.append((files, name, par, ser, cf, sp_ser, sp_par))

wr_rows.sort(key=lambda r: r[0])
wr_sp_ser = []
wr_sp_par = []
for files, name, par, ser, cf, sp_s, sp_p in wr_rows:
    out.append(f"| {name} | {files:,} | {par:.0f}ms | {ser:.0f}ms | {cf:.0f}ms | {sp_s:.1f}x | {sp_p:.1f}x |")
    wr_sp_ser.append(sp_s)
    wr_sp_par.append(sp_p)

if wr_sp_ser:
    geo_s = math.exp(sum(math.log(s) for s in wr_sp_ser) / len(wr_sp_ser))
    geo_p = math.exp(sum(math.log(s) for s in wr_sp_par) / len(wr_sp_par))
    out.append(f"\n**Geometric-mean speedup (serial): {geo_s:.1f}x**")
    out.append(f"**Geometric-mean speedup (parallel): {geo_p:.1f}x**\n")

with open(summary_path, "w") as f:
    f.write("\n".join(out) + "\n")

print(f"Summary written to {summary_path}")
print(f"  Per-file:   {len(pf_rows)} fixtures")
print(f"  Whole-repo: {len(wr_rows)} repositories")
PYEOF

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ALL DONE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Results: $SCRIPT_DIR/results/"
echo "Summary: $SUMMARY"

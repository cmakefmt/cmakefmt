#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Run the complete cmakefmt benchmark suite:
#   Phase 1: Per-file benchmarks (11 representative files, 2–55K lines)
#   Phase 2: Whole-repo benchmarks (14 repos, 4–11K files)
#   Phase 3: Parallel scaling (opencv + oomph-lib at --parallel 1/2/4/8 + RSS)
#   Phase 4: Compile results into a Markdown summary
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
#   SKIP_CMAKE_FORMAT=1 ./benches/run-all-benchmarks.sh
#
# Environment:
#   SKIP_CMAKE_FORMAT  When set to 1, skip every `cmake-format` invocation
#                      in phases 1 and 2. The summary table omits the
#                      cmake-format column. Useful when iterating on
#                      cmakefmt and the prior cmake-format numbers are
#                      already trusted.
#
# Output:
#   benches/results/per-file/*.json
#   benches/results/whole-repo/*.json
#   benches/results/parallelism/*.json
#   benches/results/parallelism/rss.txt
#   benches/results/summary.md

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CMAKEFMT="$REPO_ROOT/target/release/cmakefmt"
PF_DIR="$SCRIPT_DIR/results/per-file"
WR_DIR="$SCRIPT_DIR/results/whole-repo"
PARA_DIR="$SCRIPT_DIR/results/parallelism"
FL_DIR="$WR_DIR/filelists"
SUMMARY="$SCRIPT_DIR/results/summary.md"
SKIP_CMAKE_FORMAT="${SKIP_CMAKE_FORMAT:-0}"

# ── Preflight checks ────────────────────────────────────────────────────

required=("$CMAKEFMT" hyperfine python3)
if [[ "$SKIP_CMAKE_FORMAT" != "1" ]]; then
  required+=(cmake-format)
fi
for cmd in "${required[@]}"; do
  if ! command -v "$cmd" &>/dev/null && [[ ! -x "$cmd" ]]; then
    echo "error: $cmd not found"
    exit 1
  fi
done

# ── Clean previous results ──────────────────────────────────────────────

rm -rf "$PF_DIR" "$WR_DIR" "$PARA_DIR"
mkdir -p "$PF_DIR" "$WR_DIR" "$FL_DIR" "$PARA_DIR"

echo "cmakefmt:     $($CMAKEFMT --version 2>&1 | head -1)"
if [[ "$SKIP_CMAKE_FORMAT" == "1" ]]; then
  echo "cmake-format: skipped (SKIP_CMAKE_FORMAT=1)"
else
  echo "cmake-format: $(cmake-format --version 2>&1 | head -1)"
fi
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

  pf_args=(
    --warmup "$warmup"
    --runs "$runs"
    --export-json "$PF_DIR/$name.json"
    --command-name "cmakefmt" "$CMAKEFMT $filepath"
  )
  if [[ "$SKIP_CMAKE_FORMAT" != "1" ]]; then
    pf_args+=(--command-name "cmake-format" "cmake-format $filepath")
  fi
  hyperfine "${pf_args[@]}" --ignore-failure

  echo ""
done

# ── Phase 2: Whole-repo benchmarks ──────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 2: Whole-Repository Benchmarks"
echo "═══════════════════════════════════════════════════════════════"
echo ""

REPOS=(
  blender bullet3 catch2 fmt googletest grpc llvm nlohmann_json
  protobuf spdlog vulkan_hpp opencv oomph-lib cmake
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

  wr_args=(
    --warmup 3
    --runs 10
    --export-json "$WR_DIR/$name.json"
    --command-name "cmakefmt-parallel" "$CMAKEFMT --check $repo"
    --command-name "cmakefmt-serial" "$CMAKEFMT --check --parallel 1 $repo"
  )
  if [[ "$SKIP_CMAKE_FORMAT" != "1" ]]; then
    wr_args+=(--command-name "cmake-format" "xargs cmake-format --check < $filelist")
  fi
  hyperfine "${wr_args[@]}" --ignore-failure

  echo ""
done

# ── Phase 3: Parallel scaling ───────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 3: Parallel Scaling"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Repos to measure: pairs of "name:path". Both live in benches/repos
PARA_TARGETS=("opencv:$REPO_ROOT/benches/repos/opencv")
PARA_TARGETS+=("oomph-lib:$REPO_ROOT/benches/repos/oomph-lib")

RSS_FILE="$PARA_DIR/rss.txt"
: > "$RSS_FILE"

for entry in "${PARA_TARGETS[@]}"; do
  IFS=':' read -r name path <<< "$entry"
  if [[ ! -d "$path" ]]; then
    echo "  SKIP: $name ($path not found)"
    continue
  fi

  count=$("$CMAKEFMT" --list-input-files "$path" 2>/dev/null | wc -l | tr -d ' ')
  echo "  $name ($count files)..."

  hyperfine \
    --warmup 3 \
    --runs 10 \
    --export-json "$PARA_DIR/$name.json" \
    --command-name "serial" "$CMAKEFMT --check --parallel 1 $path" \
    --command-name "p2"     "$CMAKEFMT --check --parallel 2 $path" \
    --command-name "p4"     "$CMAKEFMT --check --parallel 4 $path" \
    --command-name "p8"     "$CMAKEFMT --check --parallel 8 $path" \
    --ignore-failure

  # Peak RSS — macOS only (Linux GNU time has different flags / output).
  if [[ "$OSTYPE" == "darwin"* ]]; then
    # `cmakefmt --check` exits 1 when any file would be reformatted; combined
    # with `set -euo pipefail`, that propagates through the pipeline and kills
    # the script. Swallow it — we only care about RSS here.
    rss_serial=$(/usr/bin/time -l "$CMAKEFMT" --check --parallel 1 "$path" 2>&1 >/dev/null \
      | awk '/maximum resident set size/ {print $1}' || true)
    rss_p8=$(/usr/bin/time -l "$CMAKEFMT" --check --parallel 8 "$path" 2>&1 >/dev/null \
      | awk '/maximum resident set size/ {print $1}' || true)
    printf "%s\tserial\t%s\n%s\tp8\t%s\n" \
      "$name" "$rss_serial" "$name" "$rss_p8" >> "$RSS_FILE"
  fi

  echo ""
done

# ── Phase 4: Compile results ────────────────────────────────────────────

echo "═══════════════════════════════════════════════════════════════"
echo "  Phase 4: Compiling Results"
echo "═══════════════════════════════════════════════════════════════"
echo ""

python3 - "$PF_DIR" "$WR_DIR" "$PARA_DIR" "$SUMMARY" "$REPO_ROOT" << 'PYEOF'
import json, os, sys, math
from pathlib import Path

pf_dir, wr_dir, para_dir, summary_path, repo_root = sys.argv[1:6]

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

pf_rows = []
pf_has_cf = False
for fname in sorted(os.listdir(pf_dir)):
    if not fname.endswith(".json"):
        continue
    name = fname.replace(".json", "")
    fpath = os.path.join(pf_dir, fname)
    if os.path.getsize(fpath) == 0:
        continue
    data = json.load(open(fpath))
    by_name = {r["command"]: r["mean"] * 1000 for r in data["results"]}
    cmf = by_name.get("cmakefmt")
    if cmf is None:
        continue
    cf = by_name.get("cmake-format")
    if cf is not None:
        pf_has_cf = True

    path = file_paths.get(name, "")
    lines = sum(1 for _ in open(path)) if path and os.path.exists(path) else 0
    label = labels.get(name, name)
    pf_rows.append((lines, label, cmf, cf))

pf_rows.sort()

if pf_has_cf:
    out.append("| Project | Lines | `cmakefmt` (ms) | `cmake-format` (ms) | Speedup |")
    out.append("|---|---:|---:|---:|---:|")
else:
    out.append("| Project | Lines | `cmakefmt` (ms) |")
    out.append("|---|---:|---:|")

pf_speedups = []
for lines, label, cmf, cf in pf_rows:
    if pf_has_cf:
        if cf is None:
            out.append(f"| {label} | {lines:,} | {cmf:.1f} | — | — |")
        else:
            sp = cf / cmf
            pf_speedups.append(sp)
            out.append(f"| {label} | {lines:,} | {cmf:.1f} | {cf:.1f} | {sp:.1f}x |")
    else:
        out.append(f"| {label} | {lines:,} | {cmf:.1f} |")

if pf_speedups:
    geo = math.exp(sum(math.log(s) for s in pf_speedups) / len(pf_speedups))
    out.append(f"\n**Geometric-mean speedup: {geo:.1f}x**\n")
else:
    out.append("")

# ── Whole-repo ──────────────────────────────────────────────────────────

out.append("## Whole-Repository Benchmarks\n")

wr_rows = []
wr_has_cf = False
for fname in sorted(os.listdir(wr_dir)):
    if not fname.endswith(".json"):
        continue
    name = fname.replace(".json", "")
    fpath = os.path.join(wr_dir, fname)
    if os.path.getsize(fpath) == 0:
        continue
    data = json.load(open(fpath))
    by_name = {r["command"]: r["mean"] * 1000 for r in data["results"]}
    par = by_name.get("cmakefmt-parallel")
    ser = by_name.get("cmakefmt-serial")
    if par is None or ser is None:
        continue
    cf = by_name.get("cmake-format")
    if cf is not None:
        wr_has_cf = True

    fl_path = os.path.join(wr_dir, "filelists", f"{name}.txt")
    files = sum(1 for _ in open(fl_path)) if os.path.exists(fl_path) else 0

    wr_rows.append((files, name, par, ser, cf))

wr_rows.sort(key=lambda r: r[0])

if wr_has_cf:
    out.append("| Repository | Files | `cmakefmt` (parallel) | `cmakefmt` (serial) | `cmake-format` | Speedup (serial) | Speedup (parallel) |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
else:
    out.append("| Repository | Files | `cmakefmt` (parallel) | `cmakefmt` (serial) |")
    out.append("|---|---:|---:|---:|")

wr_sp_ser = []
wr_sp_par = []
for files, name, par, ser, cf in wr_rows:
    if wr_has_cf:
        if cf is None:
            out.append(f"| {name} | {files:,} | {par:.0f}ms | {ser:.0f}ms | — | — | — |")
        else:
            sp_s = cf / ser
            sp_p = cf / par
            wr_sp_ser.append(sp_s)
            wr_sp_par.append(sp_p)
            out.append(f"| {name} | {files:,} | {par:.0f}ms | {ser:.0f}ms | {cf:.0f}ms | {sp_s:.1f}x | {sp_p:.1f}x |")
    else:
        out.append(f"| {name} | {files:,} | {par:.0f}ms | {ser:.0f}ms |")

if wr_sp_ser:
    geo_s = math.exp(sum(math.log(s) for s in wr_sp_ser) / len(wr_sp_ser))
    geo_p = math.exp(sum(math.log(s) for s in wr_sp_par) / len(wr_sp_par))
    out.append(f"\n**Geometric-mean speedup (serial): {geo_s:.1f}x**")
    out.append(f"**Geometric-mean speedup (parallel): {geo_p:.1f}x**\n")
else:
    out.append("")

# ── Parallel scaling ────────────────────────────────────────────────────

para_rows = []
for fname in sorted(os.listdir(para_dir)):
    if not fname.endswith(".json"):
        continue
    name = fname.replace(".json", "")
    fpath = os.path.join(para_dir, fname)
    if os.path.getsize(fpath) == 0:
        continue
    data = json.load(open(fpath))
    by_name = {r["command"]: r["mean"] * 1000 for r in data["results"]}
    para_rows.append((name, by_name))

if para_rows:
    out.append("## Parallel Scaling\n")
    out.append("| Repository | serial | --parallel 2 | --parallel 4 | --parallel 8 | speedup at p8 |")
    out.append("|---|---:|---:|---:|---:|---:|")
    for name, t in para_rows:
        ser = t.get("serial", 0)
        sp8 = ser / t["p8"] if t.get("p8") else 0
        out.append(
            f"| {name} | {ser:.0f}ms | {t.get('p2', 0):.0f}ms | "
            f"{t.get('p4', 0):.0f}ms | {t.get('p8', 0):.0f}ms | {sp8:.1f}x |"
        )

    rss_path = os.path.join(para_dir, "rss.txt")
    if os.path.exists(rss_path) and os.path.getsize(rss_path) > 0:
        out.append("\n### Peak RSS (macOS)\n")
        out.append("| Repository | serial | --parallel 8 |")
        out.append("|---|---:|---:|")
        rss = {}
        for line in open(rss_path):
            parts = line.strip().split("\t")
            if len(parts) == 3:
                repo, kind, bytes_str = parts
                rss.setdefault(repo, {})[kind] = int(bytes_str) / (1024 * 1024)
        for repo, vals in rss.items():
            ser = vals.get("serial", 0)
            p8 = vals.get("p8", 0)
            out.append(f"| {repo} | {ser:.1f} MB | {p8:.1f} MB |")
    out.append("")

with open(summary_path, "w") as f:
    f.write("\n".join(out) + "\n")

print(f"Summary written to {summary_path}")
print(f"  Per-file:    {len(pf_rows)} fixtures")
print(f"  Whole-repo:  {len(wr_rows)} repositories")
print(f"  Parallelism: {len(para_rows)} repositories")
PYEOF

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ALL DONE"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "Results: $SCRIPT_DIR/results/"
echo "Summary: $SUMMARY"

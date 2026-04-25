#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Produce the version-trend chart datapoint for a new release.
#
# Measures:
#   - release binary size of `target/release/cmakefmt`, in MB
#   - median wall-clock time of `cmakefmt <fixture>` over N runs, in ms,
#     via hyperfine (same tool used by the rest of `benches/`)
#
# The fixture is the same 656-line CMakeLists.txt used for every prior
# datapoint in `docs/src/components/VersionTrendChart.astro`, so the new
# value is comparable head-to-head with the existing series.
#
# Output is a one-line summary plus the literal `{ version: ..., time: ...,
# binary: ... }` line you can paste straight into the chart's `DATA` array.
#
# Prerequisites:
#   cargo build --release --features cli
#   brew install hyperfine
#   python3 scripts/fetch-real-world-corpus.py    (to populate qtbase, mariadb, …)
#
# Usage:
#   ./benches/version-trend-datapoint.sh                  # default settings
#   ./benches/version-trend-datapoint.sh -v               # show hyperfine table
#   ./benches/version-trend-datapoint.sh --print-command  # print the hyperfine
#                                                         # command and exit
#   RUNS=500 ./benches/version-trend-datapoint.sh         # custom run count
#   WARMUP=200 ./benches/version-trend-datapoint.sh       # custom warmup count
#   FIXTURE=/path/to/file.cmake ./benches/version-trend-datapoint.sh
#   CMAKEFMT=/path/to/other-binary ./benches/version-trend-datapoint.sh
#
# Flags:
#   -v, --verbose         show hyperfine's stats table on stdout in
#                         addition to the script's own one-line summary
#   -p, --print-command   print the hyperfine command this script would
#                         run, suitable for copy-paste into a shell, and
#                         exit without running it. The printed command
#                         omits `--export-json` so running it directly
#                         shows hyperfine's stats table.
#
# Exit codes:
#   0  measurement succeeded (or --print-command emitted the command)
#   1  binary, fixture, or hyperfine missing — see message
#   2  unknown flag
#
# Methodology:
#   - `--shell=none` removes the `sh -c` wrapper hyperfine adds by
#     default. Each iteration is a direct fork+exec of cmakefmt, with
#     no shell startup or argv parsing in the timing loop. Empirically
#     more stable than the default shelled invocation by ~0.3 ms.
#   - `--style basic` skips hyperfine's per-iteration ANSI progress
#     bar. The progress redraws perturb cache + scheduler state for
#     the next subprocess, inflating the median by ~0.3-0.6 ms vs
#     `--style basic` or `--style none`.
#   - 100 warmups + 200 runs balance stability with run time
#     (~3 seconds wall-clock per invocation on a 656-line fixture).
#     Override via `WARMUP` and `RUNS` env vars.
#   - Reports hyperfine's median, not min — closer to typical
#     CLI-startup behaviour than min, more robust than mean against
#     outliers.
#   - Both numbers go into VersionTrendChart.astro and the prose
#     paragraph immediately below it on the performance page.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CMAKEFMT="${CMAKEFMT:-$REPO_ROOT/target/release/cmakefmt}"
FIXTURE="${FIXTURE:-$REPO_ROOT/target/real-world-corpus/mariadb_server/CMakeLists.txt}"
WARMUP="${WARMUP:-100}"
RUNS="${RUNS:-200}"

VERBOSE=0
PRINT_CMD=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    -v | --verbose)
      VERBOSE=1
      shift
      ;;
    -p | --print-command)
      PRINT_CMD=1
      shift
      ;;
    -h | --help)
      awk 'NR >= 2 && NR <= 53 { sub(/^# ?/, ""); print }' "$0"
      exit 0
      ;;
    *)
      echo "error: unknown flag: $1" >&2
      echo "       see $0 --help" >&2
      exit 2
      ;;
  esac
done

# ── Preflight ──────────────────────────────────────────────────────────

if [[ ! -x "$CMAKEFMT" ]]; then
  echo "error: $CMAKEFMT not found or not executable" >&2
  echo "       run: cargo build --release --features cli" >&2
  exit 1
fi

if [[ ! -f "$FIXTURE" ]]; then
  echo "error: fixture not found: $FIXTURE" >&2
  echo "       run: python3 scripts/fetch-real-world-corpus.py" >&2
  exit 1
fi

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "error: hyperfine not on PATH" >&2
  echo "       run: brew install hyperfine" >&2
  exit 1
fi

# ── Binary size (MB, one decimal) ──────────────────────────────────────

binary_bytes=$(stat -f%z "$CMAKEFMT" 2>/dev/null || stat -c%s "$CMAKEFMT")
binary_mb=$(awk -v b="$binary_bytes" 'BEGIN { printf "%.1f", b / (1024 * 1024) }')

# ── Wall-clock median over $RUNS iterations ────────────────────────────

fixture_lines=$(wc -l <"$FIXTURE" | tr -d ' ')

hyperfine_args=(
  --shell=none
  --style basic
  --warmup "$WARMUP"
  --runs "$RUNS"
  --command-name "cmakefmt $(basename "$FIXTURE")"
)

# --print-command emits a copy-pasteable hyperfine invocation and exits
# without running it. It omits `--export-json` (no scratch file needed
# for an interactive run); the user sees hyperfine's stats table
# directly. `--verbose` is a no-op for the printed form because the
# printed command always produces output.
if (( PRINT_CMD )); then
  printf 'hyperfine'
  for arg in "${hyperfine_args[@]}" "$CMAKEFMT $FIXTURE"; do
    printf ' %q' "$arg"
  done
  printf '\n'
  exit 0
fi

trend_json=$(mktemp -t cmakefmt-trend.XXXXXX.json)
trap 'rm -f "$trend_json"' EXIT
hyperfine_args+=(--export-json "$trend_json")

if (( VERBOSE )); then
  hyperfine "${hyperfine_args[@]}" "$CMAKEFMT $FIXTURE"
  echo  # blank line before our summary so it doesn't run into hyperfine's table
else
  hyperfine "${hyperfine_args[@]}" "$CMAKEFMT $FIXTURE" >/dev/null
fi

# hyperfine reports times in seconds in JSON; pull the median and convert.
time_ms=$(awk '
  /"median"/ {
    gsub(/[",]/, "", $2)
    printf "%.1f", $2 * 1000
    exit
  }
' "$trend_json")

# ── Report ─────────────────────────────────────────────────────────────

cat <<EOF
binary:    ${binary_mb} MB  ($binary_bytes bytes)
wall time: ${time_ms} ms median over ${RUNS} runs (hyperfine, --shell=none --style basic --warmup ${WARMUP})
fixture:   $(basename "$(dirname "$FIXTURE")")/$(basename "$FIXTURE") (${fixture_lines} lines)
host:      $(uname -sm)

paste into docs/src/components/VersionTrendChart.astro DATA[]:

    { version: "vX.Y.Z", time: ${time_ms}, binary: ${binary_mb} },
EOF

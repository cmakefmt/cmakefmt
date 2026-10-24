#!/usr/bin/env python3
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Capture successful fixture formatting in a deterministic comparison artifact."""

import argparse
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--fixtures", type=Path, default=Path("tests/fixtures"))
    parser.add_argument("--output", type=Path, default=Path("cross-platform-output.txt"))
    args = parser.parse_args()
    # A failed rerun must not leave a previous successful artifact behind.
    args.output.unlink(missing_ok=True)
    fixtures = sorted(
        path for path in args.fixtures.rglob("*")
        if path.is_file() and path.suffix in {".cmake", ".txt"}
    )
    if not fixtures:
        parser.error(f"no CMake fixtures found under {args.fixtures}")

    records = []
    for fixture in fixtures:
        # Keep stderr visible and require success on every platform. Decode the
        # raw bytes explicitly so Python cannot hide CRLF differences from diff.
        result = subprocess.run(
            [str(args.binary.resolve()), "--color", "never", str(fixture)],
            stdout=subprocess.PIPE,
            check=True,
        )
        records.append({
            "file": fixture.relative_to(args.fixtures).as_posix(),
            "formatted": result.stdout.decode("utf-8"),
        })

    args.output.write_bytes((json.dumps(records, indent=2, ensure_ascii=False) + "\n").encode("utf-8"))
    print(f"Captured {len(records)} fixtures in {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()

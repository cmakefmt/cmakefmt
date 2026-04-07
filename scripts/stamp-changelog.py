#!/usr/bin/env python3

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Promote '## Unreleased' to '## <version> — <date>' in CHANGELOG.md.

Replaces the '## Unreleased' heading with the versioned heading and inserts a
fresh empty '## Unreleased' section above it, ready for the next release cycle.

Usage:
    python3 scripts/stamp-changelog.py <version> [<date>]

Arguments:
    version   Plain semver string, e.g. 0.2.0
    date      ISO 8601 date string (default: today, UTC), e.g. 2026-04-07
"""

import re
import sys
from datetime import datetime, timezone
from pathlib import Path

CHANGELOG = Path("CHANGELOG.md")


def main() -> None:
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    version = sys.argv[1]
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        print(f"error: version must be plain semver without a leading v, got {version!r}")
        sys.exit(1)

    date = sys.argv[2] if len(sys.argv) >= 3 else datetime.now(timezone.utc).strftime("%Y-%m-%d")

    text = CHANGELOG.read_text(encoding="utf-8")

    pattern = re.compile(r"^## Unreleased\s*$", re.MULTILINE)
    if not pattern.search(text):
        print("error: no '## Unreleased' heading found in CHANGELOG.md")
        sys.exit(1)

    stamped_heading = f"## {version} \u2014 {date}"
    fresh_unreleased = "## Unreleased\n\n"
    replacement = f"{fresh_unreleased}{stamped_heading}"
    updated = pattern.sub(replacement, text, count=1)
    CHANGELOG.write_text(updated, encoding="utf-8")
    print(f"stamped CHANGELOG.md: Unreleased \u2192 {version} \u2014 {date} (new Unreleased section added)")


if __name__ == "__main__":
    main()

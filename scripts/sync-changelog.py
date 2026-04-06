#!/usr/bin/env python3

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Generate the docs changelog page from the canonical root CHANGELOG.md."""

import subprocess
from pathlib import Path

FRONTMATTER = """\
---
title: Changelog
---

<!--
SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu

SPDX-License-Identifier: MIT OR Apache-2.0
-->
"""

command = ["git", "rev-parse", "--show-toplevel"]
repo_root = Path(subprocess.check_output(command, text=True).strip())
root_changelog = repo_root / "CHANGELOG.md"
docs_changelog = repo_root / "docs" / "src" / "content" / "docs" / "changelog.md"

if not root_changelog.exists():
    raise SystemExit(f"error: {root_changelog} not found")

content = root_changelog.read_text()

# Strip the top-level heading.
content = content.removeprefix("# Changelog\n")

# Remove everything from "## Release Process" onwards.
marker = "\n## Release Process"
idx = content.find(marker)
if idx != -1:
    content = content[:idx]

# Assemble and write, ensuring exactly one trailing newline.
docs_changelog.write_text(FRONTMATTER + "\n" + content.strip() + "\n")

#!/usr/bin/env python3

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Run opted-in docs examples and validate config-reference code blocks."""

import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile


REPO_ROOT = Path(__file__).resolve().parent.parent
MARKER = "<!-- docs-check -->"
FENCE = re.compile(r"^```([^\n]*)\n(.*?)^```[ \t]*$", re.MULTILINE | re.DOTALL)


def run(command, cwd, env, source=None):
    result = subprocess.run(
        command, cwd=cwd, env=env, input=source, text=True,
        capture_output=True, timeout=30,
    )
    if result.returncode:
        raise ValueError(
            f"{' '.join(map(str, command))} exited {result.returncode}\n"
            f"{result.stdout}{result.stderr}"
        )
    return result.stdout


def check_example(language, code, binary, wasm_dir):
    with tempfile.TemporaryDirectory(prefix="cmakefmt-docs-") as directory:
        root = Path(directory)
        (root / "src").mkdir()
        (root / "src/CMakeLists.txt").write_text("set(FOO bar)\n")
        env = dict(os.environ, HOME=str(root), XDG_CONFIG_HOME=str(root))
        env["PATH"] = str(binary.parent) + os.pathsep + env["PATH"]

        if language in {"yaml", "toml"}:
            config = root / f"example.{language}"
            config.write_text(code)
            run([binary, "config", "check", config], root, env)
            # Loading config alone doesn't resolve command specs. Format too.
            run([binary, "--config-file", config, "--verify", "-"], root, env,
                "set(FOO bar)\n")
        elif language == "bash":
            run(["bash", "-euo", "pipefail", "-c", code], root, env)
            for config in list(root.glob("*.yaml")) + list(root.glob("*.toml")):
                run([binary, "config", "check", config], root, env)
        elif language == "javascript":
            package = root / "pkg"
            package.mkdir()
            for name in ("cmakefmt.js", "cmakefmt_bg.wasm"):
                shutil.copyfile(wasm_dir / name, package / name)
            (package / "package.json").write_text('{"type":"module"}\n')
            example = root / "format.mjs"
            example.write_text(code)
            output = run(["node", example], root, env)
            expected = "\n".join(re.findall(r"^// → (.*)$", code, re.MULTILINE))
            if not expected or output.strip() != expected:
                raise ValueError(f"expected {expected!r}, got {output.strip()!r}")
        else:
            raise ValueError(f"unsupported docs-check language: {language!r}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--wasm-dir", type=Path, default=REPO_ROOT / "docs/public/wasm")
    parser.add_argument("--docs-dir", type=Path, default=REPO_ROOT / "docs/src/content/docs")
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"build the CLI first; binary not found: {binary}")
    if not args.docs_dir.is_dir():
        parser.error(f"docs directory not found: {args.docs_dir}")

    count = 0
    for page in sorted(args.docs_dir.rglob("*")):
        if page.suffix not in {".md", ".mdx"}:
            continue
        text = page.read_text()
        marked_count = 0
        for match in FENCE.finditer(text):
            language = match[1].strip().split(" ")[0]
            marked = text[:match.start()].rstrip().endswith(MARKER)
            marked_count += marked
            if not marked and not (page.name == "config.md" and language in {"yaml", "toml"}):
                continue
            line = text.count("\n", 0, match.start()) + 1
            try:
                check_example(language, match[2], binary, args.wasm_dir.resolve())
            except (ValueError, OSError, subprocess.TimeoutExpired) as error:
                raise SystemExit(f"{page}:{line}: {error}") from error
            count += 1
        if marked_count != text.count(MARKER):
            raise SystemExit(f"{page}: each docs-check marker must precede a fenced block")
    if not count:
        raise SystemExit("no documentation examples found")
    print(f"checked {count} documentation examples")


if __name__ == "__main__":
    main()

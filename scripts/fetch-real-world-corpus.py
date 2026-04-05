#!/usr/bin/env python3
# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

from __future__ import annotations

import argparse
import hashlib
import pathlib
import sys
import tomllib
import urllib.request


def parse_args() -> argparse.Namespace:
    # fmt: off
    parser = argparse.ArgumentParser(description="Fetch the pinned real-world CMake corpus used for local review.")
    parser.add_argument("--dest", type=pathlib.Path, default=pathlib.Path("target/real-world-corpus"), help="Directory to populate with fetched fixture files.")
    parser.add_argument("--refresh", action="store_true", help="Re-download files even if a cached copy already matches the expected hash.")
    # fmt: on
    return parser.parse_args()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_manifest() -> dict:
    manifest_path = pathlib.Path("tests/fixtures/real_world/manifest.toml")
    return tomllib.loads(manifest_path.read_text(encoding="utf-8"))


def fetch_bytes(url: str) -> bytes:
    with urllib.request.urlopen(url) as response:
        return response.read()


def ensure_fixture(dest_root: pathlib.Path, fixture: dict, refresh: bool) -> str:
    dest = dest_root / fixture["relative_path"]
    expected = fixture["sha256"]
    dest.parent.mkdir(parents=True, exist_ok=True)

    if dest.exists() and not refresh:
        current = dest.read_bytes()
        if sha256_bytes(current) == expected:
            return f"cached  {fixture['name']}"

    payload = fetch_bytes(fixture["raw_url"])
    actual = sha256_bytes(payload)
    if actual != expected:
        raise SystemExit(
            f"hash mismatch for {fixture['name']}: expected {expected}, got {actual}\n"
            f"source: {fixture['source_url']}"
        )

    dest.write_bytes(payload)
    return f"fetched {fixture['name']}"


def main() -> int:
    args = parse_args()
    manifest = load_manifest()
    args.dest.mkdir(parents=True, exist_ok=True)

    for fixture in manifest["fixture"]:
        print(ensure_fixture(args.dest, fixture, args.refresh))

    print(f"real-world corpus ready at {args.dest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

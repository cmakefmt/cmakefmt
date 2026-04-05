#!/usr/bin/env python3

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

from __future__ import annotations

import argparse
from pathlib import Path
from urllib.parse import urljoin
from xml.sax.saxutils import escape


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate sitemap.xml and robots.txt for the built docs site."
    )
    parser.add_argument("site_dir", type=Path, help="Built site directory, for example docs/book")
    parser.add_argument(
        "--base-url",
        default="https://cmakefmt.dev/",
        help="Canonical absolute site URL, ending with a slash.",
    )
    return parser.parse_args()


def iter_html_pages(site_dir: Path) -> list[Path]:
    pages = []
    for path in sorted(site_dir.rglob("*.html")):
        if path.name in {"print.html", "404.html"}:
            continue
        pages.append(path)
    return pages


def to_url(base_url: str, site_dir: Path, path: Path) -> str:
    rel = path.relative_to(site_dir).as_posix()
    if rel == "index.html":
        return base_url
    return urljoin(base_url, rel)


def write_sitemap(site_dir: Path, urls: list[str]) -> None:
    sitemap_path = site_dir / "sitemap.xml"
    entries = "\n".join(
        f"  <url><loc>{escape(url)}</loc></url>" for url in urls
    )
    sitemap_path.write_text(
        "\n".join(
            [
                '<?xml version="1.0" encoding="UTF-8"?>',
                '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
                entries,
                "</urlset>",
                "",
            ]
        ),
        encoding="utf-8",
    )


def write_robots(site_dir: Path, base_url: str) -> None:
    robots_path = site_dir / "robots.txt"
    robots_path.write_text(
        "\n".join(
            [
                "User-agent: *",
                "Allow: /",
                "",
                f"Sitemap: {urljoin(base_url, 'sitemap.xml')}",
                "",
            ]
        ),
        encoding="utf-8",
    )


def main() -> int:
    args = parse_args()
    site_dir = args.site_dir.resolve()
    base_url = args.base_url.rstrip("/") + "/"

    urls = [to_url(base_url, site_dir, path) for path in iter_html_pages(site_dir)]
    write_sitemap(site_dir, urls)
    write_robots(site_dir, base_url)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

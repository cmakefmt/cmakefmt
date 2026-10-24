#!/usr/bin/env python3

# SPDX-FileCopyrightText: Copyright 2026 Puneet Matharu
#
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Check links and fragments against the generated static documentation site."""

import argparse
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__()
        self.ids = set()
        self.links = []
        self.feed(path.read_text())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("href"):
            self.links.append((self.getpos()[0], attrs["href"]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("site", type=Path, help="Astro build output directory")
    args = parser.parse_args()
    root = args.site.resolve()
    pages = {path: Page(path) for path in sorted(root.rglob("*.html"))}
    if not pages:
        parser.error(f"no HTML pages found in {root}; build the docs first")

    errors = []
    count = 0
    for path, page in pages.items():
        for line, href in page.links:
            url = urlsplit(href)
            if url.netloc:
                if url.netloc != "cmakefmt.dev" or url.scheme not in {"", "http", "https"}:
                    continue
            elif url.scheme:
                continue
            target = unquote(url.path)
            if not target:
                destination = root / "index.html" if url.netloc else path
            elif target.startswith("/"):
                destination = root / target.lstrip("/")
            else:
                destination = path.parent / target
            destination = destination.resolve()
            if destination.is_dir():
                destination /= "index.html"

            problem = None
            fragment = unquote(url.fragment).split(":~:text=", 1)[0]
            if not destination.is_relative_to(root) or not destination.is_file():
                problem = "missing file"
            elif fragment and destination in pages and fragment not in pages[destination].ids:
                problem = "missing anchor"
            if problem:
                errors.append(f"{path.relative_to(root)}:{line}: {href}: {problem}")
            count += 1
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"checked {count} internal links across {len(pages)} pages")


if __name__ == "__main__":
    main()

# `docs/`

This directory contains the Astro + Starlight source for the GitHub Pages
documentation site for `cmakefmt`.

## Local Preview

Install dependencies and start the local dev server:

```bash
cd docs
npm install
npm run dev
```

Then open the local URL Astro prints, usually <http://localhost:4321>.

To build the static site without serving it:

```bash
cd docs
npm run build
```

## Rules

- `MagicMove.astro` uses `@shikijs/magic-move` with Shiki 4 for homepage animations.
- keep the sidebar in `docs/astro.config.mjs` aligned with the available pages
- put published docs pages under `docs/src/content/docs/`
- keep the site content aligned with `README.md`, `CHANGELOG.md`, and
  `CONTRIBUTING.md`

## Documentation Checks

`npm run build` checks internal links and fragment anchors in the rendered site.
The Pages workflow also runs CLI, config, and Node.js WASM examples:

```bash
cargo build --locked
bash scripts/build-wasm.sh
python3 scripts/check-doc-examples.py --binary target/debug/cmakefmt
```

Run these commands from the repository root. The WASM build requires `wasm-pack`
and the `wasm32-unknown-unknown` Rust target; the example checks require Python 3.9+
and Node.js 22+. Each runnable example executes in a temporary directory with a
sample `src/CMakeLists.txt`. Add `<!-- docs-check -->` immediately before a
`bash`, `yaml`, `toml`, or `javascript` fenced block to opt it in. Only mark
shell examples that need no network access, installed packages, or real Git repo.
JavaScript examples use the generated web package under `pkg/` and check stdout
against their `// → ...` comments.

Every YAML/TOML block in the Config Reference is checked automatically, including
custom command specs, so keep those blocks valid standalone configurations.

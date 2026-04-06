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

- keep the sidebar in `docs/astro.config.mjs` aligned with the available pages
- put published docs pages under `docs/src/content/docs/`
- keep the site content aligned with `README.md`, `CHANGELOG.md`, and
  `CONTRIBUTING.md`

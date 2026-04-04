# Install

## Build From This Repository

```bash
cargo build --release
./target/release/cmakefmt --help
```

## Install With Cargo

```bash
cargo install --path .
```

## Typical Local Workflow

```bash
cmakefmt --check .
cmakefmt -i .
cmakefmt --dump-config > .cmakefmt.yaml
cmakefmt --staged --check
```

For larger custom-command specs, prefer keeping your edited user config in
`.cmakefmt.yaml` or `.cmakefmt.yml`. `--dump-config` emits YAML by default;
pass `toml` if you want a TOML starter instead.

## Pre-commit

The repository already ships a pre-commit configuration:

```bash
pre-commit install
pre-commit install --hook-type pre-push
```

For lightweight manual hooks, these workflow-oriented commands are the useful baseline:

```bash
cmakefmt --staged --check
cmakefmt --changed --since origin/main --check
```

## Package Managers

Cargo-based install is available now. Homebrew and other package-manager
distribution are planned for the alpha-release phase, not guaranteed yet.

## Local Docs Preview

Use `mdbook` to preview the docs:

```bash
mdbook serve site
```

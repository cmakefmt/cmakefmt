# Troubleshooting

Your diagnostic companion for the most common failure modes and confusing
situations when using `cmakefmt`.

## `cmakefmt` Did Not Find The Files I Expected

Check how you invoked it:

- direct file arguments are always processed
- directories are recursively discovered
- discovery respects `.cmakefmtignore`
- discovery respects `.gitignore` unless you pass `--no-gitignore`
- `--path-regex` filters only discovered paths, not direct file arguments

See exactly what was found and why:

```bash
cmakefmt --list-files .
cmakefmt --debug --list-files .
```

## The Wrong Config File Was Used

Find out exactly what config was selected and what the formatter is actually
running with:

```bash
cmakefmt --show-config-path path/to/CMakeLists.txt
cmakefmt --show-config path/to/CMakeLists.txt
cmakefmt --explain-config path/to/CMakeLists.txt
```

These commands tell you:

- which config file was selected
- which files were considered
- what CLI overrides changed the final result

## A Config Key Was Rejected

`cmakefmt` deliberately fails fast on unknown config keys instead of silently
ignoring them. This is by design — a typo that disappears into a warning is a
much worse experience than an immediate error.

Common causes:

- typo in a key name
- using an old draft key name
- confusing `commands:` (argument structure) with `per_command_overrides:` (layout/style)

If you are migrating from legacy `cmake-format`:

```bash
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.toml
```

Then adapt the result to `.cmakefmt.yaml` if you want YAML as your final
format.

## Parse Error On Valid-Looking CMake

Run the failing file with full diagnostics:

```bash
cmakefmt --debug --check path/to/CMakeLists.txt
```

`cmakefmt` should surface:

- file path
- line/column
- source context
- a likely-cause hint when one can be inferred

If the failure is inside a highly custom DSL region, exclude that block
temporarily with barrier markers:

```cmake
# cmakefmt: off
...
# cmakefmt: on
```

## My Custom Command Formats Poorly

This almost always means the command registry does not know the command's
syntax yet.

Add a `commands:` entry to your config:

```yaml
commands:
  my_custom_command:
    pargs: 1
    flags:
      - QUIET
    kwargs:
      SOURCES:
        nargs: "+"
```

Once `cmakefmt` understands the argument structure, it can produce
keyword-aware, properly grouped output instead of flattening everything into a
token stream.

If you only want layout or style tweaks for an already-known command, use
`per_command_overrides:` instead.

## Stdin Formatting Ignores My Project Config

When formatting stdin, `cmakefmt` has no real file path and cannot discover
config automatically. Fix this with `--stdin-path`:

```bash
cat src/CMakeLists.txt | cmakefmt - --stdin-path src/CMakeLists.txt
```

## The Output Surprises Me

Turn on formatter diagnostics and let `cmakefmt` explain itself:

```bash
cmakefmt --debug path/to/CMakeLists.txt
```

The debug stream shows:

- which command form was chosen
- which layout family was chosen
- whether barriers or fences were active
- which effective thresholds applied

## I Want Quieter CI Logs

```bash
cmakefmt --check --quiet .
```

For machine-readable output that scripts and dashboards can consume:

```bash
cmakefmt --check --report-format json .
```

## I Want The Run To Continue After A Bad File

```bash
cmakefmt --keep-going --check .
```

`--keep-going` lets the formatter process remaining files and print an
aggregated summary instead of aborting at the first file-level error. Useful
for triage on large repositories.

## I Suspect A Performance Regression

Run the benchmark suite:

```bash
cargo bench --bench formatter
```

For one-off workflow timing, compare representative commands with `hyperfine`.
The repository's full benchmark process is documented in the performance notes.

## Still Stuck?

When reporting an issue, include:

- the exact command you ran
- the file that failed, or a minimized reproduction
- your `.cmakefmt.yaml` or `.cmakefmt.toml`
- the full stderr output
- `--debug` output when the problem is about formatting behavior rather than a hard parse error

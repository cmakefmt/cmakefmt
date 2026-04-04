# Troubleshooting

This page covers the most common failure modes and confusing situations when
using `cmakefmt`.

## `cmakefmt` Did Not Find The Files I Expected

Check how you invoked it:

- direct file arguments are always processed
- directories are recursively discovered
- discovery respects `.cmakefmtignore`
- discovery respects `.gitignore` unless you pass `--no-gitignore`
- `--path-regex` only filters discovered paths, not direct file arguments

Useful commands:

```bash
cmakefmt --list-files .
cmakefmt --debug --list-files .
```

## The Wrong Config File Was Used

Use:

```bash
cmakefmt --show-config-path path/to/CMakeLists.txt
cmakefmt --show-config path/to/CMakeLists.txt
cmakefmt --explain-config path/to/CMakeLists.txt
```

That will tell you:

- which config file was selected
- which files were considered
- what CLI overrides changed the final result

## A Config Key Was Rejected

`cmakefmt` deliberately fails fast on unknown config keys instead of silently
ignoring them.

Common causes:

- typo in a key name
- using an old draft key name
- mixing `commands:` and `per_command_overrides:` concepts

If you are migrating from legacy `cmake-format`, prefer:

```bash
cmakefmt --convert-legacy-config .cmake-format.py > .cmakefmt.toml
```

then adapt the result to `.cmakefmt.yaml` if you want YAML as your final format.

## Parse Error On Valid-Looking CMake

Run the failing file with:

```bash
cmakefmt --debug --check path/to/CMakeLists.txt
```

`cmakefmt` should show:

- file path
- line/column
- source context
- a likely-cause hint when one can be inferred

If the failure is inside a highly custom DSL region, use barrier markers to
exclude that block temporarily:

```cmake
# cmakefmt: off
...
# cmakefmt: on
```

## My Custom Command Formats Poorly

That usually means the command registry does not know the command's syntax yet.

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

If you only want layout/style tweaks for a known command, use
`per_command_overrides:` instead.

## Stdin Formatting Ignores My Project Config

When formatting stdin, use `--stdin-path` so config discovery behaves as if the
buffer had a real file path:

```bash
cat src/CMakeLists.txt | cmakefmt - --stdin-path src/CMakeLists.txt
```

## The Output Surprises Me

Turn on formatter diagnostics:

```bash
cmakefmt --debug path/to/CMakeLists.txt
```

The debug stream will tell you:

- which command form was chosen
- which layout family was chosen
- whether barriers/fences were active
- which effective thresholds applied

## I Want Quieter CI Logs

Use:

```bash
cmakefmt --check --quiet .
```

If you want machine-readable output instead:

```bash
cmakefmt --check --report-format json .
```

## I Want The Run To Continue After A Bad File

Use:

```bash
cmakefmt --keep-going --check .
```

That lets the formatter keep processing later files and print an aggregated
summary instead of failing immediately at the first file-level error.

## I Suspect A Performance Regression

Run the benchmark suite:

```bash
cargo bench --bench formatter
```

For one-off workflow timing, compare representative commands with `hyperfine`.
The repository's benchmark process is documented in the performance notes.

## Still Stuck?

When reporting an issue, include:

- the exact command you ran
- the file that failed, or a minimized reproduction
- your `.cmakefmt.yaml` / `.cmakefmt.toml`
- the full stderr output
- `--debug` output when the problem is about formatting rather than a hard parse error

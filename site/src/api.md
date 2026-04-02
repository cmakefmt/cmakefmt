# Library API

## Public Entry Points

- `format_source`
- `format_source_with_debug`
- `format_source_with_registry`
- `format_source_with_registry_debug`
- `Config`, `CaseStyle`, `DangleAlign`, `PerCommandConfig`
- `Error`, `Result`

## Example

```rust
use cmakefmt::{format_source, Config};

fn main() -> Result<(), cmakefmt::Error> {
    let src = r#"target_link_libraries(foo PUBLIC bar)"#;
    let out = format_source(src, &Config::default())?;
    println!("{out}");
    Ok(())
}
```

## Error Model

| Error | Meaning |
| --- | --- |
| `Error::Parse` | source did not parse as valid CMake under the current grammar |
| `Error::Config` | config-file parse failure |
| `Error::Spec` | built-in or override spec parse failure |
| `Error::Io` | I/O failure when reading or writing |
| `Error::Formatter` | formatter-layer invariant or unsupported case |

## Stability

The public crate surface exists and is usable now, but the project is still
pre-1.0. Expect some churn until the alpha/release phases settle the
compatibility contract.

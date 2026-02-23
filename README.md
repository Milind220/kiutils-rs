# kiutils-rs

Rust-native, sync-first KiCad parser/formatter with lossless round-trip defaults.

Scope (v1):
- `.kicad_pcb`
- `.kicad_mod`
- `fp-lib-table`
- `.kicad_dru`
- `.kicad_pro`

Current status:
- Workspace with two crates:
  - `kiutils_sexpr`: lossless S-expression CST parser
  - `kiutils_kicad`: typed KiCad API layer
- Implemented: initial `PcbFile::read` path with lossless write-back and tests
- Implemented: typed readers for PCB/footprint/lib-table/design-rules/project
- Implemented: unknown token/field capture and `WriteMode::{Lossless, Canonical}`

Design goals:
- KiCad v10 primary, v9 secondary
- Lossless default write mode for minimal SCM diffs
- Unknown token preservation for forward compatibility
- Typed API with explicit diagnostics/errors

## Quick start

```bash
cargo test
```

Feature checks:

```bash
cargo test -p kiutils_kicad --features serde
cargo test -p kiutils_kicad --features parallel
```

## License

MIT

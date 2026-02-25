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

Implementation policy (locked 2026-02-23):
- `kiutils/` Python tree is read-only baseline/reference.
- Simplify by scope/cohesion, not arbitrary file-length splits.
- AST `*_count` values are debug convenience (non-normative API).
- `kiutils-inspect` machine schema stability first; debug detail should be opt-in.
- Version checks stay post-parse by default for compatibility.
- Pre-v9 PCB/footprint versions are parsed in best-effort compatibility mode with `legacy_format` warnings.
- `.kicad_dru` condition strings stay verbatim in v1.
- Unknown diagnostics are developer-focused; end-user mode should summarize.

## Quick start

```bash
cargo test
```

Feature checks:

```bash
cargo test -p kiutils_kicad --features serde
cargo test -p kiutils_kicad --features parallel
```

## Inspect CLI (test loop)

Run parser inspection on a real file:

```bash
cargo run -p kiutils_kicad --bin kiutils-inspect -- <path>
```

Flags:
- `--type auto|pcb|footprint|fplib|dru|project`
- `--json`
- `--show-cst`
- `--show-canonical`
- `--show-unknown`
- `--show-diagnostics`

Example:

```bash
cargo run -p kiutils_kicad --bin kiutils-inspect -- \
  crates/kiutils_kicad/tests/fixtures/sample.kicad_pcb \
  --show-unknown --show-diagnostics --show-canonical
```

## License

MIT

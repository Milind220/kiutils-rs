# kiutils-rs

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/Milind220/kiutils-rs)

Rust-native KiCad parser/formatter focused on lossless round-trips, typed APIs, and forward compatibility.

## What This Project Does

`kiutils-rs` is a Rust workspace with three crates:

- `kiutils_sexpr`: lossless S-expression CST parser/printer.
- `kiutils_kicad`: typed KiCad document APIs on top of the CST layer (implementation crate).
- `kiutils-rs`: public-facing crate for the stable end-user API (Rust import: `kiutils_rs`).

Default behavior is lossless read/modify/write so unrelated formatting is preserved unless canonical output is explicitly requested.

## Public API File Types (v1 scope)

- `.kicad_pcb`
- `.kicad_mod`
- `fp-lib-table`
- `.kicad_dru`
- `.kicad_pro`

Compatibility target:

- Primary: KiCad v10
- Secondary: KiCad v9

Versioning policy:

- Public API (`kiutils-rs`) follows [SemVer](https://semver.org/).

## Alpha Documentation Note

`kiutils-rs` is still alpha. You may see `missing_docs` warnings in some crates while the public
surface is being trimmed and stabilized.

Current policy:
- Core user-facing APIs and workflows are documented first.
- Broader internal/public-by-default surfaces may stay partially undocumented temporarily.
- Coverage will tighten as API boundaries settle.

## Key Behavior

- Lossless output by default (`WriteMode::Lossless`).
- Optional canonical normalized output (`WriteMode::Canonical`).
- Unknown token/field capture for forward compatibility.
- Typed mutation helpers for common edits.
- Parser depth guard for deeply nested malformed input.

## Getting Started

Book-style guide:

- Build locally: `mdbook build docs`
- Read online (after GitHub Pages deploy): `https://milind220.github.io/kiutils-rs/`

### Build and test

```bash
cargo test
```

Feature checks:

```bash
cargo test -p kiutils-rs --features serde
cargo test -p kiutils-rs --features parallel
```

### Quick API example (PCB)

```rust
use kiutils_rs::PcbFile;

let mut doc = PcbFile::read("input.kicad_pcb")?;

doc.set_version(20260101)
    .set_generator("kiutils")
    .set_generator_version("dev")
    .set_paper_standard("A4", Some("portrait"))
    .set_title("Demo Board")
    .upsert_property("Owner", "Milind")
    .remove_property("Obsolete");

doc.write("output.kicad_pcb")?;
```

### Quick API example (Project JSON)

```rust
use kiutils_rs::ProjectFile;

let doc = ProjectFile::read("input.kicad_pro")?;
println!("meta.version = {:?}", doc.ast().meta_version);
doc.write("output.kicad_pro")?;
```

## Important API Notes

- Use document setter APIs (`set_*`, `upsert_*`, `remove_*`) for serializable edits.
- `ast_mut()` is read-side/debug convenience only. If you mutate via `ast_mut()`, `write()` returns a validation error because those changes are not auto-reconciled into CST.

## Inspect CLI

Inspect typed parse results quickly:

```bash
cargo run -p kiutils_kicad --bin kiutils-inspect -- <path>
```

Common flags:

- `--type auto|pcb|footprint|schematic|sch|symbol|fplib|symlib|dru|project|worksheet|wks`
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

## Additional Round-Trip Examples

```bash
cargo run -p kiutils_kicad --example pcb_roundtrip -- input.kicad_pcb output.kicad_pcb
cargo run -p kiutils_kicad --example footprint_roundtrip -- input.kicad_mod output.kicad_mod
cargo run -p kiutils_kicad --example schematic_roundtrip -- input.kicad_sch output.kicad_sch
cargo run -p kiutils_kicad --example symbol_roundtrip -- input.kicad_sym output.kicad_sym
cargo run -p kiutils_kicad --example symlib_roundtrip -- sym-lib-table sym-lib-table.out
cargo run -p kiutils_kicad --example dru_roundtrip -- input.kicad_dru output.kicad_dru
cargo run -p kiutils_kicad --example worksheet_roundtrip -- input.kicad_wks output.kicad_wks
```

Corpus-style examples (demo trees):

```bash
cargo run -p kiutils_kicad --example pcb_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/pcbs
cargo run -p kiutils_kicad --example footprint_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/footprints
cargo run -p kiutils_kicad --example schematic_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/schematics
cargo run -p kiutils_kicad --example symbol_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/symbols
cargo run -p kiutils_kicad --example symlib_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/symlib
cargo run -p kiutils_kicad --example dru_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/dru
cargo run -p kiutils_kicad --example worksheet_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/worksheets
```

## Development Workflow

- Format: `cargo fmt --all`
- Test: `cargo test`
- Optional lint: `cargo clippy --all-targets --all-features -- -D warnings`

## Future Work

Open backlog moved from cleanup notes:

### Medium priority

- `LibTableDocument::diagnostics()` currently carries little signal and should be made useful or removed.
- `property_count` can diverge from parsed property-vector length in some cases; normalize semantics.
- Module scaffolding is duplicated across formats; reduce drift risk with shared abstractions where beneficial.
- `kiutils-inspect` has repetitive per-format code paths; refactor while keeping machine-readable schema stable.

### Low priority

- Generator token quoting differs across some document types; align serialization policy.
- Local helper duplication in `lib_table` vs shared `sexpr_utils`.
- `VersionPolicy` is exported but not currently injectable into reader/diagnostic paths.

### Needs fixture confirmation

- `generated.members_count` may undercount alternate members syntax forms.
- Escape handling for non-quote escapes may diverge from KiCad expectations.
- `parse_paper` heuristic may misclassify malformed mixed forms.

## License

MIT

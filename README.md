# kiutils-rs

Rust-native, sync-first KiCad parser/formatter with lossless round-trip defaults.

Scope (v1):
- `.kicad_pcb`
- `.kicad_mod`
- `.kicad_sch`
- `.kicad_sym`
- `fp-lib-table`
- `sym-lib-table`
- `.kicad_dru`
- `.kicad_pro`
- `.kicad_wks`

Current status:
- Workspace with two crates:
  - `kiutils_sexpr`: lossless S-expression CST parser
  - `kiutils_kicad`: typed KiCad API layer
- Implemented: initial `PcbFile::read` path with lossless write-back and tests
- Implemented: typed readers for PCB/footprint/lib-table/design-rules/project
- Implemented: typed reader for symbol library tables (`sym-lib-table`)
- Implemented: typed reader for schematics (`.kicad_sch`)
- Implemented: typed reader for symbol libraries (`.kicad_sym`)
- Implemented: typed reader for worksheets (`.kicad_wks`)
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

## Footprint read/modify/write

Chainable edit API on `FootprintDocument`:

```rust
use kiutils_kicad::FootprintFile;

let mut doc = FootprintFile::read("input.kicad_mod")?;
doc.set_lib_id("My_Footprint")
    .set_version(20260101)
    .set_generator("kiutils")
    .set_generator_version("dev")
    .set_layer("B.Cu")
    .set_descr("demo")
    .set_tags("passive")
    .set_reference("R1")
    .set_value("10k")
    .upsert_property("LCSC", "C25804")
    .remove_property("Obsolete");

doc.write("output.kicad_mod")?;
```

Runnable example:

```bash
cargo run -p kiutils_kicad --example footprint_roundtrip -- input.kicad_mod output.kicad_mod
```

## PCB read/modify/write

Chainable edit API on `PcbDocument`:

```rust
use kiutils_kicad::PcbFile;

let mut doc = PcbFile::read("input.kicad_pcb")?;
doc.set_version(20260101)
    .set_generator("kiutils")
    .set_generator_version("dev")
    .set_paper_standard("A4", Some("portrait"))
    .set_title("Demo Board")
    .set_date("2026-02-25")
    .set_revision("B")
    .set_company("Lords")
    .upsert_property("Owner", "Milind")
    .remove_property("Obsolete");

doc.write("output.kicad_pcb")?;
```

Runnable example:

```bash
cargo run -p kiutils_kicad --example pcb_roundtrip -- input.kicad_pcb output.kicad_pcb
```

## Schematic read/modify/write

```rust
use kiutils_kicad::SchematicFile;

let mut doc = SchematicFile::read("input.kicad_sch")?;
doc.set_version(20260101)
    .set_generator("eeschema")
    .set_generator_version("9.0")
    .set_uuid("f5f10a8b-1d4a-4de6-9a77-5fd4d17f3cc5")
    .set_paper_standard("A4", Some("portrait"))
    .set_title("Demo Schematic")
    .set_date("2026-02-25")
    .set_revision("B")
    .set_company("Lords");

doc.write("output.kicad_sch")?;
```

Runnable examples:

```bash
cargo run -p kiutils_kicad --example schematic_roundtrip -- input.kicad_sch output.kicad_sch
cargo run -p kiutils_kicad --example schematic_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/schematics
```

## Symbol library read/modify/write

```rust
use kiutils_kicad::SymbolLibFile;

let mut doc = SymbolLibFile::read("input.kicad_sym")?;
doc.set_version(20260101)
    .set_generator("kiutils")
    .set_generator_version("dev")
    .rename_first_symbol("RenamedSymbol")
    .upsert_symbol_property("RenamedSymbol", "Value", "NewValue");

doc.write("output.kicad_sym")?;
```

Runnable examples:

```bash
cargo run -p kiutils_kicad --example symbol_roundtrip -- input.kicad_sym output.kicad_sym
cargo run -p kiutils_kicad --example symbol_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/symbols
```

## Symbol-lib-table read/modify/write

```rust
use kiutils_kicad::SymLibTableFile;

let mut doc = SymLibTableFile::read("sym-lib-table")?;
doc.set_version(7)
    .rename_library("Base", "BaseEdited")
    .add_library("Extra", "${KIPRJMOD}/Extra.kicad_sym")
    .remove_library("Obsolete");

doc.write("sym-lib-table.out")?;
```

Runnable examples:

```bash
cargo run -p kiutils_kicad --example symlib_roundtrip -- sym-lib-table sym-lib-table.out
cargo run -p kiutils_kicad --example symlib_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/symlib
```

## Design-rules read/modify/write

```rust
use kiutils_kicad::DesignRulesFile;

let mut doc = DesignRulesFile::read("input.kicad_dru")?;
doc.set_version(1)
    .rename_first_rule("base_rule")
    .upsert_rule_layer("base_rule", "outer")
    .upsert_rule_condition("base_rule", "A.NetClass == 'DDR4'");

doc.write("output.kicad_dru")?;
```

Runnable examples:

```bash
cargo run -p kiutils_kicad --example dru_roundtrip -- input.kicad_dru output.kicad_dru
cargo run -p kiutils_kicad --example dru_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/dru
```

## Worksheet read/modify/write

```rust
use kiutils_kicad::WorksheetFile;

let mut doc = WorksheetFile::read("input.kicad_wks")?;
doc.set_version(20260101)
    .set_generator("pl_editor")
    .set_generator_version("9.0")
    .set_setup_line_width(0.2)
    .set_setup_text_size(1.7, 1.8);

doc.write("output.kicad_wks")?;
```

Runnable examples:

```bash
cargo run -p kiutils_kicad --example worksheet_roundtrip -- input.kicad_wks output.kicad_wks
cargo run -p kiutils_kicad --example worksheet_corpus_roundtrip -- ~/Engineering/demos crates/kiutils_kicad/examples/generated/worksheets
```

## License

MIT

## Future Work

- Avoiding full-file canonical rewrites for targeted mutations is currently achieved for tested targeted single-node edits (for example `version`, `generator`, `title`) with local diffs instead of whole-file churn.
- Beyond that tested scope, behavior is not yet broadly validated across all mutation patterns.
- Further expansion of targeted span-patch coverage can be explored.

# Examples

## Real example: PCB round-trip

Snippet included from source so docs stay synced:

```rust
{{#include ../../crates/kiutils_kicad/examples/pcb_roundtrip.rs:pcb_roundtrip_main}}
```

Run it:

```bash
cargo run -p kiutils-kicad --example pcb_roundtrip -- input.kicad_pcb output.kicad_pcb
```

## Corpus-style examples

```bash
cargo run -p kiutils-kicad --example pcb_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/pcbs
cargo run -p kiutils-kicad --example footprint_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/footprints
cargo run -p kiutils-kicad --example schematic_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/schematics
cargo run -p kiutils-kicad --example symbol_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/symbols
cargo run -p kiutils-kicad --example symlib_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/symlib
cargo run -p kiutils-kicad --example dru_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/dru
cargo run -p kiutils-kicad --example worksheet_corpus_roundtrip -- <input_dir> crates/kiutils_kicad/examples/generated/worksheets
```

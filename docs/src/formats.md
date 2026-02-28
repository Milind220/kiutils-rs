# Supported Formats

Current v1 file support in public API:

| File | Type |
| --- | --- |
| `.kicad_pcb` | PCB |
| `.kicad_mod` | Footprint |
| `fp-lib-table` | Footprint lib table |
| `sym-lib-table` | Symbol lib table |
| `.kicad_dru` | Design rules |
| `.kicad_pro` | Project JSON |

Additional parser support exists in implementation crates (for example schematic/symbol/worksheet), while the stable `kiutils-rs` public surface is intentionally narrow.

## Write modes

| Mode | Behavior |
| --- | --- |
| `WriteMode::Lossless` | Preserves unrelated formatting/tokens |
| `WriteMode::Canonical` | Emits normalized/canonical representation |

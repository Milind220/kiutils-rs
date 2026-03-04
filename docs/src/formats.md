# Supported Formats

Current v1 file support in public API:

| File | Type |
| --- | --- |
| `.kicad_pcb` | PCB |
| `.kicad_mod` | Footprint |
| `.kicad_sch` | Schematic |
| `.kicad_sym` | Symbol library |
| `fp-lib-table` | Footprint lib table |
| `sym-lib-table` | Symbol lib table |
| `.kicad_dru` | Design rules |
| `.kicad_pro` | Project JSON |

## Write modes

| Mode | Behavior |
| --- | --- |
| `WriteMode::Lossless` | Preserves unrelated formatting/tokens |
| `WriteMode::Canonical` | Emits normalized/canonical representation |

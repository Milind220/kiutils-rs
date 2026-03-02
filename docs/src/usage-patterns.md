# Usage Patterns

This chapter is tuned for automation systems and code generators.

## Pattern: safe typed mutation

- Parse with `*_File::read(...)`
- Update through document setters (`set_*`, `upsert_*`, `remove_*`)
- Write with `write(...)` or `write_mode(...)`

Why: setter APIs reconcile AST/CST correctly.

Examples:
- `ProjectDocument::set_pinned_symbol_libs(...)`
- `ProjectDocument::set_pinned_footprint_libs(...)`
- `LibTableDocument::upsert_library_uri(...)` (updates existing URI only; adds if missing)

## Common pitfalls

| Pitfall | What happens | Correct pattern |
| --- | --- | --- |
| Mutating with `ast_mut()` then calling `write()` | Validation error for non-reconciled state | Use setter/upsert/remove helpers |
| Assuming unknown tokens are dropped | Future syntax might be lost in other libraries | `kiutils-rs` captures unknowns and round-trips them |
| Forcing canonical output always | Noisy diffs in VCS | Default to lossless; canonical only when required |

## Minimal code path

```rust,no_run
use kiutils_rs::{PcbFile, WriteMode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = PcbFile::read("input.kicad_pcb")?;
    doc.upsert_property("EditedBy", "agent");
    doc.write_mode("output.kicad_pcb", WriteMode::Lossless)?;
    Ok(())
}
```

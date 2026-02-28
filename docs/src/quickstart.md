# Quickstart

## Parse, edit, write

```rust,no_run
use kiutils_rs::PcbFile;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = PcbFile::read("input.kicad_pcb")?;

    doc.set_version(20260101)
        .set_generator("kiutils")
        .set_generator_version("dev")
        .set_title("Demo Board")
        .upsert_property("Owner", "Milind")
        .remove_property("Obsolete");

    doc.write("output.kicad_pcb")?;
    Ok(())
}
```

## Build + test

```bash
cargo test
cargo test -p kiutils-rs --features serde
cargo test -p kiutils-rs --features parallel
```

## Inspect CLI (typed summary)

```bash
cargo run -p kiutils-kicad --bin kiutils-inspect -- \
  crates/kiutils_kicad/tests/fixtures/sample.kicad_pcb \
  --show-unknown --show-diagnostics --show-canonical
```

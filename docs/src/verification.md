# Verification

Evidence in repository:

- Integration tests cover fixture-based round-trip behavior across supported document kinds.
- Unknown syntax preservation tested explicitly.
- `kiutils-inspect` JSON/text contract smoke tests present.

## Test evidence snippet

```rust
{{#include ../../crates/kiutils_kicad/tests/integration.rs:pcb_roundtrip_test}}
```

## Local gate

```bash
cargo fmt --all
cargo test
cargo clippy --all-targets --all-features -- -D warnings
mdbook build docs
```

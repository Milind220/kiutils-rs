# kiutils-rs v1 Architecture Plan (KiCad v10-first, v9-secondary, lossless)

## Summary
Build a Rust-native parser/formatter for:
1. `.kicad_pcb`
2. `.kicad_mod`
3. `fp-lib-table`
4. `.kicad_dru`
5. `.kicad_pro` (project JSON)

Exclude: schematics/symbols/symbol libraries.

Core strategy:
- Two-layer model: `lossless CST` + `typed AST`.
- Default read path: parse once, expose typed API, preserve unknowns/order for round-trip.
- v10 primary compatibility, v9 secondary compatibility.
- API not mirroring Python; Rust-first ergonomic + explicit error/version handling.

## Why this is better than current `kiutils` baseline
Observed in `/Users/milindsharma/Developer/kicad-oss/kiutils-rs/kiutils`:
- Parser core is regex + nested Python lists (`src/kiutils/utils/sexpr.py`).
- Token dispatch is manual `if item[0] == ...` chains (`src/kiutils/board.py`, `/footprint.py`, `/dru.py`, `/libraries.py`).
- Unknown token handling is weak (generally dropped in typed projection).
- `.kicad_dru` requires wrapper hack for root parse (`src/kiutils/dru.py`).
- Test style is mostly file equality, good baseline but limited typed invariants (`tests/testfunctions.py`).
- Known issues acknowledge ordering/format drift (`docs/misc/known-issues.rst`).

Rust port improvements:
- Deterministic parser combinators + typed enums/structs.
- Lossless preservation channel for forward compatibility.
- Strong `Result` errors with span/path context.
- Strict version policy tied to KiCad format versions.
- Cleaner APIs for partial reads/mutations and safe write-back.

## Public API / interfaces (decision-complete)
Crate layout (phase 1):
1. `kiutils_sexpr` (lexer/parser/printer + CST)
2. `kiutils_kicad` (typed AST + file-specific codecs)

Planned crate layout (phase 2+):
1. `kiutils_sexpr`
2. `kiutils_kicad`
3. `kiutils` facade crate for stable end-user API

Primary end-user API (delivered via `kiutils_kicad` first, then moved/re-exported via facade crate):
- `PcbFile::read(path) -> Result<PcbDocument, Error>`
- `FootprintFile::read(path) -> Result<FootprintDocument, Error>`
- `FpLibTableFile::read(path) -> Result<FpLibTableDocument, Error>`
- `DesignRulesFile::read(path) -> Result<DesignRulesDocument, Error>`
- `ProjectFile::read(path) -> Result<ProjectDocument, Error>`

Document dual view:
- `doc.ast()` typed immutable view
- `doc.ast_mut()` typed mutable view
- `doc.cst()` raw/lossless tree view
- `doc.write(path)` lossless round-trip by default
- `doc.write_mode(WriteMode::Canonical)` optional normalized output

Key types:
- `VersionPolicy { target: V10, accept: [V9, V10], reject_older: true }`
- `UnknownNode` + `UnknownField` carriers on typed nodes
- `Span { file, byte_range, line_col }`
- `Diagnostic { severity, code, message, span, hint }`

Feature flags:
- `lossless` (default on)
- `serde` (JSON serialization for AST)
- `parallel` (rayon-powered bulk parse)

## File-format modeling details (scoped)
1. `.kicad_pcb` / `.kicad_mod`:
- S-expression grammar with tokenized atoms, strings, numbers, symbols.
- Preserve unrecognized nodes as attached siblings to nearest typed parent.
- Version-gated fields via `since` markers in parser tables.
- Typed enums for graphic items, pads, zones, vias, groups, tables, barcodes, etc.

2. `.kicad_dru`:
- Rootless rule-list grammar supported natively (no wrapper hack).
- Constraints parsed as typed variants + fallback `Constraint::Unknown`.
- Rule condition strings preserved verbatim; no DSL rewriter in v1.

3. `fp-lib-table`:
- Dedicated typed table/row AST.
- Preserve entry ordering and unknown row fields.
- URI/options/descr preserved raw + normalized accessor helpers.

4. `.kicad_pro`:
- JSON parser with typed top-level known sections plus passthrough `extra`.
- Preserve unknown sections/keys exactly in lossless mode.
- Typed support for sections relevant to PCB/project + pinned footprint libs.

## Parsing/serialization architecture
Pipeline:
1. Read bytes
2. Parse CST (lossless tokens + trivia)
3. Build typed AST with node references into CST
4. Mutations applied to AST
5. Reconcile AST -> CST patch
6. Print (lossless default; canonical optional)

No “full regenerate from AST” default.
Reason: forward compatibility + minimal SCM diff.

## Error handling and compatibility policy
Error classes:
- `ParseError`
- `VersionError`
- `ValidationError`
- `IoError`

Behavior:
- Unknown token: warning diagnostic, preserved node.
- Unsupported mandatory token for selected target version: hard error.
- Future version marker (> v10 known): parse in compatibility mode if syntax valid, keep unknowns, emit high-severity warning.

## Test cases and scenarios
Golden tests:
1. Parse->write byte-identical for untouched files (v9/v10 fixtures per file type).
2. Targeted edits preserve unrelated formatting/trivia.
3. Unknown token injection survives round-trip unchanged.

Semantic tests:
1. AST fields map correctly for board/footprint/rules/lib-table/project.
2. Version-gated fields accepted/rejected per policy.
3. Cross-file linkage sanity (`.kicad_pro` pinned libs + `fp-lib-table` entries).

Failure tests:
1. Malformed S-expr with precise span diagnostics.
2. Rootless `.kicad_dru` edge cases.
3. Future-version files with warning/error behavior.

Property/fuzz:
1. S-expr tokenizer/parser fuzz corpus.
2. Printer idempotence under no-op transforms.

Performance:
1. Benchmarks on medium/large PCB files.
2. Parse throughput + memory snapshots.
3. Batch parse on repo trees.

## Implementation phases
1. Foundation
- Workspace, crates, diagnostics, span infra, tokenizer/parser skeleton.

2. Lossless S-expression core
- CST + printer + fuzz harness + golden tests.

3. Typed AST for `.kicad_pcb` + `.kicad_mod`
- Highest-value tokens first; unknown preservation always on.

4. `fp-lib-table` + `.kicad_dru`
- Add file-specific codecs, compatibility tests.

5. `.kicad_pro`
- JSON typed+passthrough model, project-level helpers.

6. Facade stabilization
- Introduce third crate `kiutils` as thin facade/re-export.
- Public API polish, docs, examples, migration notes from Python `kiutils`.

## Assumptions and defaults locked
- Target compatibility: KiCad v10 primary, v9 secondary; no v6/v7 support commitment in v1.
- Round-trip mode: lossless default.
- Library scope: `fp-lib-table` + project pinned footprint libs; exclude `sym-lib-table`.
- No schematic/symbol/symbol-lib parsing.
- No expression-evaluator for design-rule conditions in v1 (string-preserving only).

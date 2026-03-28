//! # kiutils-kicad
//!
//! Typed KiCad file document layer built on top of `kiutils-sexpr`.
//!
//! If you want stable end-user imports, use [`kiutils-rs`](https://docs.rs/kiutils-rs).
//! This crate exposes the implementation-layer API and additional file families.
//!
//! ## Scope
//! - `.kicad_pcb`
//! - `.kicad_mod`
//! - `.kicad_sch`
//! - `.kicad_sym`
//! - `fp-lib-table`
//! - `sym-lib-table`
//! - `.kicad_dru`
//! - `.kicad_pro`
//! - `.kicad_wks`
//!
//! ## Core behavior
//! - Default write mode is lossless (`WriteMode::Lossless`)
//! - Unknown tokens are captured on typed ASTs (`unknown_nodes`, `unknown_fields`)
//! - `write_mode(..., WriteMode::Canonical)` available for normalized output
//! - Version diagnostics produced post-parse for forward-compat signaling
//!
//! Evidence:
//! - Round-trip + unknown preservation tests:
//!   <https://github.com/Milind220/kiutils-rs/blob/main/crates/kiutils_kicad/tests/integration.rs>
//! - CLI contract tests (`kiutils-inspect`):
//!   <https://github.com/Milind220/kiutils-rs/blob/main/crates/kiutils_kicad/tests/inspect_cli.rs>
//!
//! ## Quickstart
//! ```rust,no_run
//! use kiutils_kicad::{SchematicFile, WriteMode};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let doc = SchematicFile::read("input.kicad_sch")?;
//!     doc.write_mode("output.kicad_sch", WriteMode::Lossless)?;
//!     Ok(())
//! }
//! ```
//!
//! Policy notes:
//! - AST `*_count` fields are convenience counters, not strict stability promises.
//! - Unknown-token diagnostics are developer-facing; summarize before showing end users.
//! - `.kicad_dru` rule conditions are preserved as strings in v1.
//!
//! ## Token alias policy
//!
//! KiCad file formats have evolved over major versions, sometimes renaming root
//! tokens or restructuring S-expression nodes. This crate follows a consistent
//! policy for handling these differences:
//!
//! | Format | Modern token | Legacy alias | Behavior |
//! |--------|-------------|--------------|----------|
//! | PCB | `kicad_pcb` | _(none)_ | Only modern accepted |
//! | Footprint | `footprint` | `module` | Both accepted; `legacy_root` diagnostic emitted |
//! | Schematic | `kicad_sch` | _(none)_ | Only modern accepted |
//! | Symbol lib | `kicad_symbol_lib` | _(none)_ | Only modern accepted |
//! | Worksheet | `kicad_wks` | `page_layout` | Both accepted; `legacy_root` diagnostic emitted |
//! | Design rules | `kicad_dru` | _(rootless)_ | Rootless format accepted |
//! | Lib tables | `fp_lib_table` / `sym_lib_table` | _(none)_ | Only modern accepted |
//!
//! ### Parser guarantees
//!
//! 1. **Lossless round-trip**: Writing a parsed file with `WriteMode::Lossless` produces
//!    byte-identical output to the original input, including whitespace and comments.
//! 2. **Unknown token preservation**: Any S-expression node not recognized by the typed
//!    AST parser is captured in `unknown_nodes` / `unknown_fields` vectors and preserved
//!    through round-trip writes.
//! 3. **Forward compatibility**: Files from newer KiCad versions parse without error;
//!    unrecognized tokens land in unknown vectors and a `future_format` diagnostic is
//!    emitted when the version number exceeds the known range.
//! 4. **Legacy compatibility**: Files using legacy root tokens (see table above) parse
//!    in compatibility mode with a `legacy_root` diagnostic to inform callers.
//! 5. **Typed AST is read-only over CST**: The typed AST is derived from the CST on parse.
//!    Mutations go through document setter APIs that modify the CST directly, then
//!    re-derive the AST, ensuring CST remains the source of truth.

mod batch;
mod diagnostic;
mod dru;
mod error;
mod footprint;
mod lib_table;
mod pcb;
mod project;
mod schematic;
mod sections;
mod sexpr_edit;
mod sexpr_utils;
mod symbol;
mod unknown;
mod version;
mod version_diag;
mod worksheet;
mod write_mode;

pub use batch::{read_pcbs, read_pcbs_from_refs};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use dru::{
    DesignRule, DesignRulesAst, DesignRulesDocument, DesignRulesFile, DruConstraint, DruSeverity,
};
pub use error::Error;
pub use footprint::{
    FootprintAst, FootprintDocument, FootprintFile, FpGraphic, FpGroup, FpModel, FpPad, FpPadDrill,
    FpPadNet, FpProperty, FpZone,
};
pub use lib_table::{
    FpLibTableAst, FpLibTableDocument, FpLibTableFile, LibTableKind, LibTableLibrary,
    SymLibTableAst, SymLibTableDocument, SymLibTableFile,
};
pub use pcb::{
    PcbArc, PcbAst, PcbDimension, PcbDocument, PcbFile, PcbFootprint, PcbFootprintModel,
    PcbGeneral, PcbGeneratedItem, PcbGraphic, PcbGroup, PcbImage, PcbLayer, PcbNet, PcbPad,
    PcbPadDrill, PcbPadNet, PcbPaper, PcbProperty, PcbSegment, PcbSetup, PcbTarget, PcbTitleBlock,
    PcbVia, PcbZone,
};
pub use project::{ProjectAst, ProjectDocument, ProjectExtra, ProjectFile};
pub use schematic::{
    SchematicArc, SchematicAst, SchematicBus, SchematicBusAlias, SchematicBusEntry,
    SchematicCircle, SchematicDocument, SchematicFile, SchematicImage, SchematicJunction,
    SchematicLabel, SchematicNetclassFlag, SchematicNoConnect, SchematicPaper, SchematicPolyline,
    SchematicRectangle, SchematicRuleArea, SchematicSheet, SchematicSheetInstance, SchematicSymbol,
    SchematicSymbolInfo, SchematicSymbolInstance, SchematicText, SchematicTitleBlock,
    SchematicWire,
};
pub use symbol::{
    SymGraphic, SymPin, SymProperty, SymUnit, Symbol, SymbolLibAst, SymbolLibDocument,
    SymbolLibFile,
};
pub use unknown::{UnknownField, UnknownNode};
pub use version::{KiCadSeries, VersionPolicy};
pub use worksheet::{
    WorksheetAst, WorksheetDocument, WorksheetFile, WorksheetSetup, WsBitmap, WsLine, WsPolygon,
    WsRect, WsTbtext,
};
pub use write_mode::WriteMode;

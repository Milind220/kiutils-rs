//! Typed KiCad file readers built on top of lossless S-expression CST parsing.
//!
//! Scope (v1):
//! - `.kicad_pcb`
//! - `.kicad_mod`
//! - `.kicad_sch`
//! - `.kicad_sym`
//! - `fp-lib-table`
//! - `.kicad_dru`
//! - `.kicad_pro`
//!
//! Default write mode is lossless:
//! parse -> modify typed AST -> write without regenerating unrelated formatting.
//!
//! Policy notes:
//! - AST `*_count` fields are debug-oriented convenience and are not stability guarantees.
//! - Unknown token diagnostics are primarily developer-facing; end-user tooling should summarize.
//! - Version compatibility checks run post-parse by default to maximize lossless ingestion.
//! - `.kicad_dru` rule conditions are preserved as strings in v1.

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
mod write_mode;

pub use batch::{read_pcbs, read_pcbs_from_refs};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use dru::{DesignRulesAst, DesignRulesDocument, DesignRulesFile};
pub use error::Error;
pub use footprint::{FootprintAst, FootprintDocument, FootprintFile};
pub use lib_table::{FpLibTableAst, FpLibTableDocument, FpLibTableFile};
pub use pcb::{
    PcbArcSummary, PcbAst, PcbDimensionSummary, PcbDocument, PcbFile, PcbFootprintSummary,
    PcbGeneratedSummary, PcbGraphicSummary, PcbGroupSummary, PcbLayer, PcbNet, PcbProperty,
    PcbSegmentSummary, PcbSetupSummary, PcbTargetSummary, PcbViaSummary, PcbZoneSummary,
};
pub use project::{ProjectAst, ProjectDocument, ProjectExtra, ProjectFile};
pub use schematic::{
    SchematicAst, SchematicDocument, SchematicFile, SchematicPaperSummary,
    SchematicTitleBlockSummary,
};
pub use symbol::{SymbolLibAst, SymbolLibDocument, SymbolLibFile, SymbolSummary};
pub use unknown::{UnknownField, UnknownNode};
pub use version::{KiCadSeries, VersionPolicy};
pub use write_mode::WriteMode;

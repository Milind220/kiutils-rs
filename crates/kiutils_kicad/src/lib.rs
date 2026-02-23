//! Typed KiCad file readers built on top of lossless S-expression CST parsing.
//!
//! Scope (v1):
//! - `.kicad_pcb`
//! - `.kicad_mod`
//! - `fp-lib-table`
//! - `.kicad_dru`
//! - `.kicad_pro`
//!
//! Default write mode is lossless:
//! parse -> modify typed AST -> write without regenerating unrelated formatting.

mod batch;
mod diagnostic;
mod dru;
mod error;
mod footprint;
mod lib_table;
mod unknown;
mod pcb;
mod project;
mod version;
mod write_mode;

pub use batch::{read_pcbs, read_pcbs_from_refs};
pub use diagnostic::{Diagnostic, Severity, Span};
pub use dru::{DesignRulesAst, DesignRulesDocument, DesignRulesFile};
pub use error::Error;
pub use footprint::{FootprintAst, FootprintDocument, FootprintFile};
pub use lib_table::{FpLibTableAst, FpLibTableDocument, FpLibTableFile};
pub use unknown::{UnknownField, UnknownNode};
pub use pcb::{
    PcbArcSummary, PcbAst, PcbDocument, PcbFile, PcbFootprintSummary, PcbLayer, PcbNet,
    PcbSegmentSummary, PcbViaSummary, PcbZoneSummary,
};
pub use project::{ProjectAst, ProjectDocument, ProjectExtra, ProjectFile};
pub use version::{KiCadSeries, VersionPolicy};
pub use write_mode::WriteMode;

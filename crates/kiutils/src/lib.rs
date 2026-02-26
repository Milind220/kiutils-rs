//! Public kiutils-rs API (v1 scope).
//!
//! Supported document families:
//! - `.kicad_pcb`
//! - `.kicad_mod`
//! - `fp-lib-table`
//! - `.kicad_dru`
//! - `.kicad_pro`
//!
//! Crate package name: `kiutils-rs`
//! Rust import path: `kiutils_rs`
//!
//! Compatibility target:
//! - Primary: KiCad v10
//! - Secondary: KiCad v9

pub use kiutils_kicad::{
    DesignRuleSummary, DesignRulesAst, DesignRulesDocument, DesignRulesFile, Diagnostic, Error,
    FootprintAst, FootprintDocument, FootprintFile, FpLibTableAst, FpLibTableDocument,
    FpLibTableFile, KiCadSeries, PcbArcSummary, PcbAst, PcbDimensionSummary, PcbDocument, PcbFile,
    PcbFootprintSummary, PcbGeneratedSummary, PcbGraphicSummary, PcbGroupSummary, PcbLayer, PcbNet,
    PcbProperty, PcbSegmentSummary, PcbSetupSummary, PcbTargetSummary, PcbViaSummary,
    PcbZoneSummary, ProjectAst, ProjectDocument, ProjectExtra, ProjectFile, Severity, Span,
    UnknownField, UnknownNode, VersionPolicy, WriteMode,
};

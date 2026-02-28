#![warn(missing_docs)]
//! # kiutils-rs
//!
//! **Pure-Rust, lossless KiCad S-expression document API for agent workflows.**
//! Designed for parse -> typed edit -> write with minimal diff noise and explicit forward-compat
//! surfaces.
//!
//! ## Why this crate (for AI agents and automation)
//! | Capability | `kiutils-rs` | Python `kiutils` | `KicadModTree` | KiCad IPC Rust API (`kicad-api-rs-official`) |
//! |---|---|---|---|---|
//! | File round-trip target | First-class (`WriteMode::Lossless`) | SCM-friendly goal, with known round-trip caveats in docs | Generator-centric (creates footprints; not general file parser API) | Runtime IPC API (different problem) |
//! | Unknown/future syntax | Exposed as typed unknown carriers (`UnknownNode`, `UnknownField`) | Known issues document format/ordering caveats in some cases | Focused on scripted footprint construction | N/A for S-expression file editing |
//! | Architecture for tooling | 3 layers (`kiutils-sexpr` -> `kiutils-kicad` -> `kiutils-rs`) | Single Python package/dataclass stack | Tree-based node generator framework | IPC client binding |
//! | Public tests/docs signal | Integration tests assert byte-equal lossless writes across formats | Project tests exist; changelog tracks format fixes over time | README/docs emphasize footprint generation workflows | README states docs are not yet available |
//!
//! Source evidence:
//! - This workspace architecture: <https://github.com/Milind220/kiutils-rs/blob/main/README.md>
//! - Lossless/unknown round-trip tests:
//!   <https://github.com/Milind220/kiutils-rs/blob/main/crates/kiutils_kicad/tests/integration.rs>
//! - Python `kiutils` known issues:
//!   <https://github.com/mvnmgrx/kiutils/blob/master/docs/misc/known-issues.rst>
//! - Python `kiutils` changelog:
//!   <https://github.com/mvnmgrx/kiutils/blob/master/CHANGELOG.md>
//! - `KicadModTree` repository README:
//!   <https://github.com/pointhi/kicad-footprint-generator/blob/master/README.md>
//! - Official KiCad Rust IPC README:
//!   <https://github.com/Milind220/kicad-api-rs-official/blob/main/README.md>
//!
//! ## V1 API scope
//! - `.kicad_pcb`
//! - `.kicad_mod`
//! - `fp-lib-table`
//! - `.kicad_dru`
//! - `.kicad_pro`
//!
//! Compatibility target:
//! - Primary: KiCad v10
//! - Secondary: KiCad v9
//!
//! ## Quickstart
//! ```rust,no_run
//! use kiutils_rs::{PcbFile, WriteMode};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut doc = PcbFile::read("input.kicad_pcb")?;
//!     doc.set_generator("kiutils-rs-agent")
//!         .set_generator_version("0.1.0")
//!         .upsert_property("Owner", "EDA-Agent");
//!     doc.write_mode("output.kicad_pcb", WriteMode::Lossless)?;
//!     Ok(())
//! }
//! ```
//!
//! Crate package name: `kiutils-rs`
//! Rust import path: `kiutils_rs`

/// Diagnostics, errors, and write behavior controls.
pub use kiutils_kicad::{
    Diagnostic, Error, KiCadSeries, Severity, Span, UnknownField, UnknownNode, VersionPolicy,
    WriteMode,
};

/// Design-rules (`.kicad_dru`) APIs.
pub use kiutils_kicad::{DesignRuleSummary, DesignRulesAst, DesignRulesDocument, DesignRulesFile};

/// Footprint (`.kicad_mod`) APIs.
pub use kiutils_kicad::{FootprintAst, FootprintDocument, FootprintFile};

/// Library table (`fp-lib-table`, `sym-lib-table`) APIs.
pub use kiutils_kicad::{
    FpLibTableAst, FpLibTableDocument, FpLibTableFile, SymLibTableAst, SymLibTableDocument,
    SymLibTableFile,
};

/// PCB (`.kicad_pcb`) APIs.
pub use kiutils_kicad::{
    PcbArcSummary, PcbAst, PcbDimensionSummary, PcbDocument, PcbFile, PcbFootprintSummary,
    PcbGeneratedSummary, PcbGraphicSummary, PcbGroupSummary, PcbLayer, PcbNet, PcbProperty,
    PcbSegmentSummary, PcbSetupSummary, PcbTargetSummary, PcbViaSummary, PcbZoneSummary,
};

/// Project (`.kicad_pro`) APIs.
pub use kiutils_kicad::{ProjectAst, ProjectDocument, ProjectExtra, ProjectFile};

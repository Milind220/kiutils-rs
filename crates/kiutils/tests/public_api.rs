use std::path::{Path, PathBuf};

use kiutils_rs::{
    DesignRulesFile, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, SchematicFile,
    SymLibTableFile, SymbolLibFile, WriteMode,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("kiutils_kicad")
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn facade_reads_all_v1_document_types() {
    let pcb = PcbFile::read(fixture("sample.kicad_pcb")).expect("pcb parse");
    assert_eq!(pcb.ast().version, Some(20260101));

    let footprint = FootprintFile::read(fixture("sample.kicad_mod")).expect("footprint parse");
    assert_eq!(footprint.ast().version, Some(20260101));

    let fplib = FpLibTableFile::read(fixture("fp-lib-table")).expect("fplib parse");
    assert_eq!(fplib.ast().library_count, 1);

    let symlib = SymLibTableFile::read(fixture("sym-lib-table")).expect("symlib parse");
    assert_eq!(symlib.ast().library_count, 1);

    let dru = DesignRulesFile::read(fixture("sample.kicad_dru")).expect("dru parse");
    assert_eq!(dru.ast().rule_count, 1);

    let project = ProjectFile::read(fixture("sample.kicad_pro")).expect("project parse");
    assert!(project.ast().pinned_symbol_libs.is_empty());
    assert_eq!(project.ast().pinned_footprint_libs, vec!["A"]);

    let schematic = SchematicFile::read(fixture("sample.kicad_sch")).expect("schematic parse");
    assert_eq!(schematic.ast().symbol_count, 1);

    let symbol = SymbolLibFile::read(fixture("sample.kicad_sym")).expect("symbol parse");
    assert_eq!(symbol.ast().symbol_count, 1);
}

#[test]
fn facade_exposes_write_mode() {
    assert_ne!(WriteMode::Lossless, WriteMode::Canonical);
}

#[test]
fn facade_exposes_project_setters_and_libtable_upsert() {
    let mut project = ProjectFile::read(fixture("sample.kicad_pro")).expect("project parse");
    project
        .set_pinned_symbol_libs(vec!["SYM_A"])
        .set_pinned_footprint_libs(vec!["FP_A"]);
    assert_eq!(project.ast().pinned_symbol_libs, vec!["SYM_A"]);
    assert_eq!(project.ast().pinned_footprint_libs, vec!["FP_A"]);

    let mut fplib = FpLibTableFile::read(fixture("fp-lib-table")).expect("fplib parse");
    fplib.upsert_library_uri("A", "${KIPRJMOD}/A.pretty");
    assert_eq!(
        fplib.ast().libraries[0].uri.as_deref(),
        Some("${KIPRJMOD}/A.pretty")
    );
}

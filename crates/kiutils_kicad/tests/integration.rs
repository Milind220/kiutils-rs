use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kiutils_kicad::{
    DesignRulesFile, Error, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, SymbolLibFile,
    WriteMode,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn tmp_file(name: &str, ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kiutils_kicad_{name}_{nanos}.{ext}"))
}

#[test]
fn pcb_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_pcb");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = PcbFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("pcb", "kicad_pcb");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn footprint_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_mod");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = FootprintFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("fp", "kicad_mod");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn libtable_fixture_unknown_and_canonical() {
    let src_path = fixture("fp-lib-table");

    let doc = FpLibTableFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().library_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("fplib", "table");
    doc.write_mode(&out, WriteMode::Canonical).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert!(got.contains("fp_lib_table"));

    let _ = fs::remove_file(out);
}

#[test]
fn dru_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_dru");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = DesignRulesFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().rule_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("dru", "kicad_dru");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn project_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_pro");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = ProjectFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().pinned_footprint_libs, vec!["A"]);
    assert_eq!(doc.ast().unknown_fields.len(), 1);

    let out = tmp_file("pro", "kicad_pro");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn symbol_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_sym");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = SymbolLibFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().symbol_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("sym", "kicad_sym");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn pcb_multi_unknown_roundtrip_lossless() {
    let src = "(kicad_pcb (version 20260101) (generator pcbnew) (mystery_a 1) (mystery_b \"x\"))\n";
    let path = tmp_file("pcb_multi_unknown", "kicad_pcb");
    fs::write(&path, src).expect("write fixture");

    let doc = PcbFile::read(&path).expect("parse");
    assert_eq!(doc.ast().unknown_nodes.len(), 2);

    let out = tmp_file("pcb_multi_unknown_out", "kicad_pcb");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(out);
}

#[test]
fn footprint_rejects_malformed_root() {
    let path = tmp_file("footprint_bad_root", "kicad_mod");
    fs::write(&path, "(foo \"R_0603\" (version 20260101))\n").expect("write fixture");

    let err = FootprintFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `footprint`")),
        other => panic!("unexpected error: {other}"),
    }

    let _ = fs::remove_file(path);
}

#[test]
fn symbol_rejects_malformed_root() {
    let path = tmp_file("symbol_bad_root", "kicad_sym");
    fs::write(&path, "(foo (version 20260101))\n").expect("write fixture");

    let err = SymbolLibFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `kicad_symbol_lib`")),
        other => panic!("unexpected error: {other}"),
    }

    let _ = fs::remove_file(path);
}

#[test]
fn fplib_rejects_malformed_root() {
    let path = tmp_file("fplib_bad_root", "table");
    fs::write(&path, "(sym_lib_table (version 7))\n").expect("write fixture");

    let err = FpLibTableFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `fp_lib_table`")),
        other => panic!("unexpected error: {other}"),
    }

    let _ = fs::remove_file(path);
}

#[test]
fn future_version_adds_diagnostic_for_pcb_and_footprint() {
    let pcb_path = tmp_file("pcb_future_diag", "kicad_pcb");
    fs::write(
        &pcb_path,
        "(kicad_pcb (version 20270101) (generator pcbnew))\n",
    )
    .expect("write pcb");
    let pcb_doc = PcbFile::read(&pcb_path).expect("parse pcb");
    assert_eq!(pcb_doc.diagnostics().len(), 1);
    assert_eq!(pcb_doc.diagnostics()[0].code, "future_format");

    let fp_path = tmp_file("fp_future_diag", "kicad_mod");
    fs::write(
        &fp_path,
        "(footprint \"R\" (version 20270101) (generator pcbnew))\n",
    )
    .expect("write footprint");
    let fp_doc = FootprintFile::read(&fp_path).expect("parse footprint");
    assert_eq!(fp_doc.diagnostics().len(), 1);
    assert_eq!(fp_doc.diagnostics()[0].code, "future_format");

    let _ = fs::remove_file(pcb_path);
    let _ = fs::remove_file(fp_path);
}

#[test]
fn pcb_accepts_quoted_atoms_for_numeric_and_text_fields() {
    let src = "(kicad_pcb (version \"20260101\") (generator \"pcbnew\") (layers (0 \"F.Cu\" \"signal\")) (net 1 \"GND\"))\n";
    let path = tmp_file("pcb_quoted_atoms", "kicad_pcb");
    fs::write(&path, src).expect("write fixture");

    let doc = PcbFile::read(&path).expect("parse");
    assert_eq!(doc.ast().version, Some(20260101));
    assert_eq!(doc.ast().generator.as_deref(), Some("pcbnew"));
    assert_eq!(
        doc.ast().layers.first().and_then(|l| l.name.as_deref()),
        Some("F.Cu")
    );
    assert_eq!(
        doc.ast()
            .layers
            .first()
            .and_then(|l| l.layer_type.as_deref()),
        Some("signal")
    );
    assert_eq!(
        doc.ast().nets.first().and_then(|n| n.name.as_deref()),
        Some("GND")
    );

    let _ = fs::remove_file(path);
}

#[test]
fn footprint_accepts_quoted_version_and_generator() {
    let src = "(footprint \"R\" (version \"20260101\") (generator \"pcbnew\") (layer \"F.Cu\") (property \"Reference\" \"R1\"))\n";
    let path = tmp_file("footprint_quoted_atoms", "kicad_mod");
    fs::write(&path, src).expect("write fixture");

    let doc = FootprintFile::read(&path).expect("parse");
    assert_eq!(doc.ast().version, Some(20260101));
    assert_eq!(doc.ast().generator.as_deref(), Some("pcbnew"));
    assert_eq!(doc.ast().layer.as_deref(), Some("F.Cu"));
    assert_eq!(doc.ast().property_count, 1);

    let _ = fs::remove_file(path);
}

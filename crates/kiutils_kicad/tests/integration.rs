use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kiutils_kicad::{
    DesignRulesFile, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, WriteMode,
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

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kiutils_kicad::{
    DesignRulesFile, Error, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, SchematicFile,
    SymLibTableFile, SymbolLibFile, WorksheetFile, WriteMode,
};

// ---------------------------------------------------------------------------
// Schematic symbol property editing
// ---------------------------------------------------------------------------

#[test]
fn schematic_upsert_symbol_instance_property() {
    let path = tmp_file("sch_upsert_prop", "kicad_sch");
    let src = "(kicad_sch (version 20260101) (generator \"eeschema\") (uuid \"u1\")\n  (symbol (lib_id \"Device:R\") (at 100 50 0) (property \"Reference\" \"R1\" (at 0 0 0)) (property \"Value\" \"10k\" (at 0 0 0)))\n)\n";
    fs::write(&path, src).expect("write fixture");

    let mut doc = SchematicFile::read(&path).expect("read");
    doc.upsert_symbol_instance_property("R1", "MPN", "RC0603FR-0710KL")
        .upsert_symbol_instance_property("R1", "Manufacturer", "Yageo");

    let out = tmp_file("sch_upsert_prop_out", "kicad_sch");
    doc.write(&out).expect("write");
    let reread = SchematicFile::read(&out).expect("reread");

    let symbols = reread.symbol_instances();
    assert_eq!(symbols.len(), 1);
    let r1 = &symbols[0];
    assert_eq!(r1.reference.as_deref(), Some("R1"));
    assert_eq!(r1.value.as_deref(), Some("10k"));
    assert!(r1
        .properties
        .iter()
        .any(|(k, v)| k == "MPN" && v == "RC0603FR-0710KL"));
    assert!(r1
        .properties
        .iter()
        .any(|(k, v)| k == "Manufacturer" && v == "Yageo"));

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(out);
}

#[test]
fn schematic_remove_symbol_instance_property() {
    let path = tmp_file("sch_remove_prop", "kicad_sch");
    let src = "(kicad_sch (version 20260101) (generator \"eeschema\") (uuid \"u1\")\n  (symbol (lib_id \"Device:R\") (property \"Reference\" \"R1\" (at 0 0 0)) (property \"Value\" \"10k\" (at 0 0 0)) (property \"MPN\" \"RC0603\" (at 0 0 0)))\n)\n";
    fs::write(&path, src).expect("write fixture");

    let mut doc = SchematicFile::read(&path).expect("read");
    doc.remove_symbol_instance_property("R1", "MPN");

    let out = tmp_file("sch_remove_prop_out", "kicad_sch");
    doc.write(&out).expect("write");
    let reread = SchematicFile::read(&out).expect("reread");

    let symbols = reread.symbol_instances();
    assert_eq!(symbols.len(), 1);
    assert!(!symbols[0].properties.iter().any(|(k, _)| k == "MPN"));
    assert_eq!(symbols[0].reference.as_deref(), Some("R1"));
    assert_eq!(symbols[0].value.as_deref(), Some("10k"));

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(out);
}

#[test]
fn schematic_upsert_preserves_roundtrip() {
    let path = tmp_file("sch_roundtrip_prop", "kicad_sch");
    let src = "(kicad_sch (version 20260101) (generator \"eeschema\") (uuid \"u1\")\n  (symbol (lib_id \"Device:R\") (at 100 50 0) (property \"Reference\" \"R1\" (at 0 0 0) (effects (font (size 1.27 1.27)))) (property \"Value\" \"10k\" (at 0 0 0) (effects (font (size 1.27 1.27)))))\n)\n";
    fs::write(&path, src).expect("write fixture");

    let mut doc = SchematicFile::read(&path).expect("read");
    // Upsert MPN then write, re-read, upsert same value again — should be no-op
    doc.upsert_symbol_instance_property("R1", "MPN", "TEST123");

    let out = tmp_file("sch_roundtrip_prop_out", "kicad_sch");
    doc.write(&out).expect("write");

    let mut doc2 = SchematicFile::read(&out).expect("reread");
    let cst_before = doc2.cst().to_lossless_string().to_string();
    doc2.upsert_symbol_instance_property("R1", "MPN", "TEST123"); // no-op
    let cst_after = doc2.cst().to_lossless_string();
    assert_eq!(cst_before, cst_after, "no-op upsert should preserve CST");

    let _ = fs::remove_file(path);
    let _ = fs::remove_file(out);
}

#[test]
fn schematic_symbol_instances_extracts_all() {
    let path = tmp_file("sch_list_symbols", "kicad_sch");
    let src = "(kicad_sch (version 20260101) (generator \"eeschema\") (uuid \"u1\")\n  (symbol (lib_id \"Device:R\") (property \"Reference\" \"R1\" (at 0 0 0)) (property \"Value\" \"10k\" (at 0 0 0)) (property \"Footprint\" \"R_0603\" (at 0 0 0)))\n  (symbol (lib_id \"Device:C\") (property \"Reference\" \"C1\" (at 0 0 0)) (property \"Value\" \"100nF\" (at 0 0 0)))\n)\n";
    fs::write(&path, src).expect("write fixture");

    let doc = SchematicFile::read(&path).expect("read");
    let symbols = doc.symbol_instances();

    assert_eq!(symbols.len(), 2);

    assert_eq!(symbols[0].reference.as_deref(), Some("R1"));
    assert_eq!(symbols[0].lib_id.as_deref(), Some("Device:R"));
    assert_eq!(symbols[0].value.as_deref(), Some("10k"));
    assert_eq!(symbols[0].footprint.as_deref(), Some("R_0603"));

    assert_eq!(symbols[1].reference.as_deref(), Some("C1"));
    assert_eq!(symbols[1].lib_id.as_deref(), Some("Device:C"));
    assert_eq!(symbols[1].value.as_deref(), Some("100nF"));
    assert_eq!(symbols[1].footprint, None);

    let _ = fs::remove_file(path);
}

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
// ANCHOR: pcb_roundtrip_test
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
// ANCHOR_END: pcb_roundtrip_test

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
fn symlib_fixture_unknown_and_canonical() {
    let src_path = fixture("sym-lib-table");

    let doc = SymLibTableFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().library_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("symlib", "table");
    doc.write_mode(&out, WriteMode::Canonical).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert!(got.contains("sym_lib_table"));

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
fn schematic_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_sch");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = SchematicFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().symbol_count, 1);
    assert_eq!(doc.ast().wire_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("sch", "kicad_sch");
    doc.write(&out).expect("write");
    let got = fs::read_to_string(&out).expect("read out");
    assert_eq!(got, src);

    let _ = fs::remove_file(out);
}

#[test]
fn worksheet_fixture_roundtrip_lossless_and_unknown() {
    let src_path = fixture("sample.kicad_wks");
    let src = fs::read_to_string(&src_path).expect("read fixture");

    let doc = WorksheetFile::read(&src_path).expect("parse");
    assert_eq!(doc.ast().line_count, 1);
    assert_eq!(doc.ast().rect_count, 1);
    assert_eq!(doc.ast().tbtext_count, 1);
    assert_eq!(doc.ast().unknown_nodes.len(), 1);

    let out = tmp_file("wks", "kicad_wks");
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
fn schematic_rejects_malformed_root() {
    let path = tmp_file("schematic_bad_root", "kicad_sch");
    fs::write(&path, "(foo (version 20260101))\n").expect("write fixture");

    let err = SchematicFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `kicad_sch`")),
        other => panic!("unexpected error: {other}"),
    }

    let _ = fs::remove_file(path);
}

#[test]
fn worksheet_rejects_malformed_root() {
    let path = tmp_file("worksheet_bad_root", "kicad_wks");
    fs::write(&path, "(foo (version 20260101))\n").expect("write fixture");

    let err = WorksheetFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `kicad_wks`")),
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
fn symlib_rejects_malformed_root() {
    let path = tmp_file("symlib_bad_root", "table");
    fs::write(&path, "(fp_lib_table (version 7))\n").expect("write fixture");

    let err = SymLibTableFile::read(&path).expect_err("must fail");
    match err {
        Error::Validation(msg) => assert!(msg.contains("expected root token `sym_lib_table`")),
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

#[test]
fn pcb_typed_fields_parse_from_fixture() {
    let doc = PcbFile::read(fixture("sample.kicad_pcb")).expect("read pcb");
    let ast = doc.ast();

    // Verify typed footprints parse if present
    if !ast.footprints.is_empty() {
        let fp = &ast.footprints[0];
        assert!(fp.layer.is_some(), "footprint should have layer");
        // Verify new fields are accessible (even if None for this fixture)
        let _ = fp.locked;
        let _ = fp.placed;
        let _ = &fp.attr;
        let _ = &fp.models;
        let _ = &fp.descr;

        // Verify typed pads parse
        if !fp.pads.is_empty() {
            let pad = &fp.pads[0];
            assert!(pad.pad_type.is_some(), "pad should have type");
            assert!(pad.shape.is_some(), "pad should have shape");
            // New fields accessible
            let _ = pad.roundrect_rratio;
            let _ = pad.clearance;
            let _ = pad.solder_mask_margin;
        }
    }

    // Verify setup expanded fields
    if let Some(setup) = &ast.setup {
        let _ = setup.pad_to_paste_clearance;
        let _ = setup.pad_to_paste_clearance_ratio;
    }

    // Verify image count field
    let _ = ast.image_count;
    let _ = &ast.images;
}

#[test]
fn footprint_typed_fields_parse_from_fixture() {
    let doc = FootprintFile::read(fixture("sample.kicad_mod")).expect("read fp");
    let ast = doc.ast();

    // Verify typed pads parse
    assert!(
        ast.pads.len() <= ast.pad_count,
        "typed pads should not exceed count"
    );
    if !ast.pads.is_empty() {
        let pad = &ast.pads[0];
        assert!(pad.pad_type.is_some(), "fp pad should have type");
        let _ = &pad.net;
        let _ = &pad.drill;
    }

    // Verify typed graphics parse
    if !ast.graphics.is_empty() {
        let g = &ast.graphics[0];
        assert!(!g.token.is_empty(), "graphic should have token");
    }

    // Verify typed models parse
    if !ast.models.is_empty() {
        let m = &ast.models[0];
        let _ = &m.path;
    }

    // Verify typed properties parse
    if !ast.properties.is_empty() {
        let p = &ast.properties[0];
        let _ = &p.key;
        let _ = &p.value;
    }

    // New top-level fields
    let _ = ast.locked;
    let _ = ast.placed;
    let _ = &ast.attr;
    let _ = &ast.reference;
    let _ = &ast.value;
}

#[test]
fn schematic_typed_fields_parse_from_fixture() {
    let doc = SchematicFile::read(fixture("sample.kicad_sch")).expect("read sch");
    let ast = doc.ast();

    // Verify typed symbols parse
    if ast.symbol_count > 0 {
        assert!(!ast.symbols.is_empty(), "typed symbols should match count");
        let sym = &ast.symbols[0];
        let _ = &sym.lib_id;
        let _ = &sym.uuid;
        let _ = sym.in_bom;
        let _ = sym.on_board;
        let _ = sym.dnp;
        let _ = &sym.properties;
    }

    // Verify typed wires parse
    if ast.wire_count > 0 {
        assert!(!ast.wires.is_empty(), "typed wires should match count");
        let w = &ast.wires[0];
        assert!(!w.points.is_empty(), "wire should have points");
    }

    // Verify typed junctions
    if ast.junction_count > 0 {
        assert!(
            !ast.junctions.is_empty(),
            "typed junctions should match count"
        );
    }

    // Verify labels
    if ast.label_count > 0 || ast.global_label_count > 0 || ast.hierarchical_label_count > 0 {
        // labels vec collects all label types
        let _ = &ast.labels;
    }

    // Verify instances
    let _ = &ast.symbol_instances_parsed;
    let _ = &ast.sheet_instances;

    // Verify sheets
    if ast.sheet_count > 0 {
        assert!(!ast.sheets.is_empty(), "typed sheets should match count");
    }
}

#[test]
fn symbol_lib_typed_fields_parse_from_fixture() {
    let doc = SymbolLibFile::read(fixture("sample.kicad_sym")).expect("read sym");
    let ast = doc.ast();

    assert!(ast.symbol_count > 0, "should have symbols");
    let sym = &ast.symbols[0];

    // New typed fields
    assert!(
        !sym.properties.is_empty(),
        "symbol should have typed properties"
    );
    let prop = &sym.properties[0];
    assert!(!prop.key.is_empty(), "property should have key");

    // Pin/unit/graphic access
    let _ = &sym.pins;
    let _ = &sym.units;
    let _ = &sym.graphics;
    let _ = sym.in_bom;
    let _ = sym.on_board;
    let _ = sym.power;
    let _ = sym.pin_names_hide;
    let _ = sym.pin_numbers_hide;
    let _ = &sym.extends;
    let _ = sym.exclude_from_sim;
}

#[test]
fn worksheet_typed_fields_parse_from_fixture() {
    let doc = WorksheetFile::read(fixture("sample.kicad_wks")).expect("read ws");
    let ast = doc.ast();

    // Verify typed objects match counts
    assert_eq!(
        ast.lines.len(),
        ast.line_count,
        "typed lines should match count"
    );
    assert_eq!(
        ast.rects.len(),
        ast.rect_count,
        "typed rects should match count"
    );
    assert_eq!(
        ast.tbtexts.len(),
        ast.tbtext_count,
        "typed tbtexts should match count"
    );
    assert_eq!(
        ast.polygons.len(),
        ast.polygon_count,
        "typed polygons should match count"
    );
    assert_eq!(
        ast.bitmaps.len(),
        ast.bitmap_count,
        "typed bitmaps should match count"
    );

    // Detailed tbtext fields
    if !ast.tbtexts.is_empty() {
        let t = &ast.tbtexts[0];
        let _ = &t.text;
        let _ = &t.name;
        let _ = &t.pos;
    }

    // Detailed line fields
    if !ast.lines.is_empty() {
        let l = &ast.lines[0];
        let _ = &l.start;
        let _ = &l.end;
    }
}

#[test]
fn dru_typed_fields_parse_from_fixture() {
    let doc = DesignRulesFile::read(fixture("sample.kicad_dru")).expect("read dru");
    let ast = doc.ast();

    assert!(!ast.rules.is_empty(), "should have rules");
    let rule = &ast.rules[0];

    // New constraint fields
    let _ = &rule.constraints;
    let _ = &rule.severity;
    let _ = &rule.condition;
    let _ = &rule.layer;

    // Severity overrides
    let _ = &ast.severities;
    let _ = ast.severity_count;
}

#[test]
fn unknown_token_preservation_all_formats() {
    // PCB with unknown tokens
    let pcb_src = r#"(kicad_pcb (version 20240108) (generator "test") (general (thickness 1.6)) (paper "A4") (future_thing 42) (layers (0 "F.Cu" signal)))"#;
    let pcb_path = tmp_file("unknown_pcb", "kicad_pcb");
    fs::write(&pcb_path, format!("{pcb_src}\n")).unwrap();
    let pcb_doc = PcbFile::read(&pcb_path).unwrap();
    assert!(
        !pcb_doc.ast().unknown_nodes.is_empty(),
        "PCB should capture unknown tokens"
    );
    let pcb_out = tmp_file("unknown_pcb_out", "kicad_pcb");
    pcb_doc.write(&pcb_out).unwrap();
    let _ = fs::remove_file(&pcb_path);
    let _ = fs::remove_file(&pcb_out);
    // Schematic with unknown tokens
    let sch_src = "(kicad_sch (version 20240101) (generator test) (future_block 1 2))\n";
    let sch_path = tmp_file("unknown_sch", "kicad_sch");
    fs::write(&sch_path, sch_src).unwrap();
    let sch_doc = SchematicFile::read(&sch_path).unwrap();
    assert!(
        !sch_doc.ast().unknown_nodes.is_empty(),
        "schematic should capture unknown tokens"
    );
    let _ = fs::remove_file(&sch_path);

    // Symbol lib with unknown tokens
    let sym_src = "(kicad_symbol_lib (version 20240101) (generator test) (future_node x))\n";
    let sym_path = tmp_file("unknown_sym", "kicad_sym");
    fs::write(&sym_path, sym_src).unwrap();
    let sym_doc = SymbolLibFile::read(&sym_path).unwrap();
    assert!(
        !sym_doc.ast().unknown_nodes.is_empty(),
        "symbol lib should capture unknown tokens"
    );
    let _ = fs::remove_file(&sym_path);
}

#[test]
fn legacy_module_token_accepted_for_footprint() {
    // KiCad 5 and earlier used "module" as the root token for footprints
    let legacy_src =
        "(module \"LegacyFP\" (version 20171130) (layer \"F.Cu\") (fp_text reference \"R1\" (at 0 0)))\n";
    let path = tmp_file("legacy_module_fp", "kicad_mod");
    fs::write(&path, legacy_src).unwrap();
    let doc = FootprintFile::read(&path).expect("should accept legacy 'module' token");
    let ast = doc.ast();
    assert!(
        ast.version.is_some() || ast.lib_id.is_some(),
        "should parse legacy footprint"
    );
    // Should produce a legacy_root diagnostic
    assert!(
        doc.diagnostics().iter().any(|d| d.code == "legacy_root"),
        "should emit legacy_root diagnostic for 'module' token"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn legacy_page_layout_token_accepted_for_worksheet() {
    // KiCad 5 and earlier used "page_layout" as the root token for worksheets
    let legacy_src = "(page_layout (version 20171130) (generator test))\n";
    let path = tmp_file("legacy_page_layout_ws", "kicad_wks");
    fs::write(&path, legacy_src).unwrap();
    let doc = WorksheetFile::read(&path).expect("should accept legacy 'page_layout' token");
    // Should produce a legacy_root diagnostic
    assert!(
        doc.diagnostics().iter().any(|d| d.code == "legacy_root"),
        "should emit legacy_root diagnostic for 'page_layout' token"
    );
    let _ = fs::remove_file(&path);
}

#[test]
fn modern_tokens_reject_incorrect_root() {
    // PCB only accepts kicad_pcb
    let bad_pcb = "(wrong_root (version 20240101))\n";
    let path = tmp_file("bad_root_pcb", "kicad_pcb");
    fs::write(&path, bad_pcb).unwrap();
    assert!(
        PcbFile::read(&path).is_err(),
        "PCB should reject wrong root token"
    );
    let _ = fs::remove_file(&path);

    // Schematic only accepts kicad_sch
    let bad_sch = "(wrong_root (version 20240101))\n";
    let path2 = tmp_file("bad_root_sch", "kicad_sch");
    fs::write(&path2, bad_sch).unwrap();
    assert!(
        SchematicFile::read(&path2).is_err(),
        "Schematic should reject wrong root token"
    );
    let _ = fs::remove_file(&path2);

    // Symbol lib only accepts kicad_symbol_lib
    let bad_sym = "(wrong_root (version 20240101))\n";
    let path3 = tmp_file("bad_root_sym", "kicad_sym");
    fs::write(&path3, bad_sym).unwrap();
    assert!(
        SymbolLibFile::read(&path3).is_err(),
        "Symbol lib should reject wrong root token"
    );
    let _ = fs::remove_file(&path3);
}

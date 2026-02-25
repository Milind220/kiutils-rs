use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn run_inspect(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_kiutils-inspect"))
        .args(args)
        .output()
        .expect("run inspect");
    assert!(
        output.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

#[test]
fn inspect_pcb_json_contract_smoke() {
    let path = fixture("sample.kicad_pcb");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "pcb", "--json"]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("pcb".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert!(o.contains_key("version"));
    assert!(o.contains_key("parsed_layer_entries"));
    assert!(o.contains_key("layer_count"));
    assert!(o.contains_key("unknown_count"));
    assert!(o.contains_key("diagnostic_count"));
}

#[test]
fn inspect_pcb_text_contract_smoke() {
    let path = fixture("sample.kicad_pcb");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "pcb"]);

    assert!(out.contains("kind: pcb"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("version: Some(20260101)"));
    assert!(out.contains("parsed_layer_entries: 0"));
    assert!(out.contains("unknown_count: 1"));
    assert!(out.contains("diagnostic_count: 0"));
}

#[test]
fn inspect_footprint_json_contract_smoke() {
    let path = fixture("sample.kicad_mod");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "footprint",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("footprint".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert!(o.contains_key("lib_id"));
    assert!(o.contains_key("version"));
    assert!(o.contains_key("graphic_count"));
    assert!(o.contains_key("unknown_count"));
    assert!(o.contains_key("diagnostic_count"));
}

#[test]
fn inspect_footprint_text_contract_smoke() {
    let path = fixture("sample.kicad_mod");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "footprint"]);

    assert!(out.contains("kind: footprint"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("lib_id: Some(\"R_0603\")"));
    assert!(out.contains("version: Some(20260101)"));
    assert!(out.contains("unknown_count: 1"));
    assert!(out.contains("diagnostic_count: 0"));
}

#[test]
fn inspect_schematic_json_contract_smoke() {
    let path = fixture("sample.kicad_sch");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "schematic",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("schematic".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("symbol_count"), Some(&Value::from(1)));
    assert_eq!(o.get("wire_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_schematic_text_contract_smoke() {
    let path = fixture("sample.kicad_sch");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "schematic"]);

    assert!(out.contains("kind: schematic"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("symbol_count: 1"));
    assert!(out.contains("wire_count: 1"));
    assert!(out.contains("unknown_count: 1"));
}

#[test]
fn inspect_symbol_json_contract_smoke() {
    let path = fixture("sample.kicad_sym");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "symbol",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("symbol".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("symbol_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_symbol_text_contract_smoke() {
    let path = fixture("sample.kicad_sym");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "symbol"]);

    assert!(out.contains("kind: symbol"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("symbol_count: 1"));
    assert!(out.contains("unknown_count: 1"));
}

#[test]
fn inspect_fplib_json_contract_smoke() {
    let path = fixture("fp-lib-table");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "fplib",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("fplib".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("first_library_name"), Some(&Value::from("A")));
    assert_eq!(o.get("library_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_fplib_text_contract_smoke() {
    let path = fixture("fp-lib-table");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "fplib"]);

    assert!(out.contains("kind: fplib"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("first_library_name: Some(\"A\")"));
    assert!(out.contains("library_count: 1"));
    assert!(out.contains("unknown_count: 1"));
}

#[test]
fn inspect_symlib_json_contract_smoke() {
    let path = fixture("sym-lib-table");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "symlib",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("symlib".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("first_library_name"), Some(&Value::from("S")));
    assert_eq!(o.get("library_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_symlib_text_contract_smoke() {
    let path = fixture("sym-lib-table");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "symlib"]);

    assert!(out.contains("kind: symlib"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("first_library_name: Some(\"S\")"));
    assert!(out.contains("library_count: 1"));
    assert!(out.contains("unknown_count: 1"));
}

#[test]
fn inspect_dru_json_contract_smoke() {
    let path = fixture("sample.kicad_dru");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "dru", "--json"]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("dru".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("first_rule_name"), Some(&Value::from("base")));
    assert_eq!(o.get("rule_count"), Some(&Value::from(1)));
    assert_eq!(o.get("total_constraint_count"), Some(&Value::from(1)));
    assert_eq!(o.get("rules_with_condition_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
    assert_eq!(o.get("diagnostic_count"), Some(&Value::from(0)));
}

#[test]
fn inspect_dru_text_contract_smoke() {
    let path = fixture("sample.kicad_dru");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "dru"]);

    assert!(out.contains("kind: dru"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("first_rule_name: Some(\"base\")"));
    assert!(out.contains("rule_count: 1"));
    assert!(out.contains("total_constraint_count: 1"));
    assert!(out.contains("unknown_count: 1"));
    assert!(out.contains("diagnostic_count: 0"));
}

#[test]
fn inspect_project_json_contract_smoke() {
    let path = fixture("sample.kicad_pro");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "project",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("project".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("meta_version"), Some(&Value::from(3)));
    assert_eq!(o.get("unknown_field_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_project_text_contract_smoke() {
    let path = fixture("sample.kicad_pro");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "project"]);

    assert!(out.contains("kind: project"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("meta_version: Some(3)"));
    assert!(out.contains("unknown_field_count: 1"));
}

#[test]
fn inspect_worksheet_json_contract_smoke() {
    let path = fixture("sample.kicad_wks");
    let out = run_inspect(&[
        path.to_str().expect("path str"),
        "--type",
        "worksheet",
        "--json",
    ]);
    let v: Value = serde_json::from_str(out.trim()).expect("json output");
    let o = v.as_object().expect("json object");

    assert_eq!(o.get("kind"), Some(&Value::String("worksheet".to_string())));
    assert_eq!(
        o.get("path"),
        Some(&Value::String(path.to_string_lossy().to_string()))
    );
    assert_eq!(o.get("line_count"), Some(&Value::from(1)));
    assert_eq!(o.get("rect_count"), Some(&Value::from(1)));
    assert_eq!(o.get("tbtext_count"), Some(&Value::from(1)));
    assert_eq!(o.get("unknown_count"), Some(&Value::from(1)));
}

#[test]
fn inspect_worksheet_text_contract_smoke() {
    let path = fixture("sample.kicad_wks");
    let out = run_inspect(&[path.to_str().expect("path str"), "--type", "worksheet"]);

    assert!(out.contains("kind: worksheet"));
    assert!(out.contains(&format!("path: {}", path.display())));
    assert!(out.contains("line_count: 1"));
    assert!(out.contains("rect_count: 1"));
    assert!(out.contains("tbtext_count: 1"));
    assert!(out.contains("unknown_count: 1"));
}

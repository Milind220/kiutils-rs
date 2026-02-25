use std::env;
use std::path::PathBuf;

use kiutils_kicad::DesignRulesFile;

fn usage() -> String {
    "usage: dru_roundtrip <input.kicad_dru> <output.kicad_dru>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = DesignRulesFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_version(1)
        .rename_first_rule("edited_rule")
        .upsert_rule_layer("edited_rule", "outer")
        .upsert_rule_condition("edited_rule", "A.NetClass == 'edited'");
    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = DesignRulesFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("rule_count: {}", reread.ast().rule_count);
    println!(
        "total_constraint_count: {}",
        reread.ast().total_constraint_count
    );
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

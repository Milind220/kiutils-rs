use std::env;
use std::path::PathBuf;

use kiutils_kicad::SymbolLibFile;

fn usage() -> String {
    "usage: symbol_roundtrip <input.kicad_sym> <output.kicad_sym>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = SymbolLibFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_generator("kiutils")
        .set_generator_version("roundtrip-demo");

    if let Some(first_name) = doc.ast().symbols.first().and_then(|s| s.name.clone()) {
        let renamed = format!("{first_name}_Edited");
        doc.rename_symbol(&first_name, renamed.clone())
            .upsert_symbol_property(
                &renamed,
                "EditedBy",
                "kiutils_kicad/examples/symbol_roundtrip.rs",
            );
    }

    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = SymbolLibFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("symbol_count: {}", reread.ast().symbol_count);
    println!("total_pin_count: {}", reread.ast().total_pin_count);
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

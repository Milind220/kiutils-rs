use std::env;
use std::path::PathBuf;

use kiutils_kicad::SchematicFile;

fn usage() -> String {
    "usage: schematic_roundtrip <input.kicad_sch> <output.kicad_sch>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = SchematicFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_generator("kiutils")
        .set_generator_version("roundtrip-demo")
        .set_title("Edited by kiutils")
        .set_embedded_fonts(true);
    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = SchematicFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("version: {:?}", reread.ast().version);
    println!("wire_count: {}", reread.ast().wire_count);
    println!("symbol_count: {}", reread.ast().symbol_count);
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

use std::env;
use std::path::PathBuf;

use kiutils_kicad::FootprintFile;

fn usage() -> String {
    "usage: footprint_roundtrip <input.kicad_mod> <output.kicad_mod>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = FootprintFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_generator("kiutils")
        .set_generator_version("roundtrip-demo")
        .upsert_property("EditedBy", "kiutils_kicad/examples/footprint_roundtrip.rs");

    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = FootprintFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("lib_id: {:?}", reread.ast().lib_id);
    println!("properties: {}", reread.ast().property_count);
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

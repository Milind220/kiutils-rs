use std::env;
use std::path::PathBuf;

use kiutils_kicad::PcbFile;

fn usage() -> String {
    "usage: pcb_roundtrip <input.kicad_pcb> <output.kicad_pcb>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(usage)?;
    let out_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(usage)?;

    let mut doc = PcbFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_generator("kiutils")
        .set_generator_version("roundtrip-demo")
        .set_title("Roundtrip Demo")
        .upsert_property("EditedBy", "kiutils_kicad/examples/pcb_roundtrip.rs");

    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = PcbFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("version: {:?}", reread.ast().version);
    println!("generator: {:?}", reread.ast().generator);
    println!("properties: {}", reread.ast().property_count);
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

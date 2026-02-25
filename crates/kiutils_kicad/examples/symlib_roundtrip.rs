use std::env;
use std::path::PathBuf;

use kiutils_kicad::SymLibTableFile;

fn usage() -> String {
    "usage: symlib_roundtrip <input_sym-lib-table> <output_sym-lib-table>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = SymLibTableFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_version(7)
        .rename_library("S", "S_EDITED")
        .add_library("Extra", "${KIPRJMOD}/Extra.kicad_sym");

    doc.write(&out_path).map_err(|e| e.to_string())?;
    let reread = SymLibTableFile::read(&out_path).map_err(|e| e.to_string())?;

    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("library_count: {}", reread.ast().library_count);
    println!(
        "first_library_name: {:?}",
        reread.ast().libraries.first().and_then(|l| l.name.clone())
    );
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());

    Ok(())
}

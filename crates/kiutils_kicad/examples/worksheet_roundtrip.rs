use std::env;
use std::path::PathBuf;

use kiutils_kicad::WorksheetFile;

fn usage() -> String {
    "usage: worksheet_roundtrip <input.kicad_wks> <output.kicad_wks>".to_string()
}

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let in_path = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let out_path = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut doc = WorksheetFile::read(&in_path).map_err(|e| e.to_string())?;
    doc.set_version(20260101)
        .set_generator("kiutils")
        .set_generator_version("roundtrip-demo")
        .set_setup_line_width(0.2);
    doc.write(&out_path).map_err(|e| e.to_string())?;

    let reread = WorksheetFile::read(&out_path).map_err(|e| e.to_string())?;
    println!("input: {}", in_path.display());
    println!("output: {}", out_path.display());
    println!("line_count: {}", reread.ast().line_count);
    println!("rect_count: {}", reread.ast().rect_count);
    println!("tbtext_count: {}", reread.ast().tbtext_count);
    println!("unknown_nodes: {}", reread.ast().unknown_nodes.len());
    println!("diagnostics: {}", reread.diagnostics().len());

    Ok(())
}

use std::path::PathBuf;

use kiutils_kicad::{
    DesignRulesFile, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, SchematicFile, WriteMode,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("data");

    let pcb = PcbFile::read(base.join("sample.kicad_pcb"))?;
    println!("pcb version: {:?}", pcb.ast().version);
    println!("pcb unknown nodes: {}", pcb.ast().unknown_nodes.len());

    let footprint = FootprintFile::read(base.join("sample.kicad_mod"))?;
    println!("footprint version: {:?}", footprint.ast().version);

    let schematic = SchematicFile::read(base.join("sample.kicad_sch"))?;
    println!("schematic version: {:?}", schematic.ast().version);
    println!(
        "schematic unknown nodes: {}",
        schematic.ast().unknown_nodes.len()
    );

    let table = FpLibTableFile::read(base.join("fp-lib-table"))?;
    println!("fp libs: {}", table.ast().library_count);

    let dru = DesignRulesFile::read(base.join("sample.kicad_dru"))?;
    println!("rules: {}", dru.ast().rule_count);

    let project = ProjectFile::read(base.join("sample.kicad_pro"))?;
    println!("project meta version: {:?}", project.ast().meta_version);

    pcb.write_mode("/tmp/out.kicad_pcb", WriteMode::Lossless)?;
    Ok(())
}

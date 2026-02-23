use std::env;
use std::path::{Path, PathBuf};

use kiutils_kicad::{
    DesignRulesFile, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, WriteMode,
};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Auto,
    Pcb,
    Footprint,
    FpLibTable,
    Dru,
    Project,
}

#[derive(Debug, Clone)]
struct Opts {
    path: PathBuf,
    kind: Kind,
    as_json: bool,
    show_cst: bool,
    show_canonical: bool,
    show_unknown: bool,
    show_diagnostics: bool,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let opts = parse_args(env::args().skip(1).collect())?;

    let kind = if opts.kind == Kind::Auto {
        detect_kind(&opts.path).ok_or_else(|| {
            "could not infer file type; pass --type pcb|footprint|fplib|dru|project".to_string()
        })?
    } else {
        opts.kind
    };

    match kind {
        Kind::Pcb => inspect_pcb(&opts),
        Kind::Footprint => inspect_footprint(&opts),
        Kind::FpLibTable => inspect_fplib(&opts),
        Kind::Dru => inspect_dru(&opts),
        Kind::Project => inspect_project(&opts),
        Kind::Auto => Err("internal: unresolved auto kind".to_string()),
    }
}

fn parse_args(args: Vec<String>) -> Result<Opts, String> {
    if args.is_empty() {
        return Err(usage());
    }

    let mut kind = Kind::Auto;
    let mut as_json = false;
    let mut show_cst = false;
    let mut show_canonical = false;
    let mut show_unknown = false;
    let mut show_diagnostics = false;
    let mut path: Option<PathBuf> = None;

    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => return Err(usage()),
            "--json" => as_json = true,
            "--show-cst" => show_cst = true,
            "--show-canonical" => show_canonical = true,
            "--show-unknown" => show_unknown = true,
            "--show-diagnostics" => show_diagnostics = true,
            "--type" => {
                i += 1;
                if i >= args.len() {
                    return Err("--type needs a value".to_string());
                }
                kind = parse_kind(&args[i])?;
            }
            _ if arg.starts_with('-') => return Err(format!("unknown flag: {arg}")),
            _ => {
                if path.is_some() {
                    return Err("multiple paths provided".to_string());
                }
                path = Some(PathBuf::from(arg));
            }
        }
        i += 1;
    }

    let path = path.ok_or_else(usage)?;

    Ok(Opts {
        path,
        kind,
        as_json,
        show_cst,
        show_canonical,
        show_unknown,
        show_diagnostics,
    })
}

fn parse_kind(v: &str) -> Result<Kind, String> {
    match v {
        "auto" => Ok(Kind::Auto),
        "pcb" => Ok(Kind::Pcb),
        "footprint" => Ok(Kind::Footprint),
        "fplib" => Ok(Kind::FpLibTable),
        "dru" => Ok(Kind::Dru),
        "project" => Ok(Kind::Project),
        _ => Err(format!("invalid --type: {v}")),
    }
}

fn detect_kind(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    match ext {
        "kicad_pcb" => Some(Kind::Pcb),
        "kicad_mod" => Some(Kind::Footprint),
        "kicad_dru" => Some(Kind::Dru),
        "kicad_pro" => Some(Kind::Project),
        _ if name == "fp-lib-table" => Some(Kind::FpLibTable),
        _ => None,
    }
}

fn inspect_pcb(opts: &Opts) -> Result<(), String> {
    let doc = PcbFile::read(&opts.path).map_err(|e| e.to_string())?;
    if opts.as_json {
        println!(
            "{}",
            json!({
                "kind": "pcb",
                "path": opts.path,
                "version": doc.ast().version,
                "generator": doc.ast().generator,
                "generator_version": doc.ast().generator_version,
                "parsed_layer_entries": doc.ast().layers.len(),
                "parsed_net_entries": doc.ast().nets.len(),
                "parsed_footprint_entries": doc.ast().footprints.len(),
                "parsed_segment_entries": doc.ast().segments.len(),
                "parsed_arc_entries": doc.ast().arcs.len(),
                "parsed_via_entries": doc.ast().vias.len(),
                "parsed_zone_entries": doc.ast().zones.len(),
                "first_layer": doc.ast().layers.first().and_then(|l| l.name.clone()),
                "first_net": doc.ast().nets.first().and_then(|n| n.name.clone()),
                "first_footprint_lib_id": doc.ast().footprints.first().and_then(|f| f.lib_id.clone()),
                "first_footprint_ref": doc.ast().footprints.first().and_then(|f| f.reference.clone()),
                "first_segment_layer": doc.ast().segments.first().and_then(|s| s.layer.clone()),
                "first_zone_net_name": doc.ast().zones.first().and_then(|z| z.net_name.clone()),
                "layer_count": doc.ast().layer_count,
                "property_count": doc.ast().property_count,
                "net_count": doc.ast().net_count,
                "footprint_count": doc.ast().footprint_count,
                "graphic_count": doc.ast().graphic_count,
                "gr_line_count": doc.ast().gr_line_count,
                "gr_rect_count": doc.ast().gr_rect_count,
                "gr_circle_count": doc.ast().gr_circle_count,
                "gr_arc_count": doc.ast().gr_arc_count,
                "gr_poly_count": doc.ast().gr_poly_count,
                "gr_curve_count": doc.ast().gr_curve_count,
                "gr_text_count": doc.ast().gr_text_count,
                "gr_text_box_count": doc.ast().gr_text_box_count,
                "trace_segment_count": doc.ast().trace_segment_count,
                "trace_arc_count": doc.ast().trace_arc_count,
                "via_count": doc.ast().via_count,
                "zone_count": doc.ast().zone_count,
                "dimension_count": doc.ast().dimension_count,
                "target_count": doc.ast().target_count,
                "group_count": doc.ast().group_count,
                "generated_count": doc.ast().generated_count,
                "unknown_count": doc.ast().unknown_nodes.len(),
                "diagnostic_count": doc.diagnostics().len(),
            })
        );
    } else {
        println!("kind: pcb");
        println!("path: {}", opts.path.display());
        println!("version: {:?}", doc.ast().version);
        println!("generator: {:?}", doc.ast().generator);
        println!("generator_version: {:?}", doc.ast().generator_version);
        println!("parsed_layer_entries: {}", doc.ast().layers.len());
        println!("parsed_net_entries: {}", doc.ast().nets.len());
        println!("parsed_footprint_entries: {}", doc.ast().footprints.len());
        println!("parsed_segment_entries: {}", doc.ast().segments.len());
        println!("parsed_arc_entries: {}", doc.ast().arcs.len());
        println!("parsed_via_entries: {}", doc.ast().vias.len());
        println!("parsed_zone_entries: {}", doc.ast().zones.len());
        println!(
            "first_layer: {:?}",
            doc.ast().layers.first().and_then(|l| l.name.clone())
        );
        println!(
            "first_net: {:?}",
            doc.ast().nets.first().and_then(|n| n.name.clone())
        );
        println!(
            "first_footprint_lib_id: {:?}",
            doc.ast().footprints.first().and_then(|f| f.lib_id.clone())
        );
        println!(
            "first_footprint_ref: {:?}",
            doc.ast().footprints.first().and_then(|f| f.reference.clone())
        );
        println!(
            "first_segment_layer: {:?}",
            doc.ast().segments.first().and_then(|s| s.layer.clone())
        );
        println!(
            "first_zone_net_name: {:?}",
            doc.ast().zones.first().and_then(|z| z.net_name.clone())
        );
        println!("layer_count: {}", doc.ast().layer_count);
        println!("property_count: {}", doc.ast().property_count);
        println!("net_count: {}", doc.ast().net_count);
        println!("footprint_count: {}", doc.ast().footprint_count);
        println!("graphic_count: {}", doc.ast().graphic_count);
        println!("gr_line_count: {}", doc.ast().gr_line_count);
        println!("gr_rect_count: {}", doc.ast().gr_rect_count);
        println!("gr_circle_count: {}", doc.ast().gr_circle_count);
        println!("gr_arc_count: {}", doc.ast().gr_arc_count);
        println!("gr_poly_count: {}", doc.ast().gr_poly_count);
        println!("gr_curve_count: {}", doc.ast().gr_curve_count);
        println!("gr_text_count: {}", doc.ast().gr_text_count);
        println!("gr_text_box_count: {}", doc.ast().gr_text_box_count);
        println!("trace_segment_count: {}", doc.ast().trace_segment_count);
        println!("trace_arc_count: {}", doc.ast().trace_arc_count);
        println!("via_count: {}", doc.ast().via_count);
        println!("zone_count: {}", doc.ast().zone_count);
        println!("dimension_count: {}", doc.ast().dimension_count);
        println!("target_count: {}", doc.ast().target_count);
        println!("group_count: {}", doc.ast().group_count);
        println!("generated_count: {}", doc.ast().generated_count);
        println!("unknown_count: {}", doc.ast().unknown_nodes.len());
        println!("diagnostic_count: {}", doc.diagnostics().len());
    }

    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!("unknown: head={:?} span={}..{}", n.head, n.span.start, n.span.end);
        }
    }
    if opts.show_diagnostics {
        for d in doc.diagnostics() {
            println!("diagnostic: [{:?}] {} {}", d.severity, d.code, d.message);
        }
    }
    if opts.show_cst {
        println!("--- cst (lossless) ---\n{}", doc.cst().to_lossless_string());
    }
    if opts.show_canonical {
        let out = temp_out("inspect_pcb", "kicad_pcb");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn inspect_footprint(opts: &Opts) -> Result<(), String> {
    let doc = FootprintFile::read(&opts.path).map_err(|e| e.to_string())?;
    if opts.as_json {
        println!(
            "{}",
            json!({
                "kind": "footprint",
                "path": opts.path,
                "version": doc.ast().version,
                "unknown_count": doc.ast().unknown_nodes.len(),
                "diagnostic_count": doc.diagnostics().len(),
            })
        );
    } else {
        println!("kind: footprint");
        println!("path: {}", opts.path.display());
        println!("version: {:?}", doc.ast().version);
        println!("unknown_count: {}", doc.ast().unknown_nodes.len());
        println!("diagnostic_count: {}", doc.diagnostics().len());
    }
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!("unknown: head={:?} span={}..{}", n.head, n.span.start, n.span.end);
        }
    }
    if opts.show_diagnostics {
        for d in doc.diagnostics() {
            println!("diagnostic: [{:?}] {} {}", d.severity, d.code, d.message);
        }
    }
    if opts.show_cst {
        println!("--- cst (lossless) ---\n{}", doc.cst().to_lossless_string());
    }
    if opts.show_canonical {
        let out = temp_out("inspect_fp", "kicad_mod");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn inspect_fplib(opts: &Opts) -> Result<(), String> {
    let doc = FpLibTableFile::read(&opts.path).map_err(|e| e.to_string())?;
    if opts.as_json {
        println!(
            "{}",
            json!({
                "kind": "fplib",
                "path": opts.path,
                "library_count": doc.ast().library_count,
                "unknown_count": doc.ast().unknown_nodes.len(),
            })
        );
    } else {
        println!("kind: fplib");
        println!("path: {}", opts.path.display());
        println!("library_count: {}", doc.ast().library_count);
        println!("unknown_count: {}", doc.ast().unknown_nodes.len());
    }
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!("unknown: head={:?} span={}..{}", n.head, n.span.start, n.span.end);
        }
    }
    if opts.show_cst {
        println!("--- cst (lossless) ---\n{}", doc.cst().to_lossless_string());
    }
    if opts.show_canonical {
        let out = temp_out("inspect_fplib", "table");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn inspect_dru(opts: &Opts) -> Result<(), String> {
    let doc = DesignRulesFile::read(&opts.path).map_err(|e| e.to_string())?;
    if opts.as_json {
        println!(
            "{}",
            json!({
                "kind": "dru",
                "path": opts.path,
                "version": doc.ast().version,
                "rule_count": doc.ast().rule_count,
                "unknown_count": doc.ast().unknown_nodes.len(),
            })
        );
    } else {
        println!("kind: dru");
        println!("path: {}", opts.path.display());
        println!("version: {:?}", doc.ast().version);
        println!("rule_count: {}", doc.ast().rule_count);
        println!("unknown_count: {}", doc.ast().unknown_nodes.len());
    }
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!("unknown: head={:?} span={}..{}", n.head, n.span.start, n.span.end);
        }
    }
    if opts.show_cst {
        println!("--- cst (lossless) ---\n{}", doc.cst().to_lossless_string());
    }
    if opts.show_canonical {
        let out = temp_out("inspect_dru", "kicad_dru");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn inspect_project(opts: &Opts) -> Result<(), String> {
    let doc = ProjectFile::read(&opts.path).map_err(|e| e.to_string())?;
    if opts.as_json {
        println!(
            "{}",
            json!({
                "kind": "project",
                "path": opts.path,
                "meta_version": doc.ast().meta_version,
                "pinned_footprint_libs": doc.ast().pinned_footprint_libs,
                "unknown_field_count": doc.ast().unknown_fields.len(),
            })
        );
    } else {
        println!("kind: project");
        println!("path: {}", opts.path.display());
        println!("meta_version: {:?}", doc.ast().meta_version);
        println!(
            "pinned_footprint_libs: {:?}",
            doc.ast().pinned_footprint_libs
        );
        println!("unknown_field_count: {}", doc.ast().unknown_fields.len());
    }
    if opts.show_unknown {
        for f in &doc.ast().unknown_fields {
            println!("unknown_field: key={} value={}", f.key, f.value);
        }
    }
    if opts.show_cst {
        println!("--- json (lossless) ---\n{}", doc.raw());
    }
    if opts.show_canonical {
        let out = temp_out("inspect_pro", "kicad_pro");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn temp_out(prefix: &str, ext: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{nanos}.{ext}"))
}

fn usage() -> String {
    "usage: kiutils-inspect <path> [--type auto|pcb|footprint|fplib|dru|project] [--json] [--show-cst] [--show-canonical] [--show-unknown] [--show-diagnostics]".to_string()
}

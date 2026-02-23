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
        let mut m = serde_json::Map::new();
        m.insert("kind".into(), json!("pcb"));
        m.insert("path".into(), json!(opts.path));
        m.insert("version".into(), json!(doc.ast().version));
        m.insert("generator".into(), json!(doc.ast().generator));
        m.insert("generator_version".into(), json!(doc.ast().generator_version));
        m.insert(
            "parsed_property_entries".into(),
            json!(doc.ast().properties.len()),
        );
        m.insert("parsed_layer_entries".into(), json!(doc.ast().layers.len()));
        m.insert("parsed_net_entries".into(), json!(doc.ast().nets.len()));
        m.insert("parsed_footprint_entries".into(), json!(doc.ast().footprints.len()));
        m.insert("parsed_segment_entries".into(), json!(doc.ast().segments.len()));
        m.insert("parsed_arc_entries".into(), json!(doc.ast().arcs.len()));
        m.insert("parsed_via_entries".into(), json!(doc.ast().vias.len()));
        m.insert("parsed_zone_entries".into(), json!(doc.ast().zones.len()));
        m.insert("parsed_generated_entries".into(), json!(doc.ast().generated_items.len()));
        m.insert("parsed_dimension_entries".into(), json!(doc.ast().dimensions.len()));
        m.insert("parsed_target_entries".into(), json!(doc.ast().targets.len()));
        m.insert("parsed_group_entries".into(), json!(doc.ast().groups.len()));
        m.insert("parsed_graphic_entries".into(), json!(doc.ast().graphics.len()));
        m.insert(
            "first_layer".into(),
            json!(doc.ast().layers.first().and_then(|l| l.name.clone())),
        );
        m.insert(
            "first_net".into(),
            json!(doc.ast().nets.first().and_then(|n| n.name.clone())),
        );
        m.insert(
            "first_footprint_lib_id".into(),
            json!(doc.ast().footprints.first().and_then(|f| f.lib_id.clone())),
        );
        m.insert(
            "first_footprint_ref".into(),
            json!(doc.ast().footprints.first().and_then(|f| f.reference.clone())),
        );
        m.insert(
            "first_segment_layer".into(),
            json!(doc.ast().segments.first().and_then(|s| s.layer.clone())),
        );
        m.insert(
            "first_zone_net_name".into(),
            json!(doc.ast().zones.first().and_then(|z| z.net_name.clone())),
        );
        m.insert(
            "first_zone_layer".into(),
            json!(doc.ast().zones.first().and_then(|z| z.layer.clone())),
        );
        m.insert(
            "first_zone_layers_len".into(),
            json!(doc.ast().zones.first().map(|z| z.layers.len())),
        );
        m.insert(
            "first_zone_fill_enabled".into(),
            json!(doc.ast().zones.first().and_then(|z| z.fill_enabled)),
        );
        m.insert(
            "first_generated_type".into(),
            json!(doc.ast().generated_items.first().and_then(|g| g.generated_type.clone())),
        );
        m.insert(
            "first_generated_last_netname".into(),
            json!(doc.ast().generated_items.first().and_then(|g| g.last_netname.clone())),
        );
        m.insert(
            "first_dimension_type".into(),
            json!(doc.ast().dimensions.first().and_then(|d| d.dimension_type.clone())),
        );
        m.insert(
            "first_target_shape".into(),
            json!(doc.ast().targets.first().and_then(|t| t.shape.clone())),
        );
        m.insert(
            "first_group_member_count".into(),
            json!(doc.ast().groups.first().map(|g| g.member_count)),
        );
        m.insert(
            "first_graphic_token".into(),
            json!(doc.ast().graphics.first().map(|g| g.token.clone())),
        );
        m.insert(
            "first_graphic_layer".into(),
            json!(doc.ast().graphics.first().and_then(|g| g.layer.clone())),
        );
        m.insert(
            "setup_has_stackup".into(),
            json!(doc.ast().setup.as_ref().map(|s| s.has_stackup)),
        );
        m.insert(
            "setup_stackup_layer_count".into(),
            json!(doc.ast().setup.as_ref().map(|s| s.stackup_layer_count)),
        );
        m.insert(
            "setup_has_plot_settings".into(),
            json!(doc.ast().setup.as_ref().map(|s| s.has_plot_settings)),
        );
        m.insert(
                "setup_pad_to_mask_clearance".into(),
                json!(doc.ast().setup.as_ref().and_then(|s| s.pad_to_mask_clearance)),
            );
        m.insert(
            "has_embedded_files".into(),
            json!(doc.ast().has_embedded_files),
        );
        m.insert(
            "embedded_file_count".into(),
            json!(doc.ast().embedded_file_count),
        );
        m.insert("layer_count".into(), json!(doc.ast().layer_count));
        m.insert("property_count".into(), json!(doc.ast().property_count));
        m.insert("net_count".into(), json!(doc.ast().net_count));
        m.insert("footprint_count".into(), json!(doc.ast().footprint_count));
        m.insert("graphic_count".into(), json!(doc.ast().graphic_count));
        m.insert("gr_line_count".into(), json!(doc.ast().gr_line_count));
        m.insert("gr_rect_count".into(), json!(doc.ast().gr_rect_count));
        m.insert("gr_circle_count".into(), json!(doc.ast().gr_circle_count));
        m.insert("gr_arc_count".into(), json!(doc.ast().gr_arc_count));
        m.insert("gr_poly_count".into(), json!(doc.ast().gr_poly_count));
        m.insert("gr_curve_count".into(), json!(doc.ast().gr_curve_count));
        m.insert("gr_text_count".into(), json!(doc.ast().gr_text_count));
        m.insert("gr_text_box_count".into(), json!(doc.ast().gr_text_box_count));
        m.insert("trace_segment_count".into(), json!(doc.ast().trace_segment_count));
        m.insert("trace_arc_count".into(), json!(doc.ast().trace_arc_count));
        m.insert("via_count".into(), json!(doc.ast().via_count));
        m.insert("zone_count".into(), json!(doc.ast().zone_count));
        m.insert("dimension_count".into(), json!(doc.ast().dimension_count));
        m.insert("target_count".into(), json!(doc.ast().target_count));
        m.insert("group_count".into(), json!(doc.ast().group_count));
        m.insert("generated_count".into(), json!(doc.ast().generated_count));
        m.insert("unknown_count".into(), json!(doc.ast().unknown_nodes.len()));
        m.insert("diagnostic_count".into(), json!(doc.diagnostics().len()));
        println!("{}", serde_json::Value::Object(m));
    } else {
        println!("kind: pcb");
        println!("path: {}", opts.path.display());
        println!("version: {:?}", doc.ast().version);
        println!("generator: {:?}", doc.ast().generator);
        println!("generator_version: {:?}", doc.ast().generator_version);
        println!("parsed_property_entries: {}", doc.ast().properties.len());
        println!("parsed_layer_entries: {}", doc.ast().layers.len());
        println!("parsed_net_entries: {}", doc.ast().nets.len());
        println!("parsed_footprint_entries: {}", doc.ast().footprints.len());
        println!("parsed_segment_entries: {}", doc.ast().segments.len());
        println!("parsed_arc_entries: {}", doc.ast().arcs.len());
        println!("parsed_via_entries: {}", doc.ast().vias.len());
        println!("parsed_zone_entries: {}", doc.ast().zones.len());
        println!("parsed_generated_entries: {}", doc.ast().generated_items.len());
        println!("parsed_dimension_entries: {}", doc.ast().dimensions.len());
        println!("parsed_target_entries: {}", doc.ast().targets.len());
        println!("parsed_group_entries: {}", doc.ast().groups.len());
        println!("parsed_graphic_entries: {}", doc.ast().graphics.len());
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
        println!(
            "first_zone_layer: {:?}",
            doc.ast().zones.first().and_then(|z| z.layer.clone())
        );
        println!(
            "first_zone_layers_len: {:?}",
            doc.ast().zones.first().map(|z| z.layers.len())
        );
        println!(
            "first_zone_fill_enabled: {:?}",
            doc.ast().zones.first().and_then(|z| z.fill_enabled)
        );
        println!(
            "first_generated_type: {:?}",
            doc.ast()
                .generated_items
                .first()
                .and_then(|g| g.generated_type.clone())
        );
        println!(
            "first_generated_last_netname: {:?}",
            doc.ast()
                .generated_items
                .first()
                .and_then(|g| g.last_netname.clone())
        );
        println!(
            "first_dimension_type: {:?}",
            doc.ast()
                .dimensions
                .first()
                .and_then(|d| d.dimension_type.clone())
        );
        println!(
            "first_target_shape: {:?}",
            doc.ast().targets.first().and_then(|t| t.shape.clone())
        );
        println!(
            "first_group_member_count: {:?}",
            doc.ast().groups.first().map(|g| g.member_count)
        );
        println!(
            "first_graphic_token: {:?}",
            doc.ast().graphics.first().map(|g| g.token.clone())
        );
        println!(
            "first_graphic_layer: {:?}",
            doc.ast().graphics.first().and_then(|g| g.layer.clone())
        );
        println!(
            "setup_has_stackup: {:?}",
            doc.ast().setup.as_ref().map(|s| s.has_stackup)
        );
        println!(
            "setup_stackup_layer_count: {:?}",
            doc.ast().setup.as_ref().map(|s| s.stackup_layer_count)
        );
        println!(
            "setup_has_plot_settings: {:?}",
            doc.ast().setup.as_ref().map(|s| s.has_plot_settings)
        );
        println!(
            "setup_pad_to_mask_clearance: {:?}",
            doc.ast().setup.as_ref().and_then(|s| s.pad_to_mask_clearance)
        );
        println!("has_embedded_files: {}", doc.ast().has_embedded_files);
        println!("embedded_file_count: {}", doc.ast().embedded_file_count);
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
                "lib_id": doc.ast().lib_id,
                "version": doc.ast().version,
                "generator": doc.ast().generator,
                "generator_version": doc.ast().generator_version,
                "layer": doc.ast().layer,
                "descr": doc.ast().descr,
                "tags": doc.ast().tags,
                "property_count": doc.ast().property_count,
                "attr_present": doc.ast().attr_present,
                "locked_present": doc.ast().locked_present,
                "private_layers_present": doc.ast().private_layers_present,
                "net_tie_pad_groups_present": doc.ast().net_tie_pad_groups_present,
                "embedded_fonts_present": doc.ast().embedded_fonts_present,
                "has_embedded_files": doc.ast().has_embedded_files,
                "embedded_file_count": doc.ast().embedded_file_count,
                "solder_mask_margin": doc.ast().solder_mask_margin,
                "solder_paste_margin": doc.ast().solder_paste_margin,
                "solder_paste_margin_ratio": doc.ast().solder_paste_margin_ratio,
                "duplicate_pad_numbers_are_jumpers": doc.ast().duplicate_pad_numbers_are_jumpers,
                "pad_count": doc.ast().pad_count,
                "model_count": doc.ast().model_count,
                "zone_count": doc.ast().zone_count,
                "group_count": doc.ast().group_count,
                "graphic_count": doc.ast().graphic_count,
                "fp_line_count": doc.ast().fp_line_count,
                "fp_rect_count": doc.ast().fp_rect_count,
                "fp_circle_count": doc.ast().fp_circle_count,
                "fp_arc_count": doc.ast().fp_arc_count,
                "fp_poly_count": doc.ast().fp_poly_count,
                "fp_curve_count": doc.ast().fp_curve_count,
                "fp_text_count": doc.ast().fp_text_count,
                "fp_text_box_count": doc.ast().fp_text_box_count,
                "unknown_count": doc.ast().unknown_nodes.len(),
                "diagnostic_count": doc.diagnostics().len(),
            })
        );
    } else {
        println!("kind: footprint");
        println!("path: {}", opts.path.display());
        println!("lib_id: {:?}", doc.ast().lib_id);
        println!("version: {:?}", doc.ast().version);
        println!("generator: {:?}", doc.ast().generator);
        println!("generator_version: {:?}", doc.ast().generator_version);
        println!("layer: {:?}", doc.ast().layer);
        println!("descr: {:?}", doc.ast().descr);
        println!("tags: {:?}", doc.ast().tags);
        println!("property_count: {}", doc.ast().property_count);
        println!("attr_present: {}", doc.ast().attr_present);
        println!("locked_present: {}", doc.ast().locked_present);
        println!(
            "private_layers_present: {}",
            doc.ast().private_layers_present
        );
        println!(
            "net_tie_pad_groups_present: {}",
            doc.ast().net_tie_pad_groups_present
        );
        println!("embedded_fonts_present: {}", doc.ast().embedded_fonts_present);
        println!("has_embedded_files: {}", doc.ast().has_embedded_files);
        println!("embedded_file_count: {}", doc.ast().embedded_file_count);
        println!("solder_mask_margin: {:?}", doc.ast().solder_mask_margin);
        println!("solder_paste_margin: {:?}", doc.ast().solder_paste_margin);
        println!(
            "solder_paste_margin_ratio: {:?}",
            doc.ast().solder_paste_margin_ratio
        );
        println!(
            "duplicate_pad_numbers_are_jumpers: {:?}",
            doc.ast().duplicate_pad_numbers_are_jumpers
        );
        println!("pad_count: {}", doc.ast().pad_count);
        println!("model_count: {}", doc.ast().model_count);
        println!("zone_count: {}", doc.ast().zone_count);
        println!("group_count: {}", doc.ast().group_count);
        println!("graphic_count: {}", doc.ast().graphic_count);
        println!("fp_line_count: {}", doc.ast().fp_line_count);
        println!("fp_rect_count: {}", doc.ast().fp_rect_count);
        println!("fp_circle_count: {}", doc.ast().fp_circle_count);
        println!("fp_arc_count: {}", doc.ast().fp_arc_count);
        println!("fp_poly_count: {}", doc.ast().fp_poly_count);
        println!("fp_curve_count: {}", doc.ast().fp_curve_count);
        println!("fp_text_count: {}", doc.ast().fp_text_count);
        println!("fp_text_box_count: {}", doc.ast().fp_text_box_count);
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

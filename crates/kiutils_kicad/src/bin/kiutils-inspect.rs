use std::env;
use std::path::{Path, PathBuf};

use kiutils_kicad::{
    DesignRulesFile, FootprintFile, FpLibTableFile, PcbFile, ProjectFile, SchematicFile,
    SymbolLibFile, WriteMode,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Auto,
    Pcb,
    Footprint,
    Schematic,
    Symbol,
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
            "could not infer file type; pass --type pcb|footprint|schematic|symbol|fplib|dru|project".to_string()
        })?
    } else {
        opts.kind
    };

    match kind {
        Kind::Pcb => inspect_pcb(&opts),
        Kind::Footprint => inspect_footprint(&opts),
        Kind::Schematic => inspect_schematic(&opts),
        Kind::Symbol => inspect_symbol(&opts),
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
        "schematic" | "sch" => Ok(Kind::Schematic),
        "symbol" => Ok(Kind::Symbol),
        "fplib" => Ok(Kind::FpLibTable),
        "dru" => Ok(Kind::Dru),
        "project" => Ok(Kind::Project),
        _ => Err(format!("invalid --type: {v}")),
    }
}

fn detect_kind(path: &Path) -> Option<Kind> {
    let name = path.file_name()?.to_str()?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    match ext {
        "kicad_pcb" => Some(Kind::Pcb),
        "kicad_mod" => Some(Kind::Footprint),
        "kicad_sch" => Some(Kind::Schematic),
        "kicad_sym" => Some(Kind::Symbol),
        "kicad_dru" => Some(Kind::Dru),
        "kicad_pro" => Some(Kind::Project),
        _ if name == "fp-lib-table" => Some(Kind::FpLibTable),
        _ => None,
    }
}

struct InspectField {
    key: &'static str,
    json: Value,
    text: String,
}

fn field(key: &'static str, json: Value, text: String) -> InspectField {
    InspectField { key, json, text }
}

fn emit_fields(kind: &str, path: &Path, fields: &[InspectField], as_json: bool) {
    if as_json {
        let mut m = serde_json::Map::new();
        m.insert("kind".into(), json!(kind));
        m.insert("path".into(), json!(path));
        for f in fields {
            m.insert(f.key.into(), f.json.clone());
        }
        println!("{}", Value::Object(m));
    } else {
        println!("kind: {kind}");
        println!("path: {}", path.display());
        for f in fields {
            println!("{}: {}", f.key, f.text);
        }
    }
}

fn pcb_fields(doc: &kiutils_kicad::PcbDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field(
            "generator",
            json!(ast.generator),
            format!("{:?}", ast.generator),
        ),
        field(
            "generator_version",
            json!(ast.generator_version),
            format!("{:?}", ast.generator_version),
        ),
        field(
            "parsed_property_entries",
            json!(ast.properties.len()),
            ast.properties.len().to_string(),
        ),
        field(
            "parsed_layer_entries",
            json!(ast.layers.len()),
            ast.layers.len().to_string(),
        ),
        field(
            "parsed_net_entries",
            json!(ast.nets.len()),
            ast.nets.len().to_string(),
        ),
        field(
            "parsed_footprint_entries",
            json!(ast.footprints.len()),
            ast.footprints.len().to_string(),
        ),
        field(
            "parsed_segment_entries",
            json!(ast.segments.len()),
            ast.segments.len().to_string(),
        ),
        field(
            "parsed_arc_entries",
            json!(ast.arcs.len()),
            ast.arcs.len().to_string(),
        ),
        field(
            "parsed_via_entries",
            json!(ast.vias.len()),
            ast.vias.len().to_string(),
        ),
        field(
            "parsed_zone_entries",
            json!(ast.zones.len()),
            ast.zones.len().to_string(),
        ),
        field(
            "parsed_generated_entries",
            json!(ast.generated_items.len()),
            ast.generated_items.len().to_string(),
        ),
        field(
            "parsed_dimension_entries",
            json!(ast.dimensions.len()),
            ast.dimensions.len().to_string(),
        ),
        field(
            "parsed_target_entries",
            json!(ast.targets.len()),
            ast.targets.len().to_string(),
        ),
        field(
            "parsed_group_entries",
            json!(ast.groups.len()),
            ast.groups.len().to_string(),
        ),
        field(
            "parsed_graphic_entries",
            json!(ast.graphics.len()),
            ast.graphics.len().to_string(),
        ),
        field(
            "first_layer",
            json!(ast.layers.first().and_then(|l| l.name.clone())),
            format!("{:?}", ast.layers.first().and_then(|l| l.name.clone())),
        ),
        field(
            "first_net",
            json!(ast.nets.first().and_then(|n| n.name.clone())),
            format!("{:?}", ast.nets.first().and_then(|n| n.name.clone())),
        ),
        field(
            "first_footprint_lib_id",
            json!(ast.footprints.first().and_then(|f| f.lib_id.clone())),
            format!(
                "{:?}",
                ast.footprints.first().and_then(|f| f.lib_id.clone())
            ),
        ),
        field(
            "first_footprint_ref",
            json!(ast.footprints.first().and_then(|f| f.reference.clone())),
            format!(
                "{:?}",
                ast.footprints.first().and_then(|f| f.reference.clone())
            ),
        ),
        field(
            "first_footprint_uuid",
            json!(ast.footprints.first().and_then(|f| f.uuid.clone())),
            format!("{:?}", ast.footprints.first().and_then(|f| f.uuid.clone())),
        ),
        field(
            "first_footprint_rotation",
            json!(ast.footprints.first().and_then(|f| f.rotation)),
            format!("{:?}", ast.footprints.first().and_then(|f| f.rotation)),
        ),
        field(
            "first_footprint_pad_count",
            json!(ast.footprints.first().map(|f| f.pad_count)),
            format!("{:?}", ast.footprints.first().map(|f| f.pad_count)),
        ),
        field(
            "first_segment_layer",
            json!(ast.segments.first().and_then(|s| s.layer.clone())),
            format!("{:?}", ast.segments.first().and_then(|s| s.layer.clone())),
        ),
        field(
            "first_segment_uuid",
            json!(ast.segments.first().and_then(|s| s.uuid.clone())),
            format!("{:?}", ast.segments.first().and_then(|s| s.uuid.clone())),
        ),
        field(
            "first_segment_locked",
            json!(ast.segments.first().map(|s| s.locked)),
            format!("{:?}", ast.segments.first().map(|s| s.locked)),
        ),
        field(
            "first_via_uuid",
            json!(ast.vias.first().and_then(|v| v.uuid.clone())),
            format!("{:?}", ast.vias.first().and_then(|v| v.uuid.clone())),
        ),
        field(
            "first_via_drill_shape",
            json!(ast.vias.first().and_then(|v| v.drill_shape.clone())),
            format!("{:?}", ast.vias.first().and_then(|v| v.drill_shape.clone())),
        ),
        field(
            "first_via_locked",
            json!(ast.vias.first().map(|v| v.locked)),
            format!("{:?}", ast.vias.first().map(|v| v.locked)),
        ),
        field(
            "first_zone_net_name",
            json!(ast.zones.first().and_then(|z| z.net_name.clone())),
            format!("{:?}", ast.zones.first().and_then(|z| z.net_name.clone())),
        ),
        field(
            "first_zone_layer",
            json!(ast.zones.first().and_then(|z| z.layer.clone())),
            format!("{:?}", ast.zones.first().and_then(|z| z.layer.clone())),
        ),
        field(
            "first_zone_layers_len",
            json!(ast.zones.first().map(|z| z.layers.len())),
            format!("{:?}", ast.zones.first().map(|z| z.layers.len())),
        ),
        field(
            "first_zone_fill_enabled",
            json!(ast.zones.first().and_then(|z| z.fill_enabled)),
            format!("{:?}", ast.zones.first().and_then(|z| z.fill_enabled)),
        ),
        field(
            "first_generated_type",
            json!(ast
                .generated_items
                .first()
                .and_then(|g| g.generated_type.clone())),
            format!(
                "{:?}",
                ast.generated_items
                    .first()
                    .and_then(|g| g.generated_type.clone())
            ),
        ),
        field(
            "first_generated_last_netname",
            json!(ast
                .generated_items
                .first()
                .and_then(|g| g.last_netname.clone())),
            format!(
                "{:?}",
                ast.generated_items
                    .first()
                    .and_then(|g| g.last_netname.clone())
            ),
        ),
        field(
            "first_dimension_type",
            json!(ast
                .dimensions
                .first()
                .and_then(|d| d.dimension_type.clone())),
            format!(
                "{:?}",
                ast.dimensions
                    .first()
                    .and_then(|d| d.dimension_type.clone())
            ),
        ),
        field(
            "first_target_shape",
            json!(ast.targets.first().and_then(|t| t.shape.clone())),
            format!("{:?}", ast.targets.first().and_then(|t| t.shape.clone())),
        ),
        field(
            "first_group_member_count",
            json!(ast.groups.first().map(|g| g.member_count)),
            format!("{:?}", ast.groups.first().map(|g| g.member_count)),
        ),
        field(
            "first_graphic_token",
            json!(ast.graphics.first().map(|g| g.token.clone())),
            format!("{:?}", ast.graphics.first().map(|g| g.token.clone())),
        ),
        field(
            "first_graphic_layer",
            json!(ast.graphics.first().and_then(|g| g.layer.clone())),
            format!("{:?}", ast.graphics.first().and_then(|g| g.layer.clone())),
        ),
        field(
            "first_graphic_uuid",
            json!(ast.graphics.first().and_then(|g| g.uuid.clone())),
            format!("{:?}", ast.graphics.first().and_then(|g| g.uuid.clone())),
        ),
        field(
            "first_graphic_locked",
            json!(ast.graphics.first().map(|g| g.locked)),
            format!("{:?}", ast.graphics.first().map(|g| g.locked)),
        ),
        field(
            "setup_has_stackup",
            json!(ast.setup.as_ref().map(|s| s.has_stackup)),
            format!("{:?}", ast.setup.as_ref().map(|s| s.has_stackup)),
        ),
        field(
            "general_thickness",
            json!(ast.general.as_ref().and_then(|g| g.thickness)),
            format!("{:?}", ast.general.as_ref().and_then(|g| g.thickness)),
        ),
        field(
            "paper_kind",
            json!(ast.paper.as_ref().and_then(|p| p.kind.clone())),
            format!("{:?}", ast.paper.as_ref().and_then(|p| p.kind.clone())),
        ),
        field(
            "title_block_title",
            json!(ast.title_block.as_ref().and_then(|t| t.title.clone())),
            format!(
                "{:?}",
                ast.title_block.as_ref().and_then(|t| t.title.clone())
            ),
        ),
        field(
            "setup_stackup_layer_count",
            json!(ast.setup.as_ref().map(|s| s.stackup_layer_count)),
            format!("{:?}", ast.setup.as_ref().map(|s| s.stackup_layer_count)),
        ),
        field(
            "setup_has_plot_settings",
            json!(ast.setup.as_ref().map(|s| s.has_plot_settings)),
            format!("{:?}", ast.setup.as_ref().map(|s| s.has_plot_settings)),
        ),
        field(
            "setup_pad_to_mask_clearance",
            json!(ast.setup.as_ref().and_then(|s| s.pad_to_mask_clearance)),
            format!(
                "{:?}",
                ast.setup.as_ref().and_then(|s| s.pad_to_mask_clearance)
            ),
        ),
        field(
            "has_embedded_files",
            json!(ast.has_embedded_files),
            ast.has_embedded_files.to_string(),
        ),
        field(
            "embedded_file_count",
            json!(ast.embedded_file_count),
            ast.embedded_file_count.to_string(),
        ),
        field(
            "layer_count",
            json!(ast.layer_count),
            ast.layer_count.to_string(),
        ),
        field(
            "property_count",
            json!(ast.property_count),
            ast.property_count.to_string(),
        ),
        field("net_count", json!(ast.net_count), ast.net_count.to_string()),
        field(
            "footprint_count",
            json!(ast.footprint_count),
            ast.footprint_count.to_string(),
        ),
        field(
            "graphic_count",
            json!(ast.graphic_count),
            ast.graphic_count.to_string(),
        ),
        field(
            "gr_line_count",
            json!(ast.gr_line_count),
            ast.gr_line_count.to_string(),
        ),
        field(
            "gr_rect_count",
            json!(ast.gr_rect_count),
            ast.gr_rect_count.to_string(),
        ),
        field(
            "gr_circle_count",
            json!(ast.gr_circle_count),
            ast.gr_circle_count.to_string(),
        ),
        field(
            "gr_arc_count",
            json!(ast.gr_arc_count),
            ast.gr_arc_count.to_string(),
        ),
        field(
            "gr_poly_count",
            json!(ast.gr_poly_count),
            ast.gr_poly_count.to_string(),
        ),
        field(
            "gr_curve_count",
            json!(ast.gr_curve_count),
            ast.gr_curve_count.to_string(),
        ),
        field(
            "gr_text_count",
            json!(ast.gr_text_count),
            ast.gr_text_count.to_string(),
        ),
        field(
            "gr_text_box_count",
            json!(ast.gr_text_box_count),
            ast.gr_text_box_count.to_string(),
        ),
        field(
            "trace_segment_count",
            json!(ast.trace_segment_count),
            ast.trace_segment_count.to_string(),
        ),
        field(
            "trace_arc_count",
            json!(ast.trace_arc_count),
            ast.trace_arc_count.to_string(),
        ),
        field("via_count", json!(ast.via_count), ast.via_count.to_string()),
        field(
            "zone_count",
            json!(ast.zone_count),
            ast.zone_count.to_string(),
        ),
        field(
            "dimension_count",
            json!(ast.dimension_count),
            ast.dimension_count.to_string(),
        ),
        field(
            "target_count",
            json!(ast.target_count),
            ast.target_count.to_string(),
        ),
        field(
            "group_count",
            json!(ast.group_count),
            ast.group_count.to_string(),
        ),
        field(
            "generated_count",
            json!(ast.generated_count),
            ast.generated_count.to_string(),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
        field(
            "diagnostic_count",
            json!(doc.diagnostics().len()),
            doc.diagnostics().len().to_string(),
        ),
    ]
}

fn inspect_pcb(opts: &Opts) -> Result<(), String> {
    let doc = PcbFile::read(&opts.path).map_err(|e| e.to_string())?;
    let fields = pcb_fields(&doc);
    emit_fields("pcb", &opts.path, &fields, opts.as_json);

    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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
    let fields = footprint_fields(&doc);
    emit_fields("footprint", &opts.path, &fields, opts.as_json);
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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

fn footprint_fields(doc: &kiutils_kicad::FootprintDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("lib_id", json!(ast.lib_id), format!("{:?}", ast.lib_id)),
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field("tedit", json!(ast.tedit), format!("{:?}", ast.tedit)),
        field(
            "generator",
            json!(ast.generator),
            format!("{:?}", ast.generator),
        ),
        field(
            "generator_version",
            json!(ast.generator_version),
            format!("{:?}", ast.generator_version),
        ),
        field("layer", json!(ast.layer), format!("{:?}", ast.layer)),
        field("descr", json!(ast.descr), format!("{:?}", ast.descr)),
        field("tags", json!(ast.tags), format!("{:?}", ast.tags)),
        field(
            "property_count",
            json!(ast.property_count),
            ast.property_count.to_string(),
        ),
        field(
            "attr_present",
            json!(ast.attr_present),
            ast.attr_present.to_string(),
        ),
        field(
            "locked_present",
            json!(ast.locked_present),
            ast.locked_present.to_string(),
        ),
        field(
            "private_layers_present",
            json!(ast.private_layers_present),
            ast.private_layers_present.to_string(),
        ),
        field(
            "net_tie_pad_groups_present",
            json!(ast.net_tie_pad_groups_present),
            ast.net_tie_pad_groups_present.to_string(),
        ),
        field(
            "embedded_fonts_present",
            json!(ast.embedded_fonts_present),
            ast.embedded_fonts_present.to_string(),
        ),
        field(
            "has_embedded_files",
            json!(ast.has_embedded_files),
            ast.has_embedded_files.to_string(),
        ),
        field(
            "embedded_file_count",
            json!(ast.embedded_file_count),
            ast.embedded_file_count.to_string(),
        ),
        field(
            "clearance",
            json!(ast.clearance),
            format!("{:?}", ast.clearance),
        ),
        field(
            "solder_mask_margin",
            json!(ast.solder_mask_margin),
            format!("{:?}", ast.solder_mask_margin),
        ),
        field(
            "solder_paste_margin",
            json!(ast.solder_paste_margin),
            format!("{:?}", ast.solder_paste_margin),
        ),
        field(
            "solder_paste_margin_ratio",
            json!(ast.solder_paste_margin_ratio),
            format!("{:?}", ast.solder_paste_margin_ratio),
        ),
        field(
            "duplicate_pad_numbers_are_jumpers",
            json!(ast.duplicate_pad_numbers_are_jumpers),
            format!("{:?}", ast.duplicate_pad_numbers_are_jumpers),
        ),
        field("pad_count", json!(ast.pad_count), ast.pad_count.to_string()),
        field(
            "model_count",
            json!(ast.model_count),
            ast.model_count.to_string(),
        ),
        field(
            "zone_count",
            json!(ast.zone_count),
            ast.zone_count.to_string(),
        ),
        field(
            "group_count",
            json!(ast.group_count),
            ast.group_count.to_string(),
        ),
        field(
            "dimension_count",
            json!(ast.dimension_count),
            ast.dimension_count.to_string(),
        ),
        field(
            "graphic_count",
            json!(ast.graphic_count),
            ast.graphic_count.to_string(),
        ),
        field(
            "fp_line_count",
            json!(ast.fp_line_count),
            ast.fp_line_count.to_string(),
        ),
        field(
            "fp_rect_count",
            json!(ast.fp_rect_count),
            ast.fp_rect_count.to_string(),
        ),
        field(
            "fp_circle_count",
            json!(ast.fp_circle_count),
            ast.fp_circle_count.to_string(),
        ),
        field(
            "fp_arc_count",
            json!(ast.fp_arc_count),
            ast.fp_arc_count.to_string(),
        ),
        field(
            "fp_poly_count",
            json!(ast.fp_poly_count),
            ast.fp_poly_count.to_string(),
        ),
        field(
            "fp_curve_count",
            json!(ast.fp_curve_count),
            ast.fp_curve_count.to_string(),
        ),
        field(
            "fp_text_count",
            json!(ast.fp_text_count),
            ast.fp_text_count.to_string(),
        ),
        field(
            "fp_text_box_count",
            json!(ast.fp_text_box_count),
            ast.fp_text_box_count.to_string(),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
        field(
            "diagnostic_count",
            json!(doc.diagnostics().len()),
            doc.diagnostics().len().to_string(),
        ),
    ]
}

fn inspect_schematic(opts: &Opts) -> Result<(), String> {
    let doc = SchematicFile::read(&opts.path).map_err(|e| e.to_string())?;
    let fields = schematic_fields(&doc);
    emit_fields("schematic", &opts.path, &fields, opts.as_json);
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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
        let out = temp_out("inspect_sch", "kicad_sch");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn schematic_fields(doc: &kiutils_kicad::SchematicDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field(
            "generator",
            json!(ast.generator),
            format!("{:?}", ast.generator),
        ),
        field(
            "generator_version",
            json!(ast.generator_version),
            format!("{:?}", ast.generator_version),
        ),
        field("uuid", json!(ast.uuid), format!("{:?}", ast.uuid)),
        field("has_paper", json!(ast.has_paper), ast.has_paper.to_string()),
        field(
            "paper_kind",
            json!(ast.paper.as_ref().and_then(|p| p.kind.clone())),
            format!("{:?}", ast.paper.as_ref().and_then(|p| p.kind.clone())),
        ),
        field(
            "paper_width",
            json!(ast.paper.as_ref().and_then(|p| p.width)),
            format!("{:?}", ast.paper.as_ref().and_then(|p| p.width)),
        ),
        field(
            "paper_height",
            json!(ast.paper.as_ref().and_then(|p| p.height)),
            format!("{:?}", ast.paper.as_ref().and_then(|p| p.height)),
        ),
        field(
            "paper_orientation",
            json!(ast.paper.as_ref().and_then(|p| p.orientation.clone())),
            format!(
                "{:?}",
                ast.paper.as_ref().and_then(|p| p.orientation.clone())
            ),
        ),
        field(
            "title",
            json!(ast.title_block.as_ref().and_then(|t| t.title.clone())),
            format!(
                "{:?}",
                ast.title_block.as_ref().and_then(|t| t.title.clone())
            ),
        ),
        field(
            "date",
            json!(ast.title_block.as_ref().and_then(|t| t.date.clone())),
            format!(
                "{:?}",
                ast.title_block.as_ref().and_then(|t| t.date.clone())
            ),
        ),
        field(
            "revision",
            json!(ast.title_block.as_ref().and_then(|t| t.revision.clone())),
            format!(
                "{:?}",
                ast.title_block.as_ref().and_then(|t| t.revision.clone())
            ),
        ),
        field(
            "company",
            json!(ast.title_block.as_ref().and_then(|t| t.company.clone())),
            format!(
                "{:?}",
                ast.title_block.as_ref().and_then(|t| t.company.clone())
            ),
        ),
        field(
            "title_comment_count",
            json!(ast
                .title_block
                .as_ref()
                .map(|t| t.comments.len())
                .unwrap_or(0)),
            ast.title_block
                .as_ref()
                .map(|t| t.comments.len())
                .unwrap_or(0)
                .to_string(),
        ),
        field(
            "has_title_block",
            json!(ast.has_title_block),
            ast.has_title_block.to_string(),
        ),
        field(
            "has_lib_symbols",
            json!(ast.has_lib_symbols),
            ast.has_lib_symbols.to_string(),
        ),
        field(
            "embedded_fonts",
            json!(ast.embedded_fonts),
            format!("{:?}", ast.embedded_fonts),
        ),
        field(
            "lib_symbol_count",
            json!(ast.lib_symbol_count),
            ast.lib_symbol_count.to_string(),
        ),
        field(
            "symbol_count",
            json!(ast.symbol_count),
            ast.symbol_count.to_string(),
        ),
        field(
            "sheet_count",
            json!(ast.sheet_count),
            ast.sheet_count.to_string(),
        ),
        field(
            "junction_count",
            json!(ast.junction_count),
            ast.junction_count.to_string(),
        ),
        field(
            "no_connect_count",
            json!(ast.no_connect_count),
            ast.no_connect_count.to_string(),
        ),
        field(
            "bus_entry_count",
            json!(ast.bus_entry_count),
            ast.bus_entry_count.to_string(),
        ),
        field(
            "bus_alias_count",
            json!(ast.bus_alias_count),
            ast.bus_alias_count.to_string(),
        ),
        field(
            "wire_count",
            json!(ast.wire_count),
            ast.wire_count.to_string(),
        ),
        field("bus_count", json!(ast.bus_count), ast.bus_count.to_string()),
        field(
            "text_count",
            json!(ast.text_count),
            ast.text_count.to_string(),
        ),
        field(
            "text_box_count",
            json!(ast.text_box_count),
            ast.text_box_count.to_string(),
        ),
        field(
            "image_count",
            json!(ast.image_count),
            ast.image_count.to_string(),
        ),
        field(
            "label_count",
            json!(ast.label_count),
            ast.label_count.to_string(),
        ),
        field(
            "global_label_count",
            json!(ast.global_label_count),
            ast.global_label_count.to_string(),
        ),
        field(
            "hierarchical_label_count",
            json!(ast.hierarchical_label_count),
            ast.hierarchical_label_count.to_string(),
        ),
        field(
            "netclass_flag_count",
            json!(ast.netclass_flag_count),
            ast.netclass_flag_count.to_string(),
        ),
        field(
            "polyline_count",
            json!(ast.polyline_count),
            ast.polyline_count.to_string(),
        ),
        field(
            "rectangle_count",
            json!(ast.rectangle_count),
            ast.rectangle_count.to_string(),
        ),
        field(
            "circle_count",
            json!(ast.circle_count),
            ast.circle_count.to_string(),
        ),
        field("arc_count", json!(ast.arc_count), ast.arc_count.to_string()),
        field(
            "rule_area_count",
            json!(ast.rule_area_count),
            ast.rule_area_count.to_string(),
        ),
        field(
            "sheet_instance_count",
            json!(ast.sheet_instance_count),
            ast.sheet_instance_count.to_string(),
        ),
        field(
            "symbol_instance_count",
            json!(ast.symbol_instance_count),
            ast.symbol_instance_count.to_string(),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
        field(
            "diagnostic_count",
            json!(doc.diagnostics().len()),
            doc.diagnostics().len().to_string(),
        ),
    ]
}

fn inspect_symbol(opts: &Opts) -> Result<(), String> {
    let doc = SymbolLibFile::read(&opts.path).map_err(|e| e.to_string())?;
    let fields = symbol_fields(&doc);
    emit_fields("symbol", &opts.path, &fields, opts.as_json);
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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
        let out = temp_out("inspect_sym", "kicad_sym");
        doc.write_mode(&out, WriteMode::Canonical)
            .map_err(|e| e.to_string())?;
        let s = std::fs::read_to_string(&out).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(out);
        println!("--- canonical ---\n{s}");
    }
    Ok(())
}

fn symbol_fields(doc: &kiutils_kicad::SymbolLibDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field(
            "generator",
            json!(ast.generator),
            format!("{:?}", ast.generator),
        ),
        field(
            "generator_version",
            json!(ast.generator_version),
            format!("{:?}", ast.generator_version),
        ),
        field(
            "symbol_count",
            json!(ast.symbol_count),
            ast.symbol_count.to_string(),
        ),
        field(
            "total_property_count",
            json!(ast.total_property_count),
            ast.total_property_count.to_string(),
        ),
        field(
            "total_pin_count",
            json!(ast.total_pin_count),
            ast.total_pin_count.to_string(),
        ),
        field(
            "first_symbol_name",
            json!(ast.symbols.first().and_then(|s| s.name.clone())),
            format!("{:?}", ast.symbols.first().and_then(|s| s.name.clone())),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
        field(
            "diagnostic_count",
            json!(doc.diagnostics().len()),
            doc.diagnostics().len().to_string(),
        ),
    ]
}

fn inspect_fplib(opts: &Opts) -> Result<(), String> {
    let doc = FpLibTableFile::read(&opts.path).map_err(|e| e.to_string())?;
    let fields = fplib_fields(&doc);
    emit_fields("fplib", &opts.path, &fields, opts.as_json);
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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
    let fields = dru_fields(&doc);
    emit_fields("dru", &opts.path, &fields, opts.as_json);
    if opts.show_unknown {
        for n in &doc.ast().unknown_nodes {
            println!(
                "unknown: head={:?} span={}..{}",
                n.head, n.span.start, n.span.end
            );
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
    let fields = project_fields(&doc);
    emit_fields("project", &opts.path, &fields, opts.as_json);
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

fn fplib_fields(doc: &kiutils_kicad::FpLibTableDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field(
            "library_count",
            json!(ast.library_count),
            ast.library_count.to_string(),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
    ]
}

fn dru_fields(doc: &kiutils_kicad::DesignRulesDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field("version", json!(ast.version), format!("{:?}", ast.version)),
        field(
            "rule_count",
            json!(ast.rule_count),
            ast.rule_count.to_string(),
        ),
        field(
            "unknown_count",
            json!(ast.unknown_nodes.len()),
            ast.unknown_nodes.len().to_string(),
        ),
    ]
}

fn project_fields(doc: &kiutils_kicad::ProjectDocument) -> Vec<InspectField> {
    let ast = doc.ast();
    vec![
        field(
            "meta_version",
            json!(ast.meta_version),
            format!("{:?}", ast.meta_version),
        ),
        field(
            "pinned_footprint_libs",
            json!(ast.pinned_footprint_libs),
            format!("{:?}", ast.pinned_footprint_libs),
        ),
        field(
            "unknown_field_count",
            json!(ast.unknown_fields.len()),
            ast.unknown_fields.len().to_string(),
        ),
    ]
}

fn temp_out(prefix: &str, ext: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{nanos}.{ext}"))
}

fn usage() -> String {
    "usage: kiutils-inspect <path> [--type auto|pcb|footprint|schematic|sch|symbol|fplib|dru|project] [--json] [--show-cst] [--show-canonical] [--show-unknown] [--show-diagnostics]".to_string()
}

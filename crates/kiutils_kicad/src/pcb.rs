use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::version::VersionPolicy;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbLayer {
    pub ordinal: Option<i32>,
    pub name: Option<String>,
    pub layer_type: Option<String>,
    pub user_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbNet {
    pub code: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbFootprintSummary {
    pub lib_id: Option<String>,
    pub layer: Option<String>,
    pub reference: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbSegmentSummary {
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub width: Option<f64>,
    pub layer: Option<String>,
    pub net: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbArcSummary {
    pub start: Option<[f64; 2]>,
    pub mid: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub width: Option<f64>,
    pub layer: Option<String>,
    pub net: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbViaSummary {
    pub at: Option<[f64; 2]>,
    pub size: Option<f64>,
    pub drill: Option<f64>,
    pub net: Option<i32>,
    pub via_type: Option<String>,
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbZoneSummary {
    pub net: Option<i32>,
    pub net_name: Option<String>,
    pub name: Option<String>,
    pub layer: Option<String>,
    pub layers: Vec<String>,
    pub hatch: Option<String>,
    pub fill_enabled: Option<bool>,
    pub polygon_count: usize,
    pub filled_polygon_count: usize,
    pub has_keepout: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGeneratedSummary {
    pub uuid: Option<String>,
    pub generated_type: Option<String>,
    pub name: Option<String>,
    pub layer: Option<String>,
    pub last_netname: Option<String>,
    pub members_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbDimensionSummary {
    pub dimension_type: Option<String>,
    pub layer: Option<String>,
    pub gr_text_count: usize,
    pub format_present: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbTargetSummary {
    pub shape: Option<String>,
    pub at: Option<[f64; 2]>,
    pub size: Option<f64>,
    pub width: Option<f64>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGroupSummary {
    pub name: Option<String>,
    pub group_id: Option<String>,
    pub member_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGraphicSummary {
    pub token: String,
    pub layer: Option<String>,
    pub text: Option<String>,
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub center: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbProperty {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbSetupSummary {
    pub has_stackup: bool,
    pub stackup_layer_count: usize,
    pub has_plot_settings: bool,
    pub pad_to_mask_clearance: Option<f64>,
    pub solder_mask_min_width: Option<f64>,
    pub aux_axis_origin: Option<[f64; 2]>,
    pub grid_origin: Option<[f64; 2]>,
    pub setup_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub has_general: bool,
    pub has_paper: bool,
    pub has_title_block: bool,
    pub has_setup: bool,
    pub setup: Option<PcbSetupSummary>,
    pub has_embedded_fonts: bool,
    pub has_embedded_files: bool,
    pub embedded_file_count: usize,
    pub properties: Vec<PcbProperty>,
    pub layers: Vec<PcbLayer>,
    pub nets: Vec<PcbNet>,
    pub footprints: Vec<PcbFootprintSummary>,
    pub segments: Vec<PcbSegmentSummary>,
    pub arcs: Vec<PcbArcSummary>,
    pub vias: Vec<PcbViaSummary>,
    pub zones: Vec<PcbZoneSummary>,
    pub generated_items: Vec<PcbGeneratedSummary>,
    pub dimensions: Vec<PcbDimensionSummary>,
    pub targets: Vec<PcbTargetSummary>,
    pub groups: Vec<PcbGroupSummary>,
    pub graphics: Vec<PcbGraphicSummary>,
    pub layer_count: usize,
    pub property_count: usize,
    pub net_count: usize,
    pub footprint_count: usize,
    pub graphic_count: usize,
    pub gr_line_count: usize,
    pub gr_rect_count: usize,
    pub gr_circle_count: usize,
    pub gr_arc_count: usize,
    pub gr_poly_count: usize,
    pub gr_curve_count: usize,
    pub gr_text_count: usize,
    pub gr_text_box_count: usize,
    pub trace_segment_count: usize,
    pub trace_arc_count: usize,
    pub via_count: usize,
    pub zone_count: usize,
    pub dimension_count: usize,
    pub target_count: usize,
    pub group_count: usize,
    pub generated_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct PcbDocument {
    ast: PcbAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
}

impl PcbDocument {
    pub fn ast(&self) -> &PcbAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut PcbAst {
        &mut self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        self.write_mode(path, WriteMode::Lossless)
    }

    pub fn write_mode<P: AsRef<Path>>(&self, path: P, mode: WriteMode) -> Result<(), Error> {
        match mode {
            WriteMode::Lossless => fs::write(path, self.cst.to_lossless_string())?,
            WriteMode::Canonical => fs::write(path, self.cst.to_canonical_string())?,
        }
        Ok(())
    }
}

pub struct PcbFile;

impl PcbFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<PcbDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_head(&cst, "kicad_pcb")?;

        let ast = parse_ast(&cst);
        let diagnostics = validate_version(ast.version)?;

        Ok(PcbDocument {
            ast,
            cst,
            diagnostics,
        })
    }
}

fn ensure_head(cst: &CstDocument, expected: &str) -> Result<(), Error> {
    let head = cst
        .nodes
        .first()
        .and_then(|n| match n {
            Node::List { items, .. } => items.first(),
            _ => None,
        })
        .and_then(|n| match n {
            Node::Atom {
                atom: Atom::Symbol(s),
                ..
            } => Some(s.as_str()),
            _ => None,
        });

    match head {
        Some(h) if h == expected => Ok(()),
        Some(h) => Err(Error::Validation(format!(
            "expected root token `{expected}`, got `{h}`"
        ))),
        None => Err(Error::Validation("missing root token".to_string())),
    }
}

fn parse_ast(cst: &CstDocument) -> PcbAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut has_general = false;
    let mut has_paper = false;
    let mut has_title_block = false;
    let mut has_setup = false;
    let mut setup = None;
    let mut has_embedded_fonts = false;
    let mut has_embedded_files = false;
    let mut embedded_file_count = 0usize;
    let mut properties = Vec::new();
    let mut layers = Vec::new();
    let mut nets = Vec::new();
    let mut footprints = Vec::new();
    let mut segments = Vec::new();
    let mut arcs = Vec::new();
    let mut vias = Vec::new();
    let mut zones = Vec::new();
    let mut generated_items = Vec::new();
    let mut dimensions = Vec::new();
    let mut targets = Vec::new();
    let mut groups = Vec::new();
    let mut graphics = Vec::new();
    let mut layer_count = 0usize;
    let mut property_count = 0usize;
    let mut net_count = 0usize;
    let mut footprint_count = 0usize;
    let mut graphic_count = 0usize;
    let mut gr_line_count = 0usize;
    let mut gr_rect_count = 0usize;
    let mut gr_circle_count = 0usize;
    let mut gr_arc_count = 0usize;
    let mut gr_poly_count = 0usize;
    let mut gr_curve_count = 0usize;
    let mut gr_text_count = 0usize;
    let mut gr_text_box_count = 0usize;
    let mut trace_segment_count = 0usize;
    let mut trace_arc_count = 0usize;
    let mut via_count = 0usize;
    let mut zone_count = 0usize;
    let mut dimension_count = 0usize;
    let mut target_count = 0usize;
    let mut group_count = 0usize;
    let mut generated_count = 0usize;
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for (idx, item) in items.iter().enumerate() {
            if idx == 0 {
                continue;
            }
            match head_of(item) {
                Some("version") => {
                    version = second_atom_string(item).and_then(|v| v.parse::<i32>().ok());
                }
                Some("generator") => {
                    generator = second_atom_string(item);
                }
                Some("generator_version") => {
                    generator_version = second_atom_string(item);
                }
                Some("general") => has_general = true,
                Some("paper") => has_paper = true,
                Some("title_block") => has_title_block = true,
                Some("layers") => {
                    if let Node::List { items: inner, .. } = item {
                        layers = parse_layers(inner);
                        layer_count = layers.len();
                    }
                }
                Some("setup") => {
                    has_setup = true;
                    setup = Some(parse_setup_summary(item));
                }
                Some("embedded_fonts") => has_embedded_fonts = true,
                Some("embedded_files") => {
                    has_embedded_files = true;
                    if let Node::List { items: inner, .. } = item {
                        embedded_file_count = inner
                            .iter()
                            .filter(|n| matches!(head_of(n), Some("file")))
                            .count();
                    }
                }
                Some("property") => {
                    property_count += 1;
                    if let Some(p) = parse_property(item) {
                        properties.push(p);
                    }
                }
                Some("net") => {
                    net_count += 1;
                    nets.push(parse_net(item));
                }
                Some("footprint") => {
                    footprint_count += 1;
                    footprints.push(parse_footprint_summary(item));
                }
                Some("segment") => {
                    trace_segment_count += 1;
                    segments.push(parse_segment_summary(item));
                }
                Some("arc") => {
                    trace_arc_count += 1;
                    arcs.push(parse_arc_summary(item));
                }
                Some("via") => {
                    via_count += 1;
                    vias.push(parse_via_summary(item));
                }
                Some("zone") => {
                    zone_count += 1;
                    zones.push(parse_zone_summary(item));
                }
                Some("dimension") => {
                    dimension_count += 1;
                    dimensions.push(parse_dimension_summary(item));
                }
                Some("target") => {
                    target_count += 1;
                    targets.push(parse_target_summary(item));
                }
                Some("group") => {
                    group_count += 1;
                    groups.push(parse_group_summary(item));
                }
                Some("generated") => {
                    generated_count += 1;
                    generated_items.push(parse_generated_summary(item));
                }
                Some("gr_line") => {
                    graphic_count += 1;
                    gr_line_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_line"));
                }
                Some("gr_rect") => {
                    graphic_count += 1;
                    gr_rect_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_rect"));
                }
                Some("gr_circle") => {
                    graphic_count += 1;
                    gr_circle_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_circle"));
                }
                Some("gr_arc") => {
                    graphic_count += 1;
                    gr_arc_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_arc"));
                }
                Some("gr_poly") => {
                    graphic_count += 1;
                    gr_poly_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_poly"));
                }
                Some("gr_curve") => {
                    graphic_count += 1;
                    gr_curve_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_curve"));
                }
                Some("gr_text") => {
                    graphic_count += 1;
                    gr_text_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_text"));
                }
                Some("gr_text_box") => {
                    graphic_count += 1;
                    gr_text_box_count += 1;
                    graphics.push(parse_graphic_summary(item, "gr_text_box"));
                }
                Some(h) if h.starts_with("gr_") => {
                    graphic_count += 1;
                    graphics.push(parse_graphic_summary(item, h));
                }
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    PcbAst {
        version,
        generator,
        generator_version,
        has_general,
        has_paper,
        has_title_block,
        has_setup,
        setup,
        has_embedded_fonts,
        has_embedded_files,
        embedded_file_count,
        properties,
        layers,
        nets,
        footprints,
        segments,
        arcs,
        vias,
        zones,
        generated_items,
        dimensions,
        targets,
        groups,
        graphics,
        layer_count,
        property_count,
        net_count,
        footprint_count,
        graphic_count,
        gr_line_count,
        gr_rect_count,
        gr_circle_count,
        gr_arc_count,
        gr_poly_count,
        gr_curve_count,
        gr_text_count,
        gr_text_box_count,
        trace_segment_count,
        trace_arc_count,
        via_count,
        zone_count,
        dimension_count,
        target_count,
        group_count,
        generated_count,
        unknown_nodes,
    }
}

fn head_of(node: &Node) -> Option<&str> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let Some(Node::Atom {
        atom: Atom::Symbol(head),
        ..
    }) = items.first()
    else {
        return None;
    };
    Some(head.as_str())
}

fn second_atom_string(node: &Node) -> Option<String> {
    let Node::List { items, .. } = node else {
        return None;
    };
    match items.get(1) {
        Some(Node::Atom {
            atom: Atom::Symbol(v),
            ..
        }) => Some(v.clone()),
        Some(Node::Atom {
            atom: Atom::Quoted(v),
            ..
        }) => Some(v.clone()),
        _ => None,
    }
}

fn parse_layers(items: &[Node]) -> Vec<PcbLayer> {
    let mut out = Vec::new();
    for entry in items.iter().skip(1) {
        let Node::List { items: fields, .. } = entry else {
            continue;
        };
        let ordinal = fields.first().and_then(atom_as_i32);
        let name = fields.get(1).and_then(atom_as_string);
        let layer_type = fields.get(2).and_then(atom_as_string);
        let user_name = fields.get(3).and_then(atom_as_string);
        out.push(PcbLayer {
            ordinal,
            name,
            layer_type,
            user_name,
        });
    }
    out
}

fn parse_net(node: &Node) -> PcbNet {
    let Node::List { items, .. } = node else {
        return PcbNet {
            code: None,
            name: None,
        };
    };
    let code = items.get(1).and_then(atom_as_i32);
    let name = items.get(2).and_then(atom_as_string);
    PcbNet { code, name }
}

fn parse_footprint_summary(node: &Node) -> PcbFootprintSummary {
    let Node::List { items, .. } = node else {
        return PcbFootprintSummary {
            lib_id: None,
            layer: None,
            reference: None,
            value: None,
        };
    };

    let lib_id = items.get(1).and_then(atom_as_string);
    let mut layer = None;
    let mut reference = None;
    let mut value = None;

    for child in items.iter().skip(2) {
        let Some(head) = head_of(child) else {
            continue;
        };
        match head {
            "layer" => {
                layer = second_atom_string(child);
            }
            "property" => {
                let Node::List { items: props, .. } = child else {
                    continue;
                };
                let key = props.get(1).and_then(atom_as_string);
                let val = props.get(2).and_then(atom_as_string);
                match key.as_deref() {
                    Some("Reference") => reference = val,
                    Some("Value") => value = val,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    PcbFootprintSummary {
        lib_id,
        layer,
        reference,
        value,
    }
}

fn parse_segment_summary(node: &Node) -> PcbSegmentSummary {
    let mut start = None;
    let mut end = None;
    let mut width = None;
    let mut layer = None;
    let mut net = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("width") => width = second_atom_f64(child),
                Some("layer") => layer = second_atom_string(child),
                Some("net") => net = second_atom_i32(child),
                _ => {}
            }
        }
    }
    PcbSegmentSummary {
        start,
        end,
        width,
        layer,
        net,
    }
}

fn parse_arc_summary(node: &Node) -> PcbArcSummary {
    let mut start = None;
    let mut mid = None;
    let mut end = None;
    let mut width = None;
    let mut layer = None;
    let mut net = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy(child),
                Some("mid") => mid = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("width") => width = second_atom_f64(child),
                Some("layer") => layer = second_atom_string(child),
                Some("net") => net = second_atom_i32(child),
                _ => {}
            }
        }
    }
    PcbArcSummary {
        start,
        mid,
        end,
        width,
        layer,
        net,
    }
}

fn parse_via_summary(node: &Node) -> PcbViaSummary {
    let mut at = None;
    let mut size = None;
    let mut drill = None;
    let mut net = None;
    let mut via_type = None;
    let mut layers = Vec::new();
    if let Node::List { items, .. } = node {
        // Some formats encode via type as second symbol: (via blind ...)
        via_type = items.get(1).and_then(|n| match n {
            Node::Atom {
                atom: Atom::Symbol(s),
                ..
            } if matches!(s.as_str(), "blind" | "micro" | "through") => Some(s.clone()),
            _ => None,
        });
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy(child),
                Some("size") => size = second_atom_f64(child),
                Some("drill") => drill = second_atom_f64(child),
                Some("net") => net = second_atom_i32(child),
                Some("layers") => {
                    if let Node::List { items: inner, .. } = child {
                        layers = inner.iter().skip(1).filter_map(atom_as_string).collect();
                    }
                }
                _ => {}
            }
        }
    }
    PcbViaSummary {
        at,
        size,
        drill,
        net,
        via_type,
        layers,
    }
}

fn parse_zone_summary(node: &Node) -> PcbZoneSummary {
    let mut net = None;
    let mut net_name = None;
    let mut name = None;
    let mut layer = None;
    let mut layers = Vec::new();
    let mut hatch = None;
    let mut fill_enabled = None;
    let mut polygon_count = 0usize;
    let mut filled_polygon_count = 0usize;
    let mut has_keepout = false;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("net") => net = second_atom_i32(child),
                Some("net_name") => net_name = second_atom_string(child),
                Some("name") => name = second_atom_string(child),
                Some("layer") => layer = second_atom_string(child),
                Some("layers") => {
                    if let Node::List { items: inner, .. } = child {
                        layers = inner.iter().skip(1).filter_map(atom_as_string).collect();
                    }
                }
                Some("hatch") => hatch = second_atom_string(child),
                Some("fill") => fill_enabled = second_atom_bool(child),
                Some("polygon") => polygon_count += 1,
                Some("filled_polygon") => filled_polygon_count += 1,
                Some("keepout") => has_keepout = true,
                _ => {}
            }
        }
    }
    PcbZoneSummary {
        net,
        net_name,
        name,
        layer,
        layers,
        hatch,
        fill_enabled,
        polygon_count,
        filled_polygon_count,
        has_keepout,
    }
}

fn parse_generated_summary(node: &Node) -> PcbGeneratedSummary {
    let mut uuid = None;
    let mut generated_type = None;
    let mut name = None;
    let mut layer = None;
    let mut last_netname = None;
    let mut members_count = 0usize;

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("uuid") => uuid = second_atom_string(child),
                Some("type") => generated_type = second_atom_string(child),
                Some("name") => name = second_atom_string(child),
                Some("layer") => layer = second_atom_string(child),
                Some("last_netname") => last_netname = second_atom_string(child),
                Some("members") => {
                    if let Some(members) = second_atom_string(child) {
                        members_count = members.split_whitespace().count();
                    }
                }
                _ => {}
            }
        }
    }

    PcbGeneratedSummary {
        uuid,
        generated_type,
        name,
        layer,
        last_netname,
        members_count,
    }
}

fn parse_dimension_summary(node: &Node) -> PcbDimensionSummary {
    let mut dimension_type = None;
    let mut layer = None;
    let mut gr_text_count = 0usize;
    let mut format_present = false;
    if let Node::List { items, .. } = node {
        // Some formats use: (dimension aligned ...), others: (dimension (type aligned) ...)
        dimension_type = items.get(1).and_then(atom_as_string);
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("type") => dimension_type = second_atom_string(child),
                Some("layer") => layer = second_atom_string(child),
                Some("gr_text") => gr_text_count += 1,
                Some("format") => format_present = true,
                _ => {}
            }
        }
    }
    PcbDimensionSummary {
        dimension_type,
        layer,
        gr_text_count,
        format_present,
    }
}

fn parse_target_summary(node: &Node) -> PcbTargetSummary {
    let mut shape = None;
    let mut at = None;
    let mut size = None;
    let mut width = None;
    let mut layer = None;
    if let Node::List { items, .. } = node {
        shape = items.get(1).and_then(atom_as_string);
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy(child),
                Some("size") => size = second_atom_f64(child),
                Some("width") => width = second_atom_f64(child),
                Some("layer") => layer = second_atom_string(child),
                _ => {}
            }
        }
    }
    PcbTargetSummary {
        shape,
        at,
        size,
        width,
        layer,
    }
}

fn parse_group_summary(node: &Node) -> PcbGroupSummary {
    let mut name = None;
    let mut group_id = None;
    let mut member_count = 0usize;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("name") => name = second_atom_string(child),
                Some("id") => group_id = second_atom_string(child),
                Some("members") => {
                    if let Node::List { items: inner, .. } = child {
                        member_count = inner.len().saturating_sub(1);
                    }
                }
                _ => {}
            }
        }
    }
    PcbGroupSummary {
        name,
        group_id,
        member_count,
    }
}

fn parse_graphic_summary(node: &Node, token: &str) -> PcbGraphicSummary {
    let mut layer = None;
    let mut text = None;
    let mut start = None;
    let mut end = None;
    let mut center = None;

    if let Node::List { items, .. } = node {
        // For gr_text / gr_text_box, second token may be the text content.
        text = items.get(1).and_then(atom_as_string);
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("layer") => layer = second_atom_string(child),
                Some("start") => start = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("center") => center = parse_xy(child),
                _ => {}
            }
        }
    }

    PcbGraphicSummary {
        token: token.to_string(),
        layer,
        text,
        start,
        end,
        center,
    }
}

fn parse_property(node: &Node) -> Option<PcbProperty> {
    let Node::List { items, .. } = node else {
        return None;
    };
    if !matches!(items.first().and_then(atom_as_string).as_deref(), Some("property")) {
        return None;
    }
    let key = items.get(1).and_then(atom_as_string)?;
    let value = items.get(2).and_then(atom_as_string)?;
    Some(PcbProperty { key, value })
}

fn parse_setup_summary(node: &Node) -> PcbSetupSummary {
    let mut has_stackup = false;
    let mut stackup_layer_count = 0usize;
    let mut has_plot_settings = false;
    let mut pad_to_mask_clearance = None;
    let mut solder_mask_min_width = None;
    let mut aux_axis_origin = None;
    let mut grid_origin = None;
    let mut setup_tokens = Vec::new();

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            if let Some(head) = head_of(child) {
                setup_tokens.push(head.to_string());
                match head {
                    "stackup" => {
                        has_stackup = true;
                        if let Node::List { items: stackup_items, .. } = child {
                            stackup_layer_count = stackup_items
                                .iter()
                                .filter(|n| matches!(head_of(n), Some("layer")))
                                .count();
                        }
                    }
                    "pcbplotparams" => has_plot_settings = true,
                    "pad_to_mask_clearance" => pad_to_mask_clearance = second_atom_f64(child),
                    "solder_mask_min_width" => solder_mask_min_width = second_atom_f64(child),
                    "aux_axis_origin" => aux_axis_origin = parse_xy(child),
                    "grid_origin" => grid_origin = parse_xy(child),
                    _ => {}
                }
            }
        }
    }

    PcbSetupSummary {
        has_stackup,
        stackup_layer_count,
        has_plot_settings,
        pad_to_mask_clearance,
        solder_mask_min_width,
        aux_axis_origin,
        grid_origin,
        setup_tokens,
    }
}

fn atom_as_string(node: &Node) -> Option<String> {
    match node {
        Node::Atom {
            atom: Atom::Symbol(v),
            ..
        } => Some(v.clone()),
        Node::Atom {
            atom: Atom::Quoted(v),
            ..
        } => Some(v.clone()),
        _ => None,
    }
}

fn atom_as_i32(node: &Node) -> Option<i32> {
    atom_as_string(node).and_then(|s| s.parse::<i32>().ok())
}

fn second_atom_i32(node: &Node) -> Option<i32> {
    second_atom_string(node).and_then(|s| s.parse::<i32>().ok())
}

fn second_atom_f64(node: &Node) -> Option<f64> {
    second_atom_string(node).and_then(|s| s.parse::<f64>().ok())
}

fn second_atom_bool(node: &Node) -> Option<bool> {
    match second_atom_string(node).as_deref() {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        _ => None,
    }
}

fn parse_xy(node: &Node) -> Option<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_string)?.parse::<f64>().ok()?;
    let y = items.get(2).and_then(atom_as_string)?.parse::<f64>().ok()?;
    Some([x, y])
}

fn validate_version(version: Option<i32>) -> Result<Vec<Diagnostic>, Error> {
    let policy = VersionPolicy::default();
    let mut diagnostics = Vec::new();

    if let Some(v) = version {
        if policy.reject_older && !policy.accepts(v) {
            return Err(Error::Validation(format!(
                "unsupported KiCad version {v}; expected v9+ format"
            )));
        }

        if policy.is_future_for_target(v) {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: "future_format",
                message: format!(
                    "version {v} is newer than target {:?}; keeping lossless CST for compatibility",
                    policy.target
                ),
                span: None,
                hint: Some("consider newer parser coverage for this token set".to_string()),
            });
        }
    }

    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_pcb"))
    }

    #[test]
    fn read_parses_version_and_preserves_lossless() {
        let path = tmp_file("pcb_read_ok");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20260101));
        assert_eq!(doc.ast().generator.as_deref(), Some("pcbnew"));
        assert!(doc.ast().unknown_nodes.is_empty());
        assert_eq!(doc.cst().to_lossless_string(), src);

        let out = tmp_file("pcb_write_ok");
        doc.write(&out).expect("write");
        let roundtrip = fs::read_to_string(&out).expect("read out");
        assert_eq!(roundtrip, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn read_fails_on_invalid_root() {
        let path = tmp_file("pcb_bad_root");
        fs::write(&path, "(a)(b)").expect("write fixture");

        let err = PcbFile::read(&path).expect_err("must fail");
        match err {
            Error::Parse(_) => {}
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_rejects_old_version() {
        let path = tmp_file("pcb_old_version");
        fs::write(&path, "(kicad_pcb (version 20220101))\n").expect("write fixture");

        let err = PcbFile::read(&path).expect_err("must fail");
        match err {
            Error::Validation(msg) => assert!(msg.contains("v9+")),
            other => panic!("unexpected error: {other}"),
        }

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_warns_on_future_version() {
        let path = tmp_file("pcb_future_version");
        fs::write(&path, "(kicad_pcb (version 20270101))\n").expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "future_format");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn canonical_write_normalizes_spacing() {
        let path = tmp_file("pcb_canon_src");
        fs::write(&path, "(kicad_pcb   (version 20260101)   )\n").expect("write fixture");
        let doc = PcbFile::read(&path).expect("read");

        let out = tmp_file("pcb_canon_out");
        doc.write_mode(&out, WriteMode::Canonical).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert_eq!(written, "(kicad_pcb (version 20260101))\n");

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn captures_unknown_nodes_and_preserves_roundtrip() {
        let path = tmp_file("pcb_unknown");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew) (mystery_token 1 2))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);
        assert_eq!(doc.ast().unknown_nodes[0].head.as_deref(), Some("mystery_token"));

        let out = tmp_file("pcb_unknown_out");
        doc.write(&out).expect("write");
        let roundtrip = fs::read_to_string(&out).expect("read out");
        assert_eq!(roundtrip, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn parses_top_level_counts() {
        let path = tmp_file("pcb_counts");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (property \"Owner\" \"Milind\")\n  (layers (0 F.Cu signal) (31 B.Cu signal))\n  (setup (stackup (layer \"F.Cu\" (type \"copper\")) (layer \"B.Cu\" (type \"copper\"))) (pcbplotparams) (pad_to_mask_clearance 0.1) (solder_mask_min_width 0.0) (aux_axis_origin 10 20) (grid_origin 11 21))\n  (net 0 \"\")\n  (footprint \"R_0603\")\n  (gr_line (start 0 0) (end 1 1))\n  (segment (start 0 0) (end 1 1) (width 0.25) (layer F.Cu) (net 0))\n  (via (at 0 0) (size 1) (drill 0.5) (layers F.Cu B.Cu))\n  (zone)\n  (dimension aligned (layer F.Cu) (gr_text \"1.0\" (at 0 0)))\n  (target plus (at 1 2) (size 1) (width 0.1) (layer F.Cu))\n  (group (name \"G\") (id \"abc\") (members \"u1\" \"u2\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().layer_count, 2);
        assert_eq!(doc.ast().layers.len(), 2);
        assert_eq!(doc.ast().layers[0].name.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().property_count, 1);
        assert_eq!(doc.ast().properties.len(), 1);
        assert_eq!(doc.ast().properties[0].key, "Owner");
        assert_eq!(doc.ast().setup.as_ref().map(|s| s.has_stackup), Some(true));
        assert_eq!(doc.ast().setup.as_ref().map(|s| s.stackup_layer_count), Some(2));
        assert_eq!(doc.ast().setup.as_ref().map(|s| s.has_plot_settings), Some(true));
        assert_eq!(
            doc.ast().setup.as_ref().and_then(|s| s.pad_to_mask_clearance),
            Some(0.1)
        );
        assert_eq!(doc.ast().net_count, 1);
        assert_eq!(doc.ast().nets.len(), 1);
        assert_eq!(doc.ast().nets[0].name.as_deref(), Some(""));
        assert_eq!(doc.ast().footprint_count, 1);
        assert_eq!(doc.ast().footprints.len(), 1);
        assert_eq!(doc.ast().footprints[0].lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().graphic_count, 1);
        assert_eq!(doc.ast().gr_line_count, 1);
        assert_eq!(doc.ast().graphics.len(), 1);
        assert_eq!(doc.ast().graphics[0].token, "gr_line");
        assert_eq!(doc.ast().graphics[0].layer, None);
        assert_eq!(doc.ast().trace_segment_count, 1);
        assert_eq!(doc.ast().segments.len(), 1);
        assert_eq!(doc.ast().segments[0].layer.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().via_count, 1);
        assert_eq!(doc.ast().vias.len(), 1);
        assert_eq!(doc.ast().vias[0].layers.len(), 2);
        assert_eq!(doc.ast().zone_count, 1);
        assert_eq!(doc.ast().zones.len(), 1);
        assert_eq!(doc.ast().zones[0].polygon_count, 0);
        assert_eq!(doc.ast().dimension_count, 1);
        assert_eq!(doc.ast().dimensions.len(), 1);
        assert_eq!(doc.ast().dimensions[0].layer.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().target_count, 1);
        assert_eq!(doc.ast().targets.len(), 1);
        assert_eq!(doc.ast().targets[0].shape.as_deref(), Some("plus"));
        assert_eq!(doc.ast().group_count, 1);
        assert_eq!(doc.ast().groups.len(), 1);
        assert_eq!(doc.ast().groups[0].member_count, 2);
        assert!(!doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 0);
        assert!(doc.ast().has_setup);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_embedded_files_regression() {
        let path = tmp_file("pcb_embedded_files");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (embedded_files\n    (file (name \"A.bin\") (type \"binary\") (data \"abc\"))\n    (file (name \"B.bin\") (type \"binary\") (data \"def\"))\n  )\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert!(doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 2);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_dimension_type_child_token_regression() {
        let path = tmp_file("pcb_dimension_type");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (dimension\n    (type aligned)\n    (layer \"Cmts.User\")\n    (format (units 2))\n    (gr_text \"10.0 mm\" (at 0 0))\n  )\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().dimension_count, 1);
        assert_eq!(doc.ast().dimensions.len(), 1);
        assert_eq!(doc.ast().dimensions[0].dimension_type.as_deref(), Some("aligned"));
        assert_eq!(doc.ast().dimensions[0].layer.as_deref(), Some("Cmts.User"));
        assert!(doc.ast().dimensions[0].format_present);
        assert_eq!(doc.ast().dimensions[0].gr_text_count, 1);

        let _ = fs::remove_file(path);
    }
}

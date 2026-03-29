use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sections::{parse_paper, parse_title_block, ParsedPaper, ParsedTitleBlock};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, mutate_root_and_refresh, paper_standard_node,
    paper_user_node, remove_property as remove_property_node, upsert_node,
    upsert_property_preserve_tail, upsert_scalar, upsert_section_child_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, atom_as_i32, atom_as_string, head_of, second_atom_bool, second_atom_f64,
    second_atom_i32, second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
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
pub struct PcbPadNet {
    pub code: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbPadDrill {
    pub shape: Option<String>,
    pub diameter: Option<f64>,
    pub width: Option<f64>,
    pub offset: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbPad {
    pub number: Option<String>,
    pub pad_type: Option<String>,
    pub shape: Option<String>,
    pub at: Option<[f64; 2]>,
    pub rotation: Option<f64>,
    pub size: Option<[f64; 2]>,
    pub layers: Vec<String>,
    pub net: Option<PcbPadNet>,
    pub drill: Option<PcbPadDrill>,
    pub uuid: Option<String>,
    pub pin_function: Option<String>,
    pub pin_type: Option<String>,
    pub locked: bool,
    pub property: Option<String>,
    pub remove_unused_layers: bool,
    pub keep_end_layers: bool,
    pub roundrect_rratio: Option<f64>,
    pub chamfer_ratio: Option<f64>,
    pub chamfer: Vec<String>,
    pub die_length: Option<f64>,
    pub solder_mask_margin: Option<f64>,
    pub solder_paste_margin: Option<f64>,
    pub solder_paste_margin_ratio: Option<f64>,
    pub clearance: Option<f64>,
    pub zone_connect: Option<i32>,
    pub thermal_width: Option<f64>,
    pub thermal_gap: Option<f64>,
    pub custom_clearance: Option<String>,
    pub custom_anchor: Option<String>,
    pub custom_primitives: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbFootprintModel {
    pub path: Option<String>,
    pub at: Option<[f64; 3]>,
    pub scale: Option<[f64; 3]>,
    pub rotate: Option<[f64; 3]>,
    pub hide: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbFootprint {
    pub lib_id: Option<String>,
    pub layer: Option<String>,
    pub at: Option<[f64; 2]>,
    pub rotation: Option<f64>,
    pub uuid: Option<String>,
    pub locked: bool,
    pub placed: bool,
    pub tedit: Option<String>,
    pub path: Option<String>,
    pub descr: Option<String>,
    pub tags: Option<String>,
    pub autoplace_cost90: Option<i32>,
    pub autoplace_cost180: Option<i32>,
    pub attr: Vec<String>,
    pub private_layers: Vec<String>,
    pub net_tie_pad_groups: Vec<Vec<String>>,
    pub properties: Vec<PcbProperty>,
    pub pads: Vec<PcbPad>,
    pub models: Vec<PcbFootprintModel>,
    pub model_count: usize,
    pub zone_count: usize,
    pub group_count: usize,
    pub graphic_count: usize,
    pub fp_line_count: usize,
    pub fp_rect_count: usize,
    pub fp_circle_count: usize,
    pub fp_arc_count: usize,
    pub fp_poly_count: usize,
    pub fp_curve_count: usize,
    pub fp_text_count: usize,
    pub fp_text_box_count: usize,
    pub reference: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbSegment {
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub width: Option<f64>,
    pub layer: Option<String>,
    pub net: Option<i32>,
    pub uuid: Option<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbArc {
    pub start: Option<[f64; 2]>,
    pub mid: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub width: Option<f64>,
    pub layer: Option<String>,
    pub net: Option<i32>,
    pub uuid: Option<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbVia {
    pub at: Option<[f64; 2]>,
    pub size: Option<f64>,
    pub drill: Option<f64>,
    pub drill_x: Option<f64>,
    pub drill_y: Option<f64>,
    pub drill_shape: Option<String>,
    pub net: Option<i32>,
    pub via_type: Option<String>,
    pub layers: Vec<String>,
    pub uuid: Option<String>,
    pub locked: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbZone {
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
pub struct PcbGeneratedItem {
    pub uuid: Option<String>,
    pub generated_type: Option<String>,
    pub name: Option<String>,
    pub layer: Option<String>,
    pub last_netname: Option<String>,
    pub members_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbDimension {
    pub dimension_type: Option<String>,
    pub layer: Option<String>,
    pub gr_text_count: usize,
    pub format_present: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbTarget {
    pub shape: Option<String>,
    pub at: Option<[f64; 2]>,
    pub size: Option<f64>,
    pub width: Option<f64>,
    pub layer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGroup {
    pub name: Option<String>,
    pub group_id: Option<String>,
    pub member_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGraphic {
    pub token: String,
    pub layer: Option<String>,
    pub text: Option<String>,
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub center: Option<[f64; 2]>,
    pub uuid: Option<String>,
    pub locked: bool,
    pub width: Option<f64>,
    pub stroke_type: Option<String>,
    pub fill_type: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub font_size: Option<[f64; 2]>,
    pub font_thickness: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbImage {
    pub at: Option<[f64; 2]>,
    pub layer: Option<String>,
    pub scale: Option<f64>,
    pub uuid: Option<String>,
    pub locked: bool,
    pub data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbProperty {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbSetup {
    pub has_stackup: bool,
    pub stackup_layer_count: usize,
    pub has_plot_settings: bool,
    pub pad_to_mask_clearance: Option<f64>,
    pub solder_mask_min_width: Option<f64>,
    pub aux_axis_origin: Option<[f64; 2]>,
    pub grid_origin: Option<[f64; 2]>,
    pub pad_to_paste_clearance: Option<f64>,
    pub pad_to_paste_clearance_ratio: Option<f64>,
    pub allow_soldermask_bridges_in_fps: Option<bool>,
    pub copper_finish: Option<String>,
    pub dielectric_constraints: Option<bool>,
    pub edge_connector: Option<String>,
    pub castellated_pads: Option<bool>,
    pub edge_plating: Option<bool>,
    pub setup_tokens: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbGeneral {
    pub thickness: Option<f64>,
    pub legacy_teardrops: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbPaper {
    pub kind: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub orientation: Option<String>,
}

impl From<ParsedPaper> for PcbPaper {
    fn from(value: ParsedPaper) -> Self {
        Self {
            kind: value.kind,
            width: value.width,
            height: value.height,
            orientation: value.orientation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PcbTitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<String>,
}

impl From<ParsedTitleBlock> for PcbTitleBlock {
    fn from(value: ParsedTitleBlock) -> Self {
        Self {
            title: value.title,
            date: value.date,
            revision: value.revision,
            company: value.company,
            comments: value.comments,
        }
    }
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
    pub general: Option<PcbGeneral>,
    pub paper: Option<PcbPaper>,
    pub title_block: Option<PcbTitleBlock>,
    pub has_setup: bool,
    pub setup: Option<PcbSetup>,
    pub has_embedded_fonts: bool,
    pub has_embedded_files: bool,
    pub embedded_file_count: usize,
    pub properties: Vec<PcbProperty>,
    pub layers: Vec<PcbLayer>,
    pub nets: Vec<PcbNet>,
    pub footprints: Vec<PcbFootprint>,
    pub segments: Vec<PcbSegment>,
    pub arcs: Vec<PcbArc>,
    pub vias: Vec<PcbVia>,
    pub zones: Vec<PcbZone>,
    pub generated_items: Vec<PcbGeneratedItem>,
    pub dimensions: Vec<PcbDimension>,
    pub targets: Vec<PcbTarget>,
    pub groups: Vec<PcbGroup>,
    pub graphics: Vec<PcbGraphic>,
    pub images: Vec<PcbImage>,
    pub image_count: usize,
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
    ast_dirty: bool,
}

impl PcbDocument {
    pub fn ast(&self) -> &PcbAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut PcbAst {
        self.ast_dirty = true;
        &mut self.ast
    }

    pub fn set_version(&mut self, version: i32) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "version", atom_symbol(version.to_string()), 1)
        })
    }

    pub fn set_generator<S: Into<String>>(&mut self, generator: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "generator", atom_symbol(generator.into()), 1)
        })
    }

    pub fn set_generator_version<S: Into<String>>(&mut self, generator_version: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(
                items,
                "generator_version",
                atom_quoted(generator_version.into()),
                1,
            )
        })
    }

    pub fn set_paper_standard<S: Into<String>>(
        &mut self,
        kind: S,
        orientation: Option<&str>,
    ) -> &mut Self {
        let node = paper_standard_node(kind.into(), orientation.map(|v| v.to_string()));
        self.mutate_root_items(|items| upsert_node(items, "paper", node, 1))
    }

    pub fn set_paper_user(
        &mut self,
        width: f64,
        height: f64,
        orientation: Option<&str>,
    ) -> &mut Self {
        let node = paper_user_node(width, height, orientation.map(|v| v.to_string()));
        self.mutate_root_items(|items| upsert_node(items, "paper", node, 1))
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(items, "title_block", 1, "title", atom_quoted(title.into()))
        })
    }

    pub fn set_date<S: Into<String>>(&mut self, date: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(items, "title_block", 1, "date", atom_quoted(date.into()))
        })
    }

    pub fn set_revision<S: Into<String>>(&mut self, revision: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(
                items,
                "title_block",
                1,
                "rev",
                atom_quoted(revision.into()),
            )
        })
    }

    pub fn set_company<S: Into<String>>(&mut self, company: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(
                items,
                "title_block",
                1,
                "company",
                atom_quoted(company.into()),
            )
        })
    }

    pub fn upsert_property<K: Into<String>, V: Into<String>>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Self {
        let key = key.into();
        let value = value.into();
        self.mutate_root_items(|items| upsert_property_preserve_tail(items, &key, &value, 1))
    }

    pub fn remove_property(&mut self, key: &str) -> &mut Self {
        let key = key.to_string();
        self.mutate_root_items(|items| remove_property_node(items, &key, 1))
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
        if self.ast_dirty {
            return Err(Error::Validation(
                "ast_mut changes are not serializable; use document setter APIs".to_string(),
            ));
        }
        match mode {
            WriteMode::Lossless => fs::write(path, self.cst.to_lossless_string())?,
            WriteMode::Canonical => fs::write(path, self.cst.to_canonical_string())?,
        }
        Ok(())
    }

    fn mutate_root_items<F>(&mut self, mutate: F) -> &mut Self
    where
        F: FnOnce(&mut Vec<Node>) -> bool,
    {
        mutate_root_and_refresh(
            &mut self.cst,
            &mut self.ast,
            &mut self.diagnostics,
            mutate,
            parse_ast,
            |_cst, ast| collect_diagnostics(ast.version),
        );
        self.ast_dirty = false;
        self
    }
}

pub struct PcbFile;

impl PcbFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<PcbDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["kicad_pcb"])?;

        let ast = parse_ast(&cst);
        let diagnostics = collect_diagnostics(ast.version);

        Ok(PcbDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

fn collect_diagnostics(version: Option<i32>) -> Vec<Diagnostic> {
    collect_version_diagnostics(version)
}

fn parse_ast(cst: &CstDocument) -> PcbAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut has_general = false;
    let mut has_paper = false;
    let mut has_title_block = false;
    let mut general = None;
    let mut paper = None;
    let mut title_block = None;
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
    let mut images = Vec::new();
    let mut image_count = 0usize;
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
                Some("general") => {
                    has_general = true;
                    general = Some(parse_general(item));
                    collect_unrecognized_children(
                        item,
                        &["thickness", "legacy_teardrops"],
                        &mut unknown_nodes,
                    );
                }
                Some("paper") => {
                    has_paper = true;
                    paper = Some(parse_paper(item).into());
                }
                Some("title_block") => {
                    has_title_block = true;
                    title_block = Some(parse_title_block(item).into());
                    collect_unrecognized_children(
                        item,
                        &["title", "date", "rev", "company", "comment"],
                        &mut unknown_nodes,
                    );
                }
                Some("layers") => {
                    if let Node::List { items: inner, .. } = item {
                        layers = parse_layers(inner);
                        layer_count = layers.len();
                    }
                }
                Some("setup") => {
                    has_setup = true;
                    setup = Some(parse_setup(item));
                    collect_unrecognized_setup_children(item, &mut unknown_nodes);
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
                    footprints.push(parse_footprint(item));
                }
                Some("segment") => {
                    trace_segment_count += 1;
                    segments.push(parse_segment(item));
                }
                Some("arc") => {
                    trace_arc_count += 1;
                    arcs.push(parse_arc(item));
                }
                Some("via") => {
                    via_count += 1;
                    vias.push(parse_via(item));
                }
                Some("zone") => {
                    zone_count += 1;
                    zones.push(parse_zone(item));
                }
                Some("dimension") => {
                    dimension_count += 1;
                    dimensions.push(parse_dimension(item));
                }
                Some("target") => {
                    target_count += 1;
                    targets.push(parse_target(item));
                }
                Some("group") => {
                    group_count += 1;
                    groups.push(parse_group(item));
                }
                Some("generated") => {
                    generated_count += 1;
                    generated_items.push(parse_generated_item(item));
                }
                Some("image") => {
                    image_count += 1;
                    images.push(parse_image(item));
                }
                Some("gr_line") => {
                    graphic_count += 1;
                    gr_line_count += 1;
                    graphics.push(parse_graphic(item, "gr_line"));
                }
                Some("gr_rect") => {
                    graphic_count += 1;
                    gr_rect_count += 1;
                    graphics.push(parse_graphic(item, "gr_rect"));
                }
                Some("gr_circle") => {
                    graphic_count += 1;
                    gr_circle_count += 1;
                    graphics.push(parse_graphic(item, "gr_circle"));
                }
                Some("gr_arc") => {
                    graphic_count += 1;
                    gr_arc_count += 1;
                    graphics.push(parse_graphic(item, "gr_arc"));
                }
                Some("gr_poly") => {
                    graphic_count += 1;
                    gr_poly_count += 1;
                    graphics.push(parse_graphic(item, "gr_poly"));
                }
                Some("gr_curve") => {
                    graphic_count += 1;
                    gr_curve_count += 1;
                    graphics.push(parse_graphic(item, "gr_curve"));
                }
                Some("gr_text") => {
                    graphic_count += 1;
                    gr_text_count += 1;
                    graphics.push(parse_graphic(item, "gr_text"));
                }
                Some("gr_text_box") => {
                    graphic_count += 1;
                    gr_text_box_count += 1;
                    graphics.push(parse_graphic(item, "gr_text_box"));
                }
                Some(h) if h.starts_with("gr_") => {
                    graphic_count += 1;
                    graphics.push(parse_graphic(item, h));
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
        general,
        paper,
        title_block,
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
        images,
        image_count,
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

fn parse_footprint(node: &Node) -> PcbFootprint {
    let Node::List { items, .. } = node else {
        return PcbFootprint {
            lib_id: None,
            layer: None,
            at: None,
            rotation: None,
            uuid: None,
            locked: false,
            placed: false,
            tedit: None,
            path: None,
            descr: None,
            tags: None,
            autoplace_cost90: None,
            autoplace_cost180: None,
            attr: Vec::new(),
            private_layers: Vec::new(),
            net_tie_pad_groups: Vec::new(),
            properties: Vec::new(),
            pads: Vec::new(),
            models: Vec::new(),
            model_count: 0,
            zone_count: 0,
            group_count: 0,
            graphic_count: 0,
            fp_line_count: 0,
            fp_rect_count: 0,
            fp_circle_count: 0,
            fp_arc_count: 0,
            fp_poly_count: 0,
            fp_curve_count: 0,
            fp_text_count: 0,
            fp_text_box_count: 0,
            reference: None,
            value: None,
        };
    };

    let lib_id = items.get(1).and_then(atom_as_string);
    let mut layer = None;
    let mut at = None;
    let mut rotation = None;
    let mut uuid = None;
    let mut locked = items
        .iter()
        .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
    let mut placed = false;
    let mut tedit = None;
    let mut path = None;
    let mut descr = None;
    let mut tags = None;
    let mut autoplace_cost90 = None;
    let mut autoplace_cost180 = None;
    let mut attr = Vec::new();
    let mut private_layers = Vec::new();
    let mut net_tie_pad_groups = Vec::new();
    let mut properties = Vec::new();
    let mut pads = Vec::new();
    let mut models = Vec::new();
    let mut model_count = 0usize;
    let mut zone_count = 0usize;
    let mut group_count = 0usize;
    let mut graphic_count = 0usize;
    let mut fp_line_count = 0usize;
    let mut fp_rect_count = 0usize;
    let mut fp_circle_count = 0usize;
    let mut fp_arc_count = 0usize;
    let mut fp_poly_count = 0usize;
    let mut fp_curve_count = 0usize;
    let mut fp_text_count = 0usize;
    let mut fp_text_box_count = 0usize;
    let mut reference = None;
    let mut value = None;

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("layer") => {
                layer = second_atom_string(child);
            }
            Some("at") => {
                let (xy, rot) = parse_xy_and_angle(child);
                at = xy;
                rotation = rot;
            }
            Some("uuid") => uuid = second_atom_string(child),
            Some("locked") => locked = true,
            Some("placed") => placed = true,
            Some("tedit") => tedit = second_atom_string(child),
            Some("path") => path = second_atom_string(child),
            Some("descr") => descr = second_atom_string(child),
            Some("tags") => tags = second_atom_string(child),
            Some("autoplace_cost90") => autoplace_cost90 = second_atom_i32(child),
            Some("autoplace_cost180") => autoplace_cost180 = second_atom_i32(child),
            Some("attr") => {
                if let Node::List { items: attrs, .. } = child {
                    attr = attrs.iter().skip(1).filter_map(atom_as_string).collect();
                }
            }
            Some("private_layers") => private_layers = parse_layer_names(child),
            Some("net_tie_pad_groups") => {
                if let Node::List { items: groups, .. } = child {
                    net_tie_pad_groups = groups
                        .iter()
                        .skip(1)
                        .filter_map(atom_as_string)
                        .map(|group| {
                            group
                                .split(',')
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .map(ToOwned::to_owned)
                                .collect::<Vec<_>>()
                        })
                        .filter(|group| !group.is_empty())
                        .collect();
                }
            }
            Some("property") => {
                let Node::List { items: props, .. } = child else {
                    continue;
                };
                let key = props.get(1).and_then(atom_as_string);
                let val = props.get(2).and_then(atom_as_string);
                if let (Some(key), Some(value)) = (key.clone(), val.clone()) {
                    properties.push(PcbProperty { key, value });
                }
                match key.as_deref() {
                    Some("Reference") => reference = val,
                    Some("Value") => value = val,
                    _ => {}
                }
            }
            Some("pad") => pads.push(parse_pad(child)),
            Some("model") => {
                models.push(parse_footprint_model(child));
                model_count += 1;
            }
            Some("zone") => zone_count += 1,
            Some("group") => group_count += 1,
            Some("fp_line") => {
                graphic_count += 1;
                fp_line_count += 1;
            }
            Some("fp_rect") => {
                graphic_count += 1;
                fp_rect_count += 1;
            }
            Some("fp_circle") => {
                graphic_count += 1;
                fp_circle_count += 1;
            }
            Some("fp_arc") => {
                graphic_count += 1;
                fp_arc_count += 1;
            }
            Some("fp_poly") => {
                graphic_count += 1;
                fp_poly_count += 1;
            }
            Some("fp_curve") => {
                graphic_count += 1;
                fp_curve_count += 1;
            }
            Some("fp_text") => {
                graphic_count += 1;
                fp_text_count += 1;
            }
            Some("fp_text_box") => {
                graphic_count += 1;
                fp_text_box_count += 1;
            }
            _ => {}
        }
    }

    PcbFootprint {
        lib_id,
        layer,
        at,
        rotation,
        uuid,
        locked,
        placed,
        tedit,
        path,
        descr,
        tags,
        autoplace_cost90,
        autoplace_cost180,
        attr,
        private_layers,
        net_tie_pad_groups,
        properties,
        pads,
        models,
        model_count,
        zone_count,
        group_count,
        graphic_count,
        fp_line_count,
        fp_rect_count,
        fp_circle_count,
        fp_arc_count,
        fp_poly_count,
        fp_curve_count,
        fp_text_count,
        fp_text_box_count,
        reference,
        value,
    }
}

fn parse_pad(node: &Node) -> PcbPad {
    let Node::List { items, .. } = node else {
        return PcbPad {
            number: None,
            pad_type: None,
            shape: None,
            at: None,
            rotation: None,
            size: None,
            layers: Vec::new(),
            net: None,
            drill: None,
            uuid: None,
            pin_function: None,
            pin_type: None,
            locked: false,
            property: None,
            remove_unused_layers: false,
            keep_end_layers: false,
            roundrect_rratio: None,
            chamfer_ratio: None,
            chamfer: Vec::new(),
            die_length: None,
            solder_mask_margin: None,
            solder_paste_margin: None,
            solder_paste_margin_ratio: None,
            clearance: None,
            zone_connect: None,
            thermal_width: None,
            thermal_gap: None,
            custom_clearance: None,
            custom_anchor: None,
            custom_primitives: 0,
        };
    };

    let number = items.get(1).and_then(atom_as_string);
    let pad_type = items.get(2).and_then(atom_as_string);
    let shape = items.get(3).and_then(atom_as_string);
    let mut at = None;
    let mut rotation = None;
    let mut size = None;
    let mut layers = Vec::new();
    let mut net = None;
    let mut drill = None;
    let mut uuid = None;
    let mut pin_function = None;
    let mut pin_type = None;
    let mut locked = items
        .iter()
        .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
    let mut property = None;
    let mut remove_unused_layers = false;
    let mut keep_end_layers = false;
    let mut roundrect_rratio = None;
    let mut chamfer_ratio = None;
    let mut chamfer = Vec::new();
    let mut die_length = None;
    let mut solder_mask_margin = None;
    let mut solder_paste_margin = None;
    let mut solder_paste_margin_ratio = None;
    let mut clearance = None;
    let mut zone_connect = None;
    let mut thermal_width = None;
    let mut thermal_gap = None;
    let mut custom_clearance = None;
    let mut custom_anchor = None;
    let mut custom_primitives = 0usize;

    for child in items.iter().skip(4) {
        match head_of(child) {
            Some("at") => {
                let (xy, rot) = parse_xy_and_angle(child);
                at = xy;
                rotation = rot;
            }
            Some("size") => size = parse_xy(child),
            Some("layers") => layers = parse_layer_names(child),
            Some("net") => net = Some(parse_pad_net(child)),
            Some("drill") => drill = Some(parse_pad_drill(child)),
            Some("uuid") => uuid = second_atom_string(child),
            Some("pinfunction") => pin_function = second_atom_string(child),
            Some("pintype") => pin_type = second_atom_string(child),
            Some("locked") => locked = true,
            Some("property") => property = second_atom_string(child),
            Some("remove_unused_layer") | Some("remove_unused_layers") => {
                remove_unused_layers = true
            }
            Some("keep_end_layers") => keep_end_layers = true,
            Some("roundrect_rratio") => roundrect_rratio = second_atom_f64(child),
            Some("chamfer_ratio") => chamfer_ratio = second_atom_f64(child),
            Some("chamfer") => {
                if let Node::List { items: corners, .. } = child {
                    chamfer = corners.iter().skip(1).filter_map(atom_as_string).collect();
                }
            }
            Some("die_length") => die_length = second_atom_f64(child),
            Some("solder_mask_margin") => solder_mask_margin = second_atom_f64(child),
            Some("solder_paste_margin") => solder_paste_margin = second_atom_f64(child),
            Some("solder_paste_margin_ratio") => solder_paste_margin_ratio = second_atom_f64(child),
            Some("clearance") => clearance = second_atom_f64(child),
            Some("zone_connect") => zone_connect = second_atom_i32(child),
            Some("thermal_width") => thermal_width = second_atom_f64(child),
            Some("thermal_gap") => thermal_gap = second_atom_f64(child),
            Some("options") => {
                if let Node::List { items: options, .. } = child {
                    for option in options.iter().skip(1) {
                        match head_of(option) {
                            Some("clearance") => custom_clearance = second_atom_string(option),
                            Some("anchor") => custom_anchor = second_atom_string(option),
                            _ => {}
                        }
                    }
                }
            }
            Some("primitives") => {
                if let Node::List {
                    items: primitives, ..
                } = child
                {
                    custom_primitives = primitives.len().saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    PcbPad {
        number,
        pad_type,
        shape,
        at,
        rotation,
        size,
        layers,
        net,
        drill,
        uuid,
        pin_function,
        pin_type,
        locked,
        property,
        remove_unused_layers,
        keep_end_layers,
        roundrect_rratio,
        chamfer_ratio,
        chamfer,
        die_length,
        solder_mask_margin,
        solder_paste_margin,
        solder_paste_margin_ratio,
        clearance,
        zone_connect,
        thermal_width,
        thermal_gap,
        custom_clearance,
        custom_anchor,
        custom_primitives,
    }
}

fn parse_model_xyz(node: &Node) -> Option<[f64; 3]> {
    let parse_triplet = |items: &[Node]| -> Option<[f64; 3]> {
        Some([
            items.get(1).and_then(atom_as_f64)?,
            items.get(2).and_then(atom_as_f64)?,
            items.get(3).and_then(atom_as_f64)?,
        ])
    };

    let Node::List { items, .. } = node else {
        return None;
    };

    if matches!(head_of(node), Some("xyz")) {
        return parse_triplet(items);
    }

    if let Some(Node::List {
        items: xyz_items, ..
    }) = items.get(1)
    {
        if matches!(head_of(items.get(1)?), Some("xyz")) {
            return parse_triplet(xyz_items);
        }
    }

    parse_triplet(items)
}

fn parse_footprint_model(node: &Node) -> PcbFootprintModel {
    let Node::List { items, .. } = node else {
        return PcbFootprintModel {
            path: None,
            at: None,
            scale: None,
            rotate: None,
            hide: false,
        };
    };

    let path = items.get(1).and_then(atom_as_string);
    let mut at = None;
    let mut scale = None;
    let mut rotate = None;
    let mut hide = items
        .iter()
        .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "hide"));

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("at") | Some("offset") => at = parse_model_xyz(child),
            Some("scale") => scale = parse_model_xyz(child),
            Some("rotate") => rotate = parse_model_xyz(child),
            Some("hide") => hide = true,
            _ => {}
        }
    }

    PcbFootprintModel {
        path,
        at,
        scale,
        rotate,
        hide,
    }
}

fn parse_pad_net(node: &Node) -> PcbPadNet {
    let Node::List { items, .. } = node else {
        return PcbPadNet {
            code: None,
            name: None,
        };
    };
    let code = items.get(1).and_then(atom_as_i32);
    let name = items.get(2).and_then(atom_as_string);
    PcbPadNet { code, name }
}
fn parse_pad_drill(node: &Node) -> PcbPadDrill {
    let Node::List { items, .. } = node else {
        return PcbPadDrill {
            shape: None,
            diameter: None,
            width: None,
            offset: None,
        };
    };

    let mut shape = None;
    let mut diameter = None;
    let mut width = None;
    let mut offset = None;

    for child in items.iter().skip(1) {
        match child {
            Node::List { .. } => {
                if matches!(head_of(child), Some("offset")) {
                    offset = parse_xy(child);
                }
            }
            Node::Atom { .. } => {
                if let Some(value) = atom_as_f64(child) {
                    if diameter.is_none() {
                        diameter = Some(value);
                    } else if width.is_none() {
                        width = Some(value);
                    }
                } else if let Some(token) = atom_as_string(child) {
                    shape = Some(token);
                }
            }
        }
    }

    PcbPadDrill {
        shape,
        diameter,
        width,
        offset,
    }
}

fn parse_layer_names(node: &Node) -> Vec<String> {
    let Node::List { items, .. } = node else {
        return Vec::new();
    };
    items.iter().skip(1).filter_map(atom_as_string).collect()
}

fn parse_segment(node: &Node) -> PcbSegment {
    let mut start = None;
    let mut end = None;
    let mut width = None;
    let mut layer = None;
    let mut net = None;
    let mut uuid = None;
    let mut locked = false;
    if let Node::List { items, .. } = node {
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("width") => width = second_atom_f64(child),
                Some("layer") => layer = second_atom_string(child),
                Some("net") => net = second_atom_i32(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                _ => {}
            }
        }
    }
    PcbSegment {
        start,
        end,
        width,
        layer,
        net,
        uuid,
        locked,
    }
}

fn parse_arc(node: &Node) -> PcbArc {
    let mut start = None;
    let mut mid = None;
    let mut end = None;
    let mut width = None;
    let mut layer = None;
    let mut net = None;
    let mut uuid = None;
    let mut locked = false;
    if let Node::List { items, .. } = node {
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy(child),
                Some("mid") => mid = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("width") => width = second_atom_f64(child),
                Some("layer") => layer = second_atom_string(child),
                Some("net") => net = second_atom_i32(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                _ => {}
            }
        }
    }
    PcbArc {
        start,
        mid,
        end,
        width,
        layer,
        net,
        uuid,
        locked,
    }
}

fn parse_via(node: &Node) -> PcbVia {
    let mut at = None;
    let mut size = None;
    let mut drill = None;
    let mut drill_x = None;
    let mut drill_y = None;
    let mut drill_shape = None;
    let mut net = None;
    let mut via_type = None;
    let mut layers = Vec::new();
    let mut uuid = None;
    let mut locked = false;
    if let Node::List { items, .. } = node {
        // Some formats encode via type as second symbol: (via blind ...)
        via_type = items.get(1).and_then(|n| match n {
            Node::Atom {
                atom: Atom::Symbol(s),
                ..
            } if matches!(s.as_str(), "blind" | "micro" | "through") => Some(s.clone()),
            _ => None,
        });
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy(child),
                Some("size") => size = second_atom_f64(child),
                Some("drill") => {
                    drill = second_atom_f64(child);
                    if let Node::List {
                        items: drill_items, ..
                    } = child
                    {
                        if let Some(shape) = drill_items.get(1).and_then(atom_as_string) {
                            if shape.parse::<f64>().is_err() {
                                drill_shape = Some(shape);
                                drill_x = drill_items.get(2).and_then(atom_as_f64);
                                drill_y = drill_items.get(3).and_then(atom_as_f64);
                            } else {
                                drill_x = drill_items.get(1).and_then(atom_as_f64);
                                drill_y = drill_items.get(2).and_then(atom_as_f64);
                            }
                        }
                    }
                }
                Some("net") => net = second_atom_i32(child),
                Some("layers") => {
                    if let Node::List { items: inner, .. } = child {
                        layers = inner.iter().skip(1).filter_map(atom_as_string).collect();
                    }
                }
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                _ => {}
            }
        }
    }
    PcbVia {
        at,
        size,
        drill,
        drill_x,
        drill_y,
        drill_shape,
        net,
        via_type,
        layers,
        uuid,
        locked,
    }
}

fn parse_zone(node: &Node) -> PcbZone {
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
    PcbZone {
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

fn parse_generated_item(node: &Node) -> PcbGeneratedItem {
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

    PcbGeneratedItem {
        uuid,
        generated_type,
        name,
        layer,
        last_netname,
        members_count,
    }
}

fn parse_dimension(node: &Node) -> PcbDimension {
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
    PcbDimension {
        dimension_type,
        layer,
        gr_text_count,
        format_present,
    }
}

fn parse_target(node: &Node) -> PcbTarget {
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
    PcbTarget {
        shape,
        at,
        size,
        width,
        layer,
    }
}

fn parse_group(node: &Node) -> PcbGroup {
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
    PcbGroup {
        name,
        group_id,
        member_count,
    }
}

fn parse_graphic(node: &Node, token: &str) -> PcbGraphic {
    let mut layer = None;
    let mut text = None;
    let mut start = None;
    let mut end = None;
    let mut center = None;
    let mut uuid = None;
    let mut locked = false;
    let mut width = None;
    let mut stroke_type = None;
    let mut fill_type = None;
    let mut at = None;
    let mut angle = None;
    let mut font_size = None;
    let mut font_thickness = None;

    if let Node::List { items, .. } = node {
        // Only text graphics treat second token as content.
        if matches!(token, "gr_text" | "gr_text_box") {
            text = items.get(1).and_then(atom_as_string);
        }
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("layer") => layer = second_atom_string(child),
                Some("start") => start = parse_xy(child),
                Some("end") => end = parse_xy(child),
                Some("center") => center = parse_xy(child),
                Some("width") => width = second_atom_f64(child),
                Some("stroke") => {
                    if let Node::List {
                        items: stroke_items,
                        ..
                    } = child
                    {
                        for stroke_child in stroke_items.iter().skip(1) {
                            match head_of(stroke_child) {
                                Some("width") => width = second_atom_f64(stroke_child),
                                Some("type") => stroke_type = second_atom_string(stroke_child),
                                _ => {}
                            }
                        }
                    }
                }
                Some("fill") => fill_type = second_atom_string(child),
                Some("at") => {
                    let (xy, rot) = parse_xy_and_angle(child);
                    at = xy;
                    angle = rot;
                }
                Some("angle") => angle = second_atom_f64(child),
                Some("effects") => {
                    if let Node::List {
                        items: effects_items,
                        ..
                    } = child
                    {
                        for effect_child in effects_items.iter().skip(1) {
                            match head_of(effect_child) {
                                Some("font") => {
                                    if let Node::List {
                                        items: font_items, ..
                                    } = effect_child
                                    {
                                        for font_child in font_items.iter().skip(1) {
                                            match head_of(font_child) {
                                                Some("size") => font_size = parse_xy(font_child),
                                                Some("thickness") => {
                                                    font_thickness = second_atom_f64(font_child)
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Some("thickness") => font_thickness = second_atom_f64(effect_child),
                                _ => {}
                            }
                        }
                    }
                }
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                _ => {}
            }
        }
    }

    PcbGraphic {
        token: token.to_string(),
        layer,
        text,
        start,
        end,
        center,
        uuid,
        locked,
        width,
        stroke_type,
        fill_type,
        at,
        angle,
        font_size,
        font_thickness,
    }
}

fn parse_image(node: &Node) -> PcbImage {
    let mut at = None;
    let mut layer = None;
    let mut scale = None;
    let mut uuid = None;
    let mut locked = false;
    let mut data = None;

    if let Node::List { items, .. } = node {
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => {
                    let (xy, _) = parse_xy_and_angle(child);
                    at = xy;
                }
                Some("layer") => layer = second_atom_string(child),
                Some("scale") => scale = second_atom_f64(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                Some("data") => data = second_atom_string(child),
                _ => {}
            }
        }
    }

    PcbImage {
        at,
        layer,
        scale,
        uuid,
        locked,
        data,
    }
}

fn parse_property(node: &Node) -> Option<PcbProperty> {
    let Node::List { items, .. } = node else {
        return None;
    };
    if !matches!(
        items.first().and_then(atom_as_string).as_deref(),
        Some("property")
    ) {
        return None;
    }
    let key = items.get(1).and_then(atom_as_string)?;
    let value = items.get(2).and_then(atom_as_string)?;
    Some(PcbProperty { key, value })
}

fn parse_general(node: &Node) -> PcbGeneral {
    let mut thickness = None;
    let mut legacy_teardrops = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("thickness") => thickness = second_atom_f64(child),
                Some("legacy_teardrops") => legacy_teardrops = second_atom_bool(child),
                _ => {}
            }
        }
    }
    PcbGeneral {
        thickness,
        legacy_teardrops,
    }
}

fn parse_setup(node: &Node) -> PcbSetup {
    let mut has_stackup = false;
    let mut stackup_layer_count = 0usize;
    let mut has_plot_settings = false;
    let mut pad_to_mask_clearance = None;
    let mut solder_mask_min_width = None;
    let mut aux_axis_origin = None;
    let mut grid_origin = None;
    let mut pad_to_paste_clearance = None;
    let mut pad_to_paste_clearance_ratio = None;
    let mut allow_soldermask_bridges_in_fps = None;
    let mut copper_finish = None;
    let mut dielectric_constraints = None;
    let mut edge_connector = None;
    let mut castellated_pads = None;
    let mut edge_plating = None;
    let mut setup_tokens = Vec::new();

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            if let Some(head) = head_of(child) {
                setup_tokens.push(head.to_string());
                match head {
                    "stackup" => {
                        has_stackup = true;
                        if let Node::List {
                            items: stackup_items,
                            ..
                        } = child
                        {
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
                    "pad_to_paste_clearance" => pad_to_paste_clearance = second_atom_f64(child),
                    "pad_to_paste_clearance_ratio" => {
                        pad_to_paste_clearance_ratio = second_atom_f64(child)
                    }
                    "allow_soldermask_bridges_in_footprints"
                    | "allow_soldermask_bridges_in_fps" => {
                        allow_soldermask_bridges_in_fps = second_atom_bool(child)
                    }
                    "copper_finish" => copper_finish = second_atom_string(child),
                    "dielectric_constraints" => dielectric_constraints = second_atom_bool(child),
                    "edge_connector" => edge_connector = second_atom_string(child),
                    "castellated_pads" => castellated_pads = second_atom_bool(child),
                    "edge_plating" => edge_plating = second_atom_bool(child),
                    _ => {}
                }
            }
        }
    }

    PcbSetup {
        has_stackup,
        stackup_layer_count,
        has_plot_settings,
        pad_to_mask_clearance,
        solder_mask_min_width,
        aux_axis_origin,
        grid_origin,
        pad_to_paste_clearance,
        pad_to_paste_clearance_ratio,
        allow_soldermask_bridges_in_fps,
        copper_finish,
        dielectric_constraints,
        edge_connector,
        castellated_pads,
        edge_plating,
        setup_tokens,
    }
}

fn collect_unrecognized_children(node: &Node, known_heads: &[&str], out: &mut Vec<UnknownNode>) {
    let Node::List { items, .. } = node else {
        return;
    };
    for child in items.iter().skip(1) {
        match head_of(child) {
            Some(head) if known_heads.contains(&head) => {}
            _ => {
                if let Some(unknown) = UnknownNode::from_node(child) {
                    out.push(unknown);
                }
            }
        }
    }
}

fn collect_unrecognized_setup_children(node: &Node, out: &mut Vec<UnknownNode>) {
    let Node::List { items, .. } = node else {
        return;
    };
    for child in items.iter().skip(1) {
        match head_of(child) {
            Some("stackup") => {
                collect_unrecognized_children(child, &["layer"], out);
            }
            Some("pcbplotparams")
            | Some("pad_to_mask_clearance")
            | Some("solder_mask_min_width")
            | Some("aux_axis_origin")
            | Some("grid_origin")
            | Some("pad_to_paste_clearance")
            | Some("pad_to_paste_clearance_ratio")
            | Some("allow_soldermask_bridges_in_footprints")
            | Some("allow_soldermask_bridges_in_fps")
            | Some("copper_finish")
            | Some("dielectric_constraints")
            | Some("edge_connector")
            | Some("castellated_pads")
            | Some("edge_plating") => {}
            _ => {
                if let Some(unknown) = UnknownNode::from_node(child) {
                    out.push(unknown);
                }
            }
        }
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

fn parse_xy_and_angle(node: &Node) -> (Option<[f64; 2]>, Option<f64>) {
    let Node::List { items, .. } = node else {
        return (None, None);
    };
    let x = items.get(1).and_then(atom_as_f64);
    let y = items.get(2).and_then(atom_as_f64);
    let rot = items.get(3).and_then(atom_as_f64);
    match (x, y) {
        (Some(x), Some(y)) => (Some([x, y]), rot),
        _ => (None, rot),
    }
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
    fn read_warns_on_legacy_version() {
        let path = tmp_file("pcb_old_version");
        fs::write(&path, "(kicad_pcb (version 20220101))\n").expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_format");

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
        assert_eq!(
            doc.ast().unknown_nodes[0].head.as_deref(),
            Some("mystery_token")
        );

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
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (general (thickness 1.6) (legacy_teardrops no))\n  (paper \"User\" 100 80 portrait)\n  (title_block (title \"Demo\") (date \"2026-02-23\") (rev \"A\") (company \"Milind\") (comment 1 \"c1\") (comment 2 \"c2\"))\n  (property \"Owner\" \"Milind\")\n  (layers (0 F.Cu signal) (31 B.Cu signal))\n  (setup (stackup (layer \"F.Cu\" (type \"copper\")) (layer \"B.Cu\" (type \"copper\"))) (pcbplotparams) (pad_to_mask_clearance 0.1) (solder_mask_min_width 0.0) (aux_axis_origin 10 20) (grid_origin 11 21))\n  (net 0 \"\")\n  (footprint \"R_0603\"\n    (at 10 20 90)\n    (layer F.Cu)\n    (uuid \"fp-1\")\n    (property \"Reference\" \"R1\")\n    (property \"Value\" \"1k\")\n    (fp_line (start 0 0) (end 1 1) (layer F.SilkS))\n    (fp_text reference \"R1\" (at 0 0) (layer F.SilkS))\n    (pad \"1\" smd rect (at 0 0) (size 1 1) (layers F.Cu F.Mask))\n    (model \"r.step\")\n  )\n  (gr_line locked (start 0 0) (end 1 1) (layer F.Cu) (uuid \"g-1\"))\n  (segment locked (start 0 0) (end 1 1) (width 0.25) (layer F.Cu) (net 0) (uuid \"s-1\"))\n  (arc (start 0 0) (mid 0.5 0.5) (end 1 1) (width 0.25) (layer F.Cu) (net 0) (uuid \"a-1\"))\n  (via blind locked (at 0 0) (size 1) (drill oval 0.5 0.25) (layers F.Cu B.Cu) (uuid \"v-1\"))\n  (zone)\n  (dimension aligned (layer F.Cu) (gr_text \"1.0\" (at 0 0)))\n  (target plus (at 1 2) (size 1) (width 0.1) (layer F.Cu))\n  (group (name \"G\") (id \"abc\") (members \"u1\" \"u2\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().layer_count, 2);
        assert_eq!(doc.ast().layers.len(), 2);
        assert_eq!(doc.ast().layers[0].name.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().property_count, 1);
        assert_eq!(doc.ast().properties.len(), 1);
        assert_eq!(doc.ast().properties[0].key, "Owner");
        assert!(doc.ast().has_general);
        assert!(doc.ast().has_paper);
        assert!(doc.ast().has_title_block);
        assert_eq!(
            doc.ast().general.as_ref().and_then(|g| g.thickness),
            Some(1.6)
        );
        assert_eq!(
            doc.ast().general.as_ref().and_then(|g| g.legacy_teardrops),
            Some(false)
        );
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("User".to_string())
        );
        assert_eq!(doc.ast().paper.as_ref().and_then(|p| p.width), Some(100.0));
        assert_eq!(doc.ast().paper.as_ref().and_then(|p| p.height), Some(80.0));
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.orientation.clone()),
            Some("portrait".to_string())
        );
        assert_eq!(
            doc.ast().title_block.as_ref().and_then(|t| t.title.clone()),
            Some("Demo".to_string())
        );
        assert_eq!(
            doc.ast().title_block.as_ref().map(|t| t.comments.len()),
            Some(2)
        );
        assert_eq!(
            doc.ast()
                .title_block
                .as_ref()
                .and_then(|t| t.comments.first().cloned()),
            Some("c1".to_string())
        );
        assert_eq!(doc.ast().setup.as_ref().map(|s| s.has_stackup), Some(true));
        assert_eq!(
            doc.ast().setup.as_ref().map(|s| s.stackup_layer_count),
            Some(2)
        );
        assert_eq!(
            doc.ast().setup.as_ref().map(|s| s.has_plot_settings),
            Some(true)
        );
        assert_eq!(
            doc.ast()
                .setup
                .as_ref()
                .and_then(|s| s.pad_to_mask_clearance),
            Some(0.1)
        );
        assert_eq!(doc.ast().net_count, 1);
        assert_eq!(doc.ast().nets.len(), 1);
        assert_eq!(doc.ast().nets[0].name.as_deref(), Some(""));
        assert_eq!(doc.ast().footprint_count, 1);
        assert_eq!(doc.ast().footprints.len(), 1);
        assert_eq!(doc.ast().footprints[0].lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().footprints[0].layer.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().footprints[0].at, Some([10.0, 20.0]));
        assert_eq!(doc.ast().footprints[0].rotation, Some(90.0));
        assert_eq!(doc.ast().footprints[0].uuid.as_deref(), Some("fp-1"));
        assert_eq!(doc.ast().footprints[0].reference.as_deref(), Some("R1"));
        assert_eq!(doc.ast().footprints[0].value.as_deref(), Some("1k"));
        assert_eq!(doc.ast().footprints[0].properties.len(), 2);
        assert_eq!(doc.ast().footprints[0].pads.len(), 1);
        assert_eq!(doc.ast().footprints[0].model_count, 1);
        assert_eq!(doc.ast().footprints[0].graphic_count, 2);
        assert_eq!(doc.ast().footprints[0].fp_line_count, 1);
        assert_eq!(doc.ast().footprints[0].fp_text_count, 1);
        assert_eq!(doc.ast().footprints[0].pads[0].number.as_deref(), Some("1"));
        assert_eq!(
            doc.ast().footprints[0].pads[0].pad_type.as_deref(),
            Some("smd")
        );
        assert_eq!(
            doc.ast().footprints[0].pads[0].shape.as_deref(),
            Some("rect")
        );
        assert_eq!(doc.ast().footprints[0].pads[0].at, Some([0.0, 0.0]));
        assert_eq!(doc.ast().footprints[0].pads[0].size, Some([1.0, 1.0]));
        assert_eq!(doc.ast().footprints[0].pads[0].layers.len(), 2);
        assert_eq!(doc.ast().footprints[0].pads[0].layers[0].as_str(), "F.Cu");
        assert_eq!(doc.ast().footprints[0].pads[0].layers[1].as_str(), "F.Mask");
        assert_eq!(doc.ast().graphic_count, 1);
        assert_eq!(doc.ast().gr_line_count, 1);
        assert_eq!(doc.ast().graphics.len(), 1);
        assert_eq!(doc.ast().graphics[0].token, "gr_line");
        assert_eq!(doc.ast().graphics[0].layer.as_deref(), Some("F.Cu"));
        assert!(doc.ast().graphics[0].locked);
        assert_eq!(doc.ast().graphics[0].uuid.as_deref(), Some("g-1"));
        assert_eq!(doc.ast().trace_segment_count, 1);
        assert_eq!(doc.ast().segments.len(), 1);
        assert_eq!(doc.ast().segments[0].layer.as_deref(), Some("F.Cu"));
        assert!(doc.ast().segments[0].locked);
        assert_eq!(doc.ast().segments[0].uuid.as_deref(), Some("s-1"));
        assert_eq!(doc.ast().trace_arc_count, 1);
        assert_eq!(doc.ast().arcs.len(), 1);
        assert_eq!(doc.ast().arcs[0].uuid.as_deref(), Some("a-1"));
        assert_eq!(doc.ast().via_count, 1);
        assert_eq!(doc.ast().vias.len(), 1);
        assert_eq!(doc.ast().vias[0].via_type.as_deref(), Some("blind"));
        assert!(doc.ast().vias[0].locked);
        assert_eq!(doc.ast().vias[0].drill, None);
        assert_eq!(doc.ast().vias[0].drill_shape.as_deref(), Some("oval"));
        assert_eq!(doc.ast().vias[0].drill_x, Some(0.5));
        assert_eq!(doc.ast().vias[0].drill_y, Some(0.25));
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
    fn parses_detailed_footprint_pad_data() {
        let path = tmp_file("pcb_footprint_pad_details");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (net 1 \"GND\")\n  (footprint \"Connector\"\n    (at 12.5 30 180)\n    (layer F.Cu)\n    (property \"Reference\" \"J1\")\n    (pad \"1\" thru_hole circle\n      (at 1 2 90)\n      (size 1.4 1.4)\n      (drill oval 0.9 0.7 (offset 0.1 0.2))\n      (layers *.Cu *.Mask)\n      (net 1 \"GND\")\n      (pinfunction \"A\")\n      (pintype \"passive\")\n      (uuid \"pad-1\")\n    )\n  )\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().footprints.len(), 1);
        assert_eq!(doc.ast().footprints[0].pads.len(), 1);

        let pad = &doc.ast().footprints[0].pads[0];
        assert_eq!(pad.number.as_deref(), Some("1"));
        assert_eq!(pad.pad_type.as_deref(), Some("thru_hole"));
        assert_eq!(pad.shape.as_deref(), Some("circle"));
        assert_eq!(pad.at, Some([1.0, 2.0]));
        assert_eq!(pad.rotation, Some(90.0));
        assert_eq!(pad.size, Some([1.4, 1.4]));
        assert_eq!(pad.layers, vec!["*.Cu", "*.Mask"]);
        assert_eq!(pad.net.as_ref().and_then(|n| n.code), Some(1));
        assert_eq!(
            pad.net.as_ref().and_then(|n| n.name.as_deref()),
            Some("GND")
        );
        assert_eq!(
            pad.drill.as_ref().and_then(|d| d.shape.as_deref()),
            Some("oval")
        );
        assert_eq!(pad.drill.as_ref().and_then(|d| d.diameter), Some(0.9));
        assert_eq!(pad.drill.as_ref().and_then(|d| d.width), Some(0.7));
        assert_eq!(pad.drill.as_ref().and_then(|d| d.offset), Some([0.1, 0.2]));
        assert_eq!(pad.pin_function.as_deref(), Some("A"));
        assert_eq!(pad.pin_type.as_deref(), Some("passive"));
        assert_eq!(pad.uuid.as_deref(), Some("pad-1"));

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
        assert_eq!(
            doc.ast().dimensions[0].dimension_type.as_deref(),
            Some("aligned")
        );
        assert_eq!(doc.ast().dimensions[0].layer.as_deref(), Some("Cmts.User"));
        assert!(doc.ast().dimensions[0].format_present);
        assert_eq!(doc.ast().dimensions[0].gr_text_count, 1);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_standard_paper_orientation() {
        let path = tmp_file("pcb_std_paper");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew) (paper A4 portrait))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert!(doc.ast().has_paper);
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("A4".to_string())
        );
        assert_eq!(doc.ast().paper.as_ref().and_then(|p| p.width), None);
        assert_eq!(doc.ast().paper.as_ref().and_then(|p| p.height), None);
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.orientation.clone()),
            Some("portrait".to_string())
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_updates_core_fields_and_preserves_unknowns() {
        let path = tmp_file("pcb_edit_roundtrip");
        let src = "(kicad_pcb (version 20241229) (generator pcbnew)\n  (paper \"A4\")\n  (title_block (title \"Old\") (date \"2025-01-01\") (rev \"A\") (company \"OldCo\"))\n  (property \"Owner\" \"Milind\")\n  (future_token 1 2)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .set_paper_standard("A3", Some("portrait"))
            .set_title("Roundtrip Demo")
            .set_date("2026-02-25")
            .set_revision("B")
            .set_company("Lords")
            .upsert_property("Owner", "Milind Sharma")
            .upsert_property("Build", "2")
            .remove_property("DoesNotExist");

        let out = tmp_file("pcb_edit_roundtrip_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert!(written.contains("(future_token 1 2)"));

        let reread = PcbFile::read(&out).expect("reread");
        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("A3".to_string())
        );
        assert_eq!(
            reread
                .ast()
                .paper
                .as_ref()
                .and_then(|p| p.orientation.clone()),
            Some("portrait".to_string())
        );
        assert_eq!(
            reread
                .ast()
                .title_block
                .as_ref()
                .and_then(|t| t.title.clone()),
            Some("Roundtrip Demo".to_string())
        );
        assert_eq!(reread.ast().property_count, 2);
        assert_eq!(reread.ast().unknown_nodes.len(), 1);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn lossless_write_preserves_unrelated_formatting_for_targeted_edit() {
        let path = tmp_file("pcb_lossless_targeted_edit");
        let src =
            "(kicad_pcb  (version   20241229)\n  (generator pcbnew)\n  (future_token   1   2)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_version(20260101);

        let out = tmp_file("pcb_lossless_targeted_edit_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert_eq!(
            written,
            "(kicad_pcb  (version 20260101)\n  (generator pcbnew)\n  (future_token   1   2)\n)\n"
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn no_op_setter_keeps_lossless_raw_unchanged() {
        let path = tmp_file("pcb_noop_setter");
        let src = "(kicad_pcb  (version   20241229)\n  (generator pcbnew)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_version(20241229);

        let out = tmp_file("pcb_noop_setter_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert_eq!(written, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn ast_mut_write_returns_validation_error() {
        let path = tmp_file("pcb_ast_mut_write_error");
        let src = "(kicad_pcb (version 20241229) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.ast_mut().version = Some(20260101);

        let out = tmp_file("pcb_ast_mut_write_error_out");
        let err = doc.write(&out).expect_err("write should fail");
        match err {
            Error::Validation(msg) => {
                assert!(msg.contains("ast_mut changes are not serializable"));
            }
            _ => panic!("expected validation error"),
        }

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn set_paper_preserves_unknown_tail_children() {
        let path = tmp_file("pcb_paper_preserve_tail");
        let src =
            "(kicad_pcb (version 20241229) (generator pcbnew) (paper \"A4\" portrait (mystery 1)))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_paper_standard("A3", Some("portrait"));

        let out = tmp_file("pcb_paper_preserve_tail_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert!(written.contains("(paper \"A3\" portrait (mystery 1))"));

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn set_version_preserves_unknown_tail_children() {
        let path = tmp_file("pcb_version_preserve_tail");
        let src = "(kicad_pcb (version 20241229 extra_token) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_version(20260101);

        let out = tmp_file("pcb_version_preserve_tail_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert!(written.contains("(version 20260101 extra_token)"));

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn captures_nested_unknown_nodes_in_setup_section() {
        let path = tmp_file("pcb_nested_unknown_setup");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (setup (stackup (layer \"F.Cu\") (mystery_stackup_token x)) (mystery_setup_token y))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert!(doc
            .ast()
            .unknown_nodes
            .iter()
            .any(|n| n.head.as_deref() == Some("mystery_setup_token")));
        assert!(doc
            .ast()
            .unknown_nodes
            .iter()
            .any(|n| n.head.as_deref() == Some("mystery_stackup_token")));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn non_text_graphics_do_not_parse_locked_as_text() {
        let path = tmp_file("pcb_graphic_text_regression");
        let src = "(kicad_pcb (version 20241229) (generator pcbnew)\n  (gr_line locked (start 0 0) (end 1 1) (layer \"F.SilkS\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = PcbFile::read(&path).expect("read");
        assert_eq!(doc.ast().graphics.len(), 1);
        assert_eq!(doc.ast().graphics[0].token, "gr_line");
        assert_eq!(doc.ast().graphics[0].text, None);
        assert!(doc.ast().graphics[0].locked);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_updates_user_paper_dimensions() {
        let path = tmp_file("pcb_edit_paper_user");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew) (paper A4))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.set_paper_user(100.0, 80.0, Some("landscape"));

        let out = tmp_file("pcb_edit_paper_user_out");
        doc.write(&out).expect("write");
        let reread = PcbFile::read(&out).expect("reread");
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("User".to_string())
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.width),
            Some(100.0)
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.height),
            Some(80.0)
        );
        assert_eq!(
            reread
                .ast()
                .paper
                .as_ref()
                .and_then(|p| p.orientation.clone()),
            Some("landscape".to_string())
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn upsert_property_preserves_existing_extra_children() {
        let path = tmp_file("pcb_property_preserve_extra");
        let src = "(kicad_pcb (version 20260101) (generator pcbnew)\n  (property \"Owner\" \"Old\" (at 1 2 0))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = PcbFile::read(&path).expect("read");
        doc.upsert_property("Owner", "New");

        let out = tmp_file("pcb_property_preserve_extra_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert!(written.contains("(property \"Owner\" \"New\" (at 1 2 0))"));

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

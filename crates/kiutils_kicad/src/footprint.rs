use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, mutate_root_and_refresh,
    remove_property as remove_property_node, root_head, upsert_property_preserve_tail,
    upsert_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, atom_as_string, head_of, list_child_head_count, second_atom_bool, second_atom_f64,
    second_atom_i32, second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

// --- Footprint pad types ---

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpPadNet {
    pub code: Option<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpPadDrill {
    pub shape: Option<String>,
    pub diameter: Option<f64>,
    pub width: Option<f64>,
    pub offset: Option<[f64; 2]>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpPad {
    pub number: Option<String>,
    pub pad_type: Option<String>,
    pub shape: Option<String>,
    pub at: Option<[f64; 2]>,
    pub rotation: Option<f64>,
    pub size: Option<[f64; 2]>,
    pub layers: Vec<String>,
    pub net: Option<FpPadNet>,
    pub drill: Option<FpPadDrill>,
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

// --- Footprint graphic types ---

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpGraphic {
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

// --- Footprint model type ---

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpModel {
    pub path: Option<String>,
    pub at: Option<[f64; 3]>,
    pub scale: Option<[f64; 3]>,
    pub rotate: Option<[f64; 3]>,
    pub hide: bool,
}

// --- Footprint zone type ---

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpZone {
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

// --- Footprint group type ---

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpGroup {
    pub name: Option<String>,
    pub group_id: Option<String>,
    pub member_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpProperty {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FootprintAst {
    pub lib_id: Option<String>,
    pub version: Option<i32>,
    pub tedit: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub layer: Option<String>,
    pub descr: Option<String>,
    pub tags: Option<String>,
    pub property_count: usize,
    pub attr_present: bool,
    pub locked_present: bool,
    pub private_layers_present: bool,
    pub net_tie_pad_groups_present: bool,
    pub embedded_fonts_present: bool,
    pub has_embedded_files: bool,
    pub embedded_file_count: usize,
    pub clearance: Option<String>,
    pub solder_mask_margin: Option<String>,
    pub solder_paste_margin: Option<String>,
    pub solder_paste_margin_ratio: Option<String>,
    pub duplicate_pad_numbers_are_jumpers: Option<bool>,
    pub pad_count: usize,
    pub pads: Vec<FpPad>,
    pub model_count: usize,
    pub models: Vec<FpModel>,
    pub zone_count: usize,
    pub zones: Vec<FpZone>,
    pub group_count: usize,
    pub groups: Vec<FpGroup>,
    pub fp_line_count: usize,
    pub fp_rect_count: usize,
    pub fp_circle_count: usize,
    pub fp_arc_count: usize,
    pub fp_poly_count: usize,
    pub fp_curve_count: usize,
    pub fp_text_count: usize,
    pub fp_text_box_count: usize,
    pub dimension_count: usize,
    pub graphic_count: usize,
    pub graphics: Vec<FpGraphic>,
    pub attr: Vec<String>,
    pub locked: bool,
    pub placed: bool,
    pub private_layers: Vec<String>,
    pub net_tie_pad_groups: Vec<Vec<String>>,
    pub reference: Option<String>,
    pub value: Option<String>,
    pub properties: Vec<FpProperty>,
    pub unknown_nodes: Vec<UnknownNode>,
}
#[derive(Debug, Clone)]
pub struct FootprintDocument {
    ast: FootprintAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

impl FootprintDocument {
    pub fn ast(&self) -> &FootprintAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut FootprintAst {
        self.ast_dirty = true;
        &mut self.ast
    }

    pub fn set_lib_id<S: Into<String>>(&mut self, lib_id: S) -> &mut Self {
        let lib_id = lib_id.into();
        self.mutate_root_items(|items| {
            let value = atom_quoted(lib_id);
            if let Some(current) = items.get(1) {
                if *current == value {
                    false
                } else {
                    items[1] = value;
                    true
                }
            } else {
                items.push(value);
                true
            }
        })
    }

    pub fn set_version(&mut self, version: i32) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "version", atom_symbol(version.to_string()), 2)
        })
    }

    pub fn set_generator<S: Into<String>>(&mut self, generator: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "generator", atom_symbol(generator.into()), 2)
        })
    }

    pub fn set_generator_version<S: Into<String>>(&mut self, generator_version: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(
                items,
                "generator_version",
                atom_quoted(generator_version.into()),
                2,
            )
        })
    }

    pub fn set_layer<S: Into<String>>(&mut self, layer: S) -> &mut Self {
        self.mutate_root_items(|items| upsert_scalar(items, "layer", atom_quoted(layer.into()), 2))
    }

    pub fn set_descr<S: Into<String>>(&mut self, descr: S) -> &mut Self {
        self.mutate_root_items(|items| upsert_scalar(items, "descr", atom_quoted(descr.into()), 2))
    }

    pub fn set_tags<S: Into<String>>(&mut self, tags: S) -> &mut Self {
        self.mutate_root_items(|items| upsert_scalar(items, "tags", atom_quoted(tags.into()), 2))
    }

    pub fn set_reference<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.upsert_property("Reference", value)
    }

    pub fn set_value<S: Into<String>>(&mut self, value: S) -> &mut Self {
        self.upsert_property("Value", value)
    }

    pub fn upsert_property<K: Into<String>, V: Into<String>>(
        &mut self,
        key: K,
        value: V,
    ) -> &mut Self {
        let key = key.into();
        let value = value.into();
        self.mutate_root_items(|items| upsert_property_preserve_tail(items, &key, &value, 2))
    }

    pub fn remove_property(&mut self, key: &str) -> &mut Self {
        let key = key.to_string();
        self.mutate_root_items(|items| remove_property_node(items, &key, 2))
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
            |cst, ast| collect_diagnostics(cst, ast.version),
        );
        self.ast_dirty = false;
        self
    }
}

pub struct FootprintFile;

impl FootprintFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<FootprintDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["footprint", "module"])?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_diagnostics(&cst, ast.version);
        Ok(FootprintDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

fn collect_diagnostics(cst: &CstDocument, version: Option<i32>) -> Vec<Diagnostic> {
    let mut diagnostics = collect_version_diagnostics(version);
    if root_head(cst) == Some("module") {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            code: "legacy_root",
            message: "legacy root token `module` detected; parsing in compatibility mode"
                .to_string(),
            span: None,
            hint: Some("save from newer KiCad to normalize root token to `footprint`".to_string()),
        });
    }
    diagnostics
}

fn parse_ast(cst: &CstDocument) -> FootprintAst {
    let mut lib_id = None;
    let mut version = None;
    let mut tedit = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut layer = None;
    let mut descr = None;
    let mut tags = None;
    let mut property_count = 0usize;
    let mut attr_present = false;
    let mut locked_present = false;
    let mut private_layers_present = false;
    let mut net_tie_pad_groups_present = false;
    let mut embedded_fonts_present = false;
    let mut has_embedded_files = false;
    let mut embedded_file_count = 0usize;
    let mut clearance = None;
    let mut solder_mask_margin = None;
    let mut solder_paste_margin = None;
    let mut solder_paste_margin_ratio = None;
    let mut duplicate_pad_numbers_are_jumpers = None;
    let mut pad_count = 0usize;
    let mut pads = Vec::new();
    let mut model_count = 0usize;
    let mut models = Vec::new();
    let mut zone_count = 0usize;
    let mut zones = Vec::new();
    let mut group_count = 0usize;
    let mut groups = Vec::new();
    let mut fp_line_count = 0usize;
    let mut fp_rect_count = 0usize;
    let mut fp_circle_count = 0usize;
    let mut fp_arc_count = 0usize;
    let mut fp_poly_count = 0usize;
    let mut fp_curve_count = 0usize;
    let mut fp_text_count = 0usize;
    let mut fp_text_box_count = 0usize;
    let mut dimension_count = 0usize;
    let mut graphic_count = 0usize;
    let mut graphics = Vec::new();
    let mut attr = Vec::new();
    let mut locked = false;
    let mut placed = false;
    let mut private_layers = Vec::new();
    let mut net_tie_pad_groups = Vec::new();
    let mut reference = None;
    let mut value = None;
    let mut properties = Vec::new();
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        lib_id = items.get(1).and_then(atom_as_string);
        for item in items.iter().skip(2) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("tedit") => tedit = second_atom_string(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("layer") => layer = second_atom_string(item),
                Some("descr") => descr = second_atom_string(item),
                Some("tags") => tags = second_atom_string(item),
                Some("property") => {
                    property_count += 1;
                    if let Node::List { items: props, .. } = item {
                        let key = props.get(1).and_then(atom_as_string);
                        let val = props.get(2).and_then(atom_as_string);
                        if let (Some(k), Some(v)) = (key.clone(), val.clone()) {
                            properties.push(FpProperty { key: k, value: v });
                        }
                        match key.as_deref() {
                            Some("Reference") => reference = val,
                            Some("Value") => value = val,
                            _ => {}
                        }
                    }
                }
                Some("attr") => {
                    attr_present = true;
                    if let Node::List { items: inner, .. } = item {
                        attr = inner.iter().skip(1).filter_map(atom_as_string).collect();
                    }
                }
                Some("locked") => {
                    locked_present = true;
                    locked = true;
                }
                Some("placed") => placed = true,
                Some("private_layers") => {
                    private_layers_present = true;
                    if let Node::List { items: inner, .. } = item {
                        private_layers = inner.iter().skip(1).filter_map(atom_as_string).collect();
                    }
                }
                Some("net_tie_pad_groups") => {
                    net_tie_pad_groups_present = true;
                    if let Node::List { items: inner, .. } = item {
                        for child in inner.iter().skip(1) {
                            if let Node::List { items: grp, .. } = child {
                                let group: Vec<String> =
                                    grp.iter().filter_map(atom_as_string).collect();
                                if !group.is_empty() {
                                    net_tie_pad_groups.push(group);
                                }
                            }
                        }
                    }
                }
                Some("embedded_fonts") => embedded_fonts_present = true,
                Some("embedded_files") => {
                    has_embedded_files = true;
                    embedded_file_count = list_child_head_count(item, "file");
                }
                Some("clearance") => clearance = second_atom_string(item),
                Some("solder_mask_margin") => solder_mask_margin = second_atom_string(item),
                Some("solder_paste_margin") => solder_paste_margin = second_atom_string(item),
                Some("solder_paste_margin_ratio") => {
                    solder_paste_margin_ratio = second_atom_string(item)
                }
                Some("duplicate_pad_numbers_are_jumpers") => {
                    duplicate_pad_numbers_are_jumpers =
                        second_atom_string(item).and_then(|s| match s.as_str() {
                            "yes" => Some(true),
                            "no" => Some(false),
                            _ => None,
                        })
                }
                Some("pad") => {
                    pad_count += 1;
                    pads.push(parse_fp_pad(item));
                }
                Some("model") => {
                    model_count += 1;
                    models.push(parse_fp_model(item));
                }
                Some("zone") => {
                    zone_count += 1;
                    zones.push(parse_fp_zone(item));
                }
                Some("group") => {
                    group_count += 1;
                    groups.push(parse_fp_group(item));
                }
                Some("fp_line") => {
                    fp_line_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_line"));
                }
                Some("fp_rect") => {
                    fp_rect_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_rect"));
                }
                Some("fp_circle") => {
                    fp_circle_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_circle"));
                }
                Some("fp_arc") => {
                    fp_arc_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_arc"));
                }
                Some("fp_poly") => {
                    fp_poly_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_poly"));
                }
                Some("fp_curve") => {
                    fp_curve_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_curve"));
                }
                Some("fp_text") => {
                    fp_text_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_text"));
                }
                Some("fp_text_box") => {
                    fp_text_box_count += 1;
                    graphic_count += 1;
                    graphics.push(parse_fp_graphic(item, "fp_text_box"));
                }
                Some("dimension") => dimension_count += 1,
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    FootprintAst {
        lib_id,
        version,
        tedit,
        generator,
        generator_version,
        layer,
        descr,
        tags,
        property_count,
        attr_present,
        locked_present,
        private_layers_present,
        net_tie_pad_groups_present,
        embedded_fonts_present,
        has_embedded_files,
        embedded_file_count,
        clearance,
        solder_mask_margin,
        solder_paste_margin,
        solder_paste_margin_ratio,
        duplicate_pad_numbers_are_jumpers,
        pad_count,
        pads,
        model_count,
        models,
        zone_count,
        zones,
        group_count,
        groups,
        fp_line_count,
        fp_rect_count,
        fp_circle_count,
        fp_arc_count,
        fp_poly_count,
        fp_curve_count,
        fp_text_count,
        fp_text_box_count,
        dimension_count,
        graphic_count,
        graphics,
        attr,
        locked,
        placed,
        private_layers,
        net_tie_pad_groups,
        reference,
        value,
        properties,
        unknown_nodes,
    }
}

fn parse_fp_pad(node: &Node) -> FpPad {
    let Node::List { items, .. } = node else {
        return FpPad {
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
                let (xy, rot) = parse_fp_xy_and_angle(child);
                at = xy;
                rotation = rot;
            }
            Some("size") => size = parse_fp_xy(child),
            Some("layers") => {
                if let Node::List { items: inner, .. } = child {
                    layers = inner.iter().skip(1).filter_map(atom_as_string).collect();
                }
            }
            Some("net") => {
                if let Node::List { items: inner, .. } = child {
                    net = Some(FpPadNet {
                        code: inner
                            .get(1)
                            .and_then(atom_as_string)
                            .and_then(|s| s.parse().ok()),
                        name: inner.get(2).and_then(atom_as_string),
                    });
                }
            }
            Some("drill") => drill = Some(parse_fp_drill(child)),
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
                if let Node::List { items: inner, .. } = child {
                    chamfer = inner.iter().skip(1).filter_map(atom_as_string).collect();
                }
            }
            Some("die_length") => die_length = second_atom_f64(child),
            Some("solder_mask_margin") => solder_mask_margin = second_atom_f64(child),
            Some("solder_paste_margin") => solder_paste_margin = second_atom_f64(child),
            Some("solder_paste_margin_ratio") => solder_paste_margin_ratio = second_atom_f64(child),
            Some("clearance") => clearance = second_atom_f64(child),
            Some("zone_connect") => {
                zone_connect = second_atom_string(child).and_then(|s| s.parse().ok())
            }
            Some("thermal_width") => thermal_width = second_atom_f64(child),
            Some("thermal_gap") => thermal_gap = second_atom_f64(child),
            Some("options") => {
                if let Node::List { items: inner, .. } = child {
                    for opt in inner.iter().skip(1) {
                        match head_of(opt) {
                            Some("clearance") => custom_clearance = second_atom_string(opt),
                            Some("anchor") => custom_anchor = second_atom_string(opt),
                            _ => {}
                        }
                    }
                }
            }
            Some("primitives") => {
                if let Node::List { items: inner, .. } = child {
                    custom_primitives = inner.len().saturating_sub(1);
                }
            }
            _ => {}
        }
    }
    FpPad {
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

fn parse_fp_drill(node: &Node) -> FpPadDrill {
    let Node::List { items, .. } = node else {
        return FpPadDrill {
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
                    offset = parse_fp_xy(child);
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
    FpPadDrill {
        shape,
        diameter,
        width,
        offset,
    }
}

fn parse_fp_graphic(node: &Node, token: &str) -> FpGraphic {
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
        if matches!(token, "fp_text" | "fp_text_box") {
            text = items.get(1).and_then(atom_as_string);
        }
        locked = items
            .iter()
            .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "locked"));
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("layer") => layer = second_atom_string(child),
                Some("start") => start = parse_fp_xy(child),
                Some("end") => end = parse_fp_xy(child),
                Some("center") => center = parse_fp_xy(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("locked") => locked = true,
                Some("width") => width = second_atom_f64(child),
                Some("stroke") => {
                    if let Node::List { items: inner, .. } = child {
                        for s in inner.iter().skip(1) {
                            match head_of(s) {
                                Some("width") => width = second_atom_f64(s),
                                Some("type") => stroke_type = second_atom_string(s),
                                _ => {}
                            }
                        }
                    }
                }
                Some("fill") => {
                    if let Node::List { items: inner, .. } = child {
                        for f in inner.iter().skip(1) {
                            if head_of(f) == Some("type") {
                                fill_type = second_atom_string(f);
                            }
                        }
                    }
                }
                Some("at") => {
                    let (xy, rot) = parse_fp_xy_and_angle(child);
                    at = xy;
                    angle = rot;
                }
                Some("effects") => {
                    if let Node::List { items: inner, .. } = child {
                        for e in inner.iter().skip(1) {
                            if head_of(e) == Some("font") {
                                if let Node::List {
                                    items: font_items, ..
                                } = e
                                {
                                    for fi in font_items.iter().skip(1) {
                                        match head_of(fi) {
                                            Some("size") => font_size = parse_fp_xy(fi),
                                            Some("thickness") => {
                                                font_thickness = second_atom_f64(fi)
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    FpGraphic {
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

fn parse_fp_model(node: &Node) -> FpModel {
    let Node::List { items, .. } = node else {
        return FpModel {
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
    let hide = items
        .iter()
        .any(|n| matches!(n, Node::Atom { atom: Atom::Symbol(s), .. } if s == "hide"));
    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("at") | Some("offset") => at = parse_fp_model_xyz(child),
            Some("scale") => scale = parse_fp_model_xyz(child),
            Some("rotate") => rotate = parse_fp_model_xyz(child),
            _ => {}
        }
    }
    FpModel {
        path,
        at,
        scale,
        rotate,
        hide,
    }
}

fn parse_fp_model_xyz(node: &Node) -> Option<[f64; 3]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    for child in items.iter().skip(1) {
        if head_of(child) == Some("xyz") {
            if let Node::List {
                items: xyz_items, ..
            } = child
            {
                let x = xyz_items.get(1).and_then(atom_as_f64)?;
                let y = xyz_items.get(2).and_then(atom_as_f64)?;
                let z = xyz_items.get(3).and_then(atom_as_f64)?;
                return Some([x, y, z]);
            }
        }
    }
    None
}

fn parse_fp_zone(node: &Node) -> FpZone {
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
                Some("net") => net = second_atom_string(child).and_then(|s| s.parse().ok()),
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
    FpZone {
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

fn parse_fp_group(node: &Node) -> FpGroup {
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
    FpGroup {
        name,
        group_id,
        member_count,
    }
}

fn parse_fp_xy(node: &Node) -> Option<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_string)?.parse::<f64>().ok()?;
    let y = items.get(2).and_then(atom_as_string)?.parse::<f64>().ok()?;
    Some([x, y])
}

fn parse_fp_xy_and_angle(node: &Node) -> (Option<[f64; 2]>, Option<f64>) {
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_mod"))
    }

    #[test]
    fn read_footprint_and_preserve_lossless() {
        let path = tmp_file("footprint_read_ok");
        let src = "(footprint \"R_0603\" (version 20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().version, Some(20260101));
        assert_eq!(doc.ast().generator.as_deref(), Some("pcbnew"));
        assert!(doc.ast().unknown_nodes.is_empty());
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_warns_on_future_version() {
        let path = tmp_file("footprint_future");
        fs::write(
            &path,
            "(footprint \"R\" (version 20270101) (generator pcbnew))\n",
        )
        .expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.diagnostics().len(), 1);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_warns_on_legacy_version() {
        let path = tmp_file("footprint_legacy");
        fs::write(
            &path,
            "(footprint \"R\" (version 20221018) (generator pcbnew))\n",
        )
        .expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_format");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_accepts_legacy_module_root() {
        let path = tmp_file("footprint_module_root");
        let src = "(module R_0603 (layer F.Cu) (tedit 5F0C7995) (attr smd))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().tedit.as_deref(), Some("5F0C7995"));
        assert!(doc.ast().attr_present);
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_root");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_captures_unknown_nodes() {
        let path = tmp_file("footprint_unknown");
        let src =
            "(footprint \"R\" (version 20260101) (generator pcbnew) (future_shape foo bar))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);
        assert_eq!(
            doc.ast().unknown_nodes[0].head.as_deref(),
            Some("future_shape")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_parses_top_level_counts() {
        let path = tmp_file("footprint_counts");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew) (generator_version \"10.0\") (layer \"F.Cu\")\n  (descr \"demo\")\n  (tags \"a b\")\n  (property \"Reference\" \"R?\")\n  (property \"Value\" \"X\")\n  (attr smd)\n  (private_layers \"In1.Cu\")\n  (net_tie_pad_groups \"1,2\")\n  (solder_mask_margin 0.02)\n  (solder_paste_margin -0.01)\n  (solder_paste_margin_ratio -0.2)\n  (duplicate_pad_numbers_are_jumpers yes)\n  (fp_text reference \"R1\" (at 0 0) (layer \"F.SilkS\"))\n  (fp_line (start 0 0) (end 1 1) (layer \"F.SilkS\"))\n  (pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\" \"F.Mask\"))\n  (model \"foo.step\")\n  (zone)\n  (group (id \"g1\"))\n  (dimension)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().lib_id.as_deref(), Some("X"));
        assert_eq!(doc.ast().generator_version.as_deref(), Some("10.0"));
        assert_eq!(doc.ast().layer.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().property_count, 2);
        assert!(doc.ast().attr_present);
        assert!(!doc.ast().locked_present);
        assert!(doc.ast().private_layers_present);
        assert!(doc.ast().net_tie_pad_groups_present);
        assert!(!doc.ast().embedded_fonts_present);
        assert!(!doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 0);
        assert_eq!(doc.ast().clearance, None);
        assert_eq!(doc.ast().solder_mask_margin.as_deref(), Some("0.02"));
        assert_eq!(doc.ast().solder_paste_margin.as_deref(), Some("-0.01"));
        assert_eq!(doc.ast().solder_paste_margin_ratio.as_deref(), Some("-0.2"));
        assert_eq!(doc.ast().duplicate_pad_numbers_are_jumpers, Some(true));
        assert_eq!(doc.ast().fp_text_count, 1);
        assert_eq!(doc.ast().fp_line_count, 1);
        assert_eq!(doc.ast().graphic_count, 2);
        assert_eq!(doc.ast().pad_count, 1);
        assert_eq!(doc.ast().model_count, 1);
        assert_eq!(doc.ast().zone_count, 1);
        assert_eq!(doc.ast().group_count, 1);
        assert_eq!(doc.ast().dimension_count, 1);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_embedded_fonts_regression() {
        let path = tmp_file("footprint_embedded_fonts");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew) (embedded_fonts no))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().embedded_fonts_present);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_locked_regression() {
        let path = tmp_file("footprint_locked");
        let src = "(footprint \"X\" (locked) (version 20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().locked_present);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_solder_margins_and_jumpers_regression() {
        let path = tmp_file("footprint_margins_jumpers");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew)\n  (clearance 0.15)\n  (solder_mask_margin 0.03)\n  (solder_paste_margin -0.02)\n  (solder_paste_margin_ratio -0.3)\n  (duplicate_pad_numbers_are_jumpers no)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().clearance.as_deref(), Some("0.15"));
        assert_eq!(doc.ast().solder_mask_margin.as_deref(), Some("0.03"));
        assert_eq!(doc.ast().solder_paste_margin.as_deref(), Some("-0.02"));
        assert_eq!(doc.ast().solder_paste_margin_ratio.as_deref(), Some("-0.3"));
        assert_eq!(doc.ast().duplicate_pad_numbers_are_jumpers, Some(false));
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_embedded_files_regression() {
        let path = tmp_file("footprint_embedded_files");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew)\n  (embedded_files\n    (file \"A\" \"base64\")\n    (file \"B\" \"base64\")\n  )\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 2);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_updates_core_fields_and_properties() {
        let path = tmp_file("footprint_edit_input");
        let src = "(footprint \"Old\" (version 20241229) (generator pcbnew) (layer \"F.Cu\")\n  (property \"Reference\" \"R1\")\n  (property \"Value\" \"10k\")\n  (future_shape foo bar)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FootprintFile::read(&path).expect("read");
        doc.set_lib_id("New_Footprint")
            .set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .set_layer("B.Cu")
            .set_descr("demo footprint")
            .set_tags("r c passives")
            .set_reference("R99")
            .set_value("22k")
            .upsert_property("LCSC", "C1234")
            .remove_property("DoesNotExist");

        let out = tmp_file("footprint_edit_output");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert!(written.contains("(future_shape foo bar)"));
        assert!(written.contains("(property \"LCSC\" \"C1234\")"));

        let reread = FootprintFile::read(&out).expect("reread");
        assert_eq!(reread.ast().lib_id.as_deref(), Some("New_Footprint"));
        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(reread.ast().layer.as_deref(), Some("B.Cu"));
        assert_eq!(reread.ast().descr.as_deref(), Some("demo footprint"));
        assert_eq!(reread.ast().tags.as_deref(), Some("r c passives"));
        assert_eq!(reread.ast().property_count, 3);
        assert_eq!(reread.ast().unknown_nodes.len(), 1);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn remove_property_roundtrip_removes_entry() {
        let path = tmp_file("footprint_remove_property");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew)\n  (property \"Reference\" \"R1\")\n  (property \"Value\" \"10k\")\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FootprintFile::read(&path).expect("read");
        doc.remove_property("Value");

        let out = tmp_file("footprint_remove_property_out");
        doc.write(&out).expect("write");
        let reread = FootprintFile::read(&out).expect("reread");
        assert_eq!(reread.ast().property_count, 1);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn no_op_setter_keeps_lossless_raw_unchanged() {
        let path = tmp_file("footprint_noop_setter");
        let src = "(footprint \"X\" (version   20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FootprintFile::read(&path).expect("read");
        doc.set_version(20260101);

        let out = tmp_file("footprint_noop_setter_out");
        doc.write(&out).expect("write");
        let written = fs::read_to_string(&out).expect("read out");
        assert_eq!(written, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

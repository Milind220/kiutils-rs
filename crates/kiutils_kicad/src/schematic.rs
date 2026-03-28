use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sections::{parse_paper, parse_title_block, ParsedPaper, ParsedTitleBlock};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, find_property_index, mutate_root_and_refresh,
    paper_standard_node, paper_user_node, remove_property, upsert_node,
    upsert_property_preserve_tail, upsert_scalar, upsert_section_child_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, atom_as_string, head_of, list_child_head_count, second_atom_bool, second_atom_f64,
    second_atom_i32, second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicPaper {
    pub kind: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub orientation: Option<String>,
}

impl From<ParsedPaper> for SchematicPaper {
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
pub struct SchematicTitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<String>,
}

impl From<ParsedTitleBlock> for SchematicTitleBlock {
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

/// Symbol instance details embedded in a schematic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSymbolInfo {
    pub reference: Option<String>,
    pub lib_id: Option<String>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    /// All properties as (key, value) pairs.
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicJunction {
    pub at: Option<[f64; 2]>,
    pub diameter: Option<f64>,
    pub color: Option<String>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicNoConnect {
    pub at: Option<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicWire {
    pub points: Vec<[f64; 2]>,
    pub uuid: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicBus {
    pub points: Vec<[f64; 2]>,
    pub uuid: Option<String>,
    pub stroke_width: Option<f64>,
    pub stroke_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicBusEntry {
    pub at: Option<[f64; 2]>,
    pub size: Option<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicBusAlias {
    pub name: Option<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicNetclassFlag {
    pub text: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicPolyline {
    pub points: Vec<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicRectangle {
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicCircle {
    pub center: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicArc {
    pub start: Option<[f64; 2]>,
    pub mid: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicRuleArea {
    pub name: Option<String>,
    pub points: Vec<[f64; 2]>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicText {
    pub text: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicLabel {
    pub label_type: String,
    pub text: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub uuid: Option<String>,
    pub shape: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSymbol {
    pub lib_id: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub mirror: Option<String>,
    pub unit: Option<i32>,
    pub uuid: Option<String>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub dnp: bool,
    pub fields_autoplaced: bool,
    pub properties: Vec<(String, String)>,
    pub pin_count: usize,
    pub reference: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSheet {
    pub at: Option<[f64; 2]>,
    pub size: Option<[f64; 2]>,
    pub uuid: Option<String>,
    pub fields_autoplaced: bool,
    pub name: Option<String>,
    pub filename: Option<String>,
    pub pin_count: usize,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicImage {
    pub at: Option<[f64; 2]>,
    pub scale: Option<f64>,
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSymbolInstance {
    pub path: Option<String>,
    pub reference: Option<String>,
    pub unit: Option<i32>,
    pub value: Option<String>,
    pub footprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSheetInstance {
    pub path: Option<String>,
    pub page: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub uuid: Option<String>,
    pub has_paper: bool,
    pub paper: Option<SchematicPaper>,
    pub has_title_block: bool,
    pub title_block: Option<SchematicTitleBlock>,
    pub has_lib_symbols: bool,
    pub embedded_fonts: Option<bool>,
    pub lib_symbol_count: usize,
    pub symbol_count: usize,
    pub symbols: Vec<SchematicSymbol>,
    pub sheet_count: usize,
    pub sheets: Vec<SchematicSheet>,
    pub junction_count: usize,
    pub junctions: Vec<SchematicJunction>,
    pub no_connect_count: usize,
    pub no_connects: Vec<SchematicNoConnect>,
    pub bus_entry_count: usize,
    pub bus_entries: Vec<SchematicBusEntry>,
    pub bus_alias_count: usize,
    pub bus_aliases: Vec<SchematicBusAlias>,
    pub wire_count: usize,
    pub wires: Vec<SchematicWire>,
    pub bus_count: usize,
    pub buses: Vec<SchematicBus>,
    pub image_count: usize,
    pub images: Vec<SchematicImage>,
    pub text_count: usize,
    pub texts: Vec<SchematicText>,
    pub text_box_count: usize,
    pub label_count: usize,
    pub labels: Vec<SchematicLabel>,
    pub global_label_count: usize,
    // global labels go into labels vec with label_type
    pub hierarchical_label_count: usize,
    // hierarchical labels go into labels vec with label_type
    pub netclass_flag_count: usize,
    pub netclass_flags: Vec<SchematicNetclassFlag>,
    pub polyline_count: usize,
    pub polylines: Vec<SchematicPolyline>,
    pub rectangle_count: usize,
    pub rectangles: Vec<SchematicRectangle>,
    pub circle_count: usize,
    pub circles: Vec<SchematicCircle>,
    pub arc_count: usize,
    pub arcs: Vec<SchematicArc>,
    pub rule_area_count: usize,
    pub rule_areas: Vec<SchematicRuleArea>,
    pub sheet_instance_count: usize,
    pub sheet_instances: Vec<SchematicSheetInstance>,
    pub symbol_instance_count: usize,
    pub symbol_instances_parsed: Vec<SchematicSymbolInstance>,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct SchematicDocument {
    ast: SchematicAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

impl SchematicDocument {
    pub fn ast(&self) -> &SchematicAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut SchematicAst {
        self.ast_dirty = true;
        &mut self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn set_version(&mut self, version: i32) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "version", atom_symbol(version.to_string()), 1)
        })
    }

    pub fn set_generator<S: Into<String>>(&mut self, generator: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_scalar(items, "generator", atom_quoted(generator.into()), 1)
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

    pub fn set_uuid<S: Into<String>>(&mut self, uuid: S) -> &mut Self {
        self.mutate_root_items(|items| upsert_scalar(items, "uuid", atom_quoted(uuid.into()), 1))
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

    pub fn set_embedded_fonts(&mut self, enabled: bool) -> &mut Self {
        let value = if enabled { "yes" } else { "no" };
        self.mutate_root_items(|items| {
            upsert_scalar(items, "embedded_fonts", atom_symbol(value.to_string()), 1)
        })
    }

    /// Return filenames of sub-sheets referenced by `(sheet ...)` nodes.
    ///
    /// The filenames come from the `Sheetfile` property on each sheet node
    /// and are relative to the directory containing this schematic.
    pub fn sheet_filenames(&self) -> Vec<String> {
        let items = match self.cst.nodes.first() {
            Some(Node::List { items, .. }) => items,
            _ => return Vec::new(),
        };
        items
            .iter()
            .skip(1)
            .filter(|node| head_of(node) == Some("sheet"))
            .filter_map(|node| {
                let Node::List {
                    items: sheet_items, ..
                } = node
                else {
                    return None;
                };
                // Look for (property "Sheetfile" "filename.kicad_sch" ...)
                find_property_index(sheet_items, "Sheetfile", 1).and_then(|idx| {
                    if let Some(Node::List {
                        items: prop_items, ..
                    }) = sheet_items.get(idx)
                    {
                        prop_items.get(2).and_then(atom_as_string)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Return info for all symbol instances in the schematic.
    pub fn symbol_instances(&self) -> Vec<SchematicSymbolInfo> {
        let items = match self.cst.nodes.first() {
            Some(Node::List { items, .. }) => items,
            _ => return Vec::new(),
        };
        items
            .iter()
            .skip(1)
            .filter(|node| head_of(node) == Some("symbol"))
            .map(parse_schematic_symbol_info)
            .collect()
    }

    /// Upsert a property on every symbol instance matching `reference`.
    pub fn upsert_symbol_instance_property<R: Into<String>, K: Into<String>, V: Into<String>>(
        &mut self,
        reference: R,
        key: K,
        value: V,
    ) -> &mut Self {
        let reference = reference.into();
        let key = key.into();
        let value = value.into();
        self.mutate_root_items(|items| {
            let indices = find_schematic_symbol_indices_by_reference(items, &reference);
            let mut changed = false;
            for idx in indices {
                if let Some(Node::List {
                    items: sym_items, ..
                }) = items.get_mut(idx)
                {
                    if upsert_property_preserve_tail(sym_items, &key, &value, 1) {
                        changed = true;
                    }
                }
            }
            changed
        })
    }

    /// Remove a property from every symbol instance matching `reference`.
    pub fn remove_symbol_instance_property<R: Into<String>, K: Into<String>>(
        &mut self,
        reference: R,
        key: K,
    ) -> &mut Self {
        let reference = reference.into();
        let key = key.into();
        self.mutate_root_items(|items| {
            let indices = find_schematic_symbol_indices_by_reference(items, &reference);
            let mut changed = false;
            for idx in indices {
                if let Some(Node::List {
                    items: sym_items, ..
                }) = items.get_mut(idx)
                {
                    if remove_property(sym_items, &key, 1) {
                        changed = true;
                    }
                }
            }
            changed
        })
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
            |_cst, ast| collect_version_diagnostics(ast.version),
        );
        self.ast_dirty = false;
        self
    }
}

pub struct SchematicFile;

impl SchematicFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<SchematicDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["kicad_sch"])?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_version_diagnostics(ast.version);
        Ok(SchematicDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

/// Find indices of root-level `(symbol ...)` nodes whose "Reference" property matches.
fn find_schematic_symbol_indices_by_reference(items: &[Node], reference: &str) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, node)| {
            if head_of(node) != Some("symbol") {
                return false;
            }
            let Node::List {
                items: sym_items, ..
            } = node
            else {
                return false;
            };
            if let Some(prop_idx) = find_property_index(sym_items, "Reference", 1) {
                if let Some(Node::List {
                    items: prop_items, ..
                }) = sym_items.get(prop_idx)
                {
                    return prop_items.get(2).and_then(atom_as_string).as_deref()
                        == Some(reference);
                }
            }
            false
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Extract property value from a symbol node's items.
fn get_property_value(sym_items: &[Node], key: &str) -> Option<String> {
    find_property_index(sym_items, key, 1).and_then(|idx| {
        if let Some(Node::List {
            items: prop_items, ..
        }) = sym_items.get(idx)
        {
            prop_items.get(2).and_then(atom_as_string)
        } else {
            None
        }
    })
}

fn parse_schematic_symbol_info(node: &Node) -> SchematicSymbolInfo {
    let Node::List { items, .. } = node else {
        return SchematicSymbolInfo {
            reference: None,
            lib_id: None,
            value: None,
            footprint: None,
            properties: Vec::new(),
        };
    };

    let lib_id = items
        .iter()
        .skip(1)
        .find(|n| head_of(n) == Some("lib_id"))
        .and_then(second_atom_string);

    let reference = get_property_value(items, "Reference");
    let value = get_property_value(items, "Value");
    let footprint = get_property_value(items, "Footprint");

    let properties: Vec<(String, String)> = items
        .iter()
        .skip(1)
        .filter(|n| head_of(n) == Some("property"))
        .filter_map(|n| {
            let Node::List {
                items: prop_items, ..
            } = n
            else {
                return None;
            };
            let key = prop_items.get(1).and_then(atom_as_string)?;
            let val = prop_items
                .get(2)
                .and_then(atom_as_string)
                .unwrap_or_default();
            Some((key, val))
        })
        .collect();

    SchematicSymbolInfo {
        reference,
        lib_id,
        value,
        footprint,
        properties,
    }
}

fn parse_xy2(node: &Node) -> Option<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_f64)?;
    let y = items.get(2).and_then(atom_as_f64)?;
    Some([x, y])
}

fn parse_at_and_angle(node: &Node) -> (Option<[f64; 2]>, Option<f64>) {
    let Node::List { items, .. } = node else {
        return (None, None);
    };
    let at = match (
        items.get(1).and_then(atom_as_f64),
        items.get(2).and_then(atom_as_f64),
    ) {
        (Some(x), Some(y)) => Some([x, y]),
        _ => None,
    };
    let angle = items.get(3).and_then(atom_as_f64);
    (at, angle)
}

fn parse_pts(node: &Node) -> Vec<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return Vec::new();
    };
    items
        .iter()
        .skip(1)
        .filter(|n| head_of(n) == Some("xy"))
        .filter_map(parse_xy2)
        .collect()
}

fn parse_property_pairs(items: &[Node]) -> Vec<(String, String)> {
    items
        .iter()
        .skip(1)
        .filter(|n| head_of(n) == Some("property"))
        .filter_map(|n| {
            let Node::List {
                items: prop_items, ..
            } = n
            else {
                return None;
            };
            let key = prop_items.get(1).and_then(atom_as_string)?;
            let val = prop_items
                .get(2)
                .and_then(atom_as_string)
                .unwrap_or_default();
            Some((key, val))
        })
        .collect()
}

fn parse_stroke(node: &Node) -> (Option<f64>, Option<String>) {
    let mut width = None;
    let mut stroke_type = None;
    let Node::List { items, .. } = node else {
        return (width, stroke_type);
    };
    for child in items.iter().skip(1) {
        match head_of(child) {
            Some("width") => width = second_atom_f64(child),
            Some("type") => stroke_type = second_atom_string(child),
            _ => {}
        }
    }
    (width, stroke_type)
}

fn parse_junction(node: &Node) -> SchematicJunction {
    let mut at = None;
    let mut diameter = None;
    let mut color = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy2(child),
                Some("diameter") => diameter = second_atom_f64(child),
                Some("color") => {
                    if let Node::List {
                        items: color_items, ..
                    } = child
                    {
                        let parts: Vec<String> = color_items
                            .iter()
                            .skip(1)
                            .filter_map(atom_as_string)
                            .collect();
                        if !parts.is_empty() {
                            color = Some(parts.join(" "));
                        }
                    }
                }
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicJunction {
        at,
        diameter,
        color,
        uuid,
    }
}

fn parse_no_connect(node: &Node) -> SchematicNoConnect {
    let mut at = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicNoConnect { at, uuid }
}

fn parse_wire(node: &Node) -> SchematicWire {
    let mut points = Vec::new();
    let mut uuid = None;
    let mut stroke_width = None;
    let mut stroke_type = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("pts") => points = parse_pts(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("stroke") => (stroke_width, stroke_type) = parse_stroke(child),
                _ => {}
            }
        }
    }
    SchematicWire {
        points,
        uuid,
        stroke_width,
        stroke_type,
    }
}

fn parse_bus(node: &Node) -> SchematicBus {
    let mut points = Vec::new();
    let mut uuid = None;
    let mut stroke_width = None;
    let mut stroke_type = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("pts") => points = parse_pts(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("stroke") => (stroke_width, stroke_type) = parse_stroke(child),
                _ => {}
            }
        }
    }
    SchematicBus {
        points,
        uuid,
        stroke_width,
        stroke_type,
    }
}

fn parse_bus_entry(node: &Node) -> SchematicBusEntry {
    let mut at = None;
    let mut size = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy2(child),
                Some("size") => size = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicBusEntry { at, size, uuid }
}

fn parse_bus_alias(node: &Node) -> SchematicBusAlias {
    let mut name = None;
    let mut members = Vec::new();
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("name") => name = second_atom_string(child),
                Some("members") => {
                    if let Node::List {
                        items: member_items,
                        ..
                    } = child
                    {
                        members = member_items
                            .iter()
                            .skip(1)
                            .filter_map(atom_as_string)
                            .collect();
                    }
                }
                _ => {}
            }
        }
    }
    SchematicBusAlias { name, members }
}

fn parse_netclass_flag(node: &Node) -> SchematicNetclassFlag {
    let mut text = second_atom_string(node);
    let mut at = None;
    let mut angle = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => (at, angle) = parse_at_and_angle(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
        if text.is_none() {
            text = items.get(1).and_then(atom_as_string);
        }
    }
    SchematicNetclassFlag {
        text,
        at,
        angle,
        uuid,
    }
}

fn parse_polyline(node: &Node) -> SchematicPolyline {
    let mut points = Vec::new();
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("pts") => points = parse_pts(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicPolyline { points, uuid }
}

fn parse_rectangle(node: &Node) -> SchematicRectangle {
    let mut start = None;
    let mut end = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy2(child),
                Some("end") => end = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicRectangle { start, end, uuid }
}

fn parse_circle(node: &Node) -> SchematicCircle {
    let mut center = None;
    let mut end = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("center") => center = parse_xy2(child),
                Some("end") => end = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicCircle { center, end, uuid }
}

fn parse_arc(node: &Node) -> SchematicArc {
    let mut start = None;
    let mut mid = None;
    let mut end = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_xy2(child),
                Some("mid") => mid = parse_xy2(child),
                Some("end") => end = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicArc {
        start,
        mid,
        end,
        uuid,
    }
}

fn parse_rule_area(node: &Node) -> SchematicRuleArea {
    let mut name = None;
    let mut points = Vec::new();
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("name") => name = second_atom_string(child),
                Some("pts") => points = parse_pts(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicRuleArea { name, points, uuid }
}

fn parse_text(node: &Node) -> SchematicText {
    let mut text = second_atom_string(node);
    let mut at = None;
    let mut angle = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => (at, angle) = parse_at_and_angle(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
        if text.is_none() {
            text = items.get(1).and_then(atom_as_string);
        }
    }
    SchematicText {
        text,
        at,
        angle,
        uuid,
    }
}

fn parse_label(node: &Node, label_type: &str) -> SchematicLabel {
    let mut text = second_atom_string(node);
    let mut at = None;
    let mut angle = None;
    let mut uuid = None;
    let mut shape = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => (at, angle) = parse_at_and_angle(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("shape") => shape = second_atom_string(child),
                _ => {}
            }
        }
        if text.is_none() {
            text = items.get(1).and_then(atom_as_string);
        }
    }
    SchematicLabel {
        label_type: label_type.to_string(),
        text,
        at,
        angle,
        uuid,
        shape,
    }
}

fn parse_symbol(node: &Node) -> SchematicSymbol {
    let mut lib_id = None;
    let mut at = None;
    let mut angle = None;
    let mut mirror = None;
    let mut unit = None;
    let mut uuid = None;
    let mut in_bom = None;
    let mut on_board = None;
    let mut dnp = false;
    let mut fields_autoplaced = false;
    let mut pin_count = 0usize;
    let mut reference = None;
    let mut value = None;
    let mut properties = Vec::new();

    if let Node::List { items, .. } = node {
        pin_count = list_child_head_count(node, "pin");
        reference = get_property_value(items, "Reference");
        value = get_property_value(items, "Value");
        properties = parse_property_pairs(items);

        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("lib_id") => lib_id = second_atom_string(child),
                Some("at") => (at, angle) = parse_at_and_angle(child),
                Some("mirror") => mirror = second_atom_string(child),
                Some("unit") => unit = second_atom_i32(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("in_bom") => in_bom = second_atom_bool(child),
                Some("on_board") => on_board = second_atom_bool(child),
                Some("dnp") => dnp = second_atom_bool(child).unwrap_or(true),
                Some("fields_autoplaced") => fields_autoplaced = true,
                _ => {
                    if matches!(
                        child,
                        Node::Atom {
                            atom: Atom::Symbol(s),
                            ..
                        } if s == "fields_autoplaced"
                    ) {
                        fields_autoplaced = true;
                    }
                }
            }
        }
    }

    SchematicSymbol {
        lib_id,
        at,
        angle,
        mirror,
        unit,
        uuid,
        in_bom,
        on_board,
        dnp,
        fields_autoplaced,
        properties,
        pin_count,
        reference,
        value,
    }
}

fn parse_sheet(node: &Node) -> SchematicSheet {
    let mut at = None;
    let mut size = None;
    let mut uuid = None;
    let mut fields_autoplaced = false;
    let mut pin_count = 0usize;
    let mut properties = Vec::new();
    if let Node::List { items, .. } = node {
        pin_count = list_child_head_count(node, "pin");
        properties = parse_property_pairs(items);
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy2(child),
                Some("size") => size = parse_xy2(child),
                Some("uuid") => uuid = second_atom_string(child),
                Some("fields_autoplaced") => fields_autoplaced = true,
                _ => {
                    if matches!(
                        child,
                        Node::Atom {
                            atom: Atom::Symbol(s),
                            ..
                        } if s == "fields_autoplaced"
                    ) {
                        fields_autoplaced = true;
                    }
                }
            }
        }
    }
    let name = properties
        .iter()
        .find(|(k, _)| k == "Sheetname")
        .map(|(_, v)| v.clone());
    let filename = properties
        .iter()
        .find(|(k, _)| k == "Sheetfile")
        .map(|(_, v)| v.clone());

    SchematicSheet {
        at,
        size,
        uuid,
        fields_autoplaced,
        name,
        filename,
        pin_count,
        properties,
    }
}

fn parse_image(node: &Node) -> SchematicImage {
    let mut at = None;
    let mut scale = None;
    let mut uuid = None;
    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("at") => at = parse_xy2(child),
                Some("scale") => scale = second_atom_f64(child),
                Some("uuid") => uuid = second_atom_string(child),
                _ => {}
            }
        }
    }
    SchematicImage { at, scale, uuid }
}

fn parse_sheet_instance(node: &Node) -> SchematicSheetInstance {
    let Node::List { items, .. } = node else {
        return SchematicSheetInstance {
            path: None,
            page: None,
        };
    };

    let path = items.get(1).and_then(atom_as_string);
    let page = items
        .iter()
        .skip(2)
        .find(|n| head_of(n) == Some("page"))
        .and_then(second_atom_string);

    SchematicSheetInstance { path, page }
}

fn parse_symbol_instance(node: &Node) -> SchematicSymbolInstance {
    let Node::List { items, .. } = node else {
        return SchematicSymbolInstance {
            path: None,
            reference: None,
            unit: None,
            value: None,
            footprint: None,
        };
    };

    let mut reference = None;
    let mut unit = None;
    let mut value = None;
    let mut footprint = None;

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("reference") => reference = second_atom_string(child),
            Some("unit") => unit = second_atom_i32(child),
            Some("value") => value = second_atom_string(child),
            Some("footprint") => footprint = second_atom_string(child),
            _ => {}
        }
    }

    SchematicSymbolInstance {
        path: items.get(1).and_then(atom_as_string),
        reference,
        unit,
        value,
        footprint,
    }
}

fn parse_ast(cst: &CstDocument) -> SchematicAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut uuid = None;
    let mut has_paper = false;
    let mut paper = None;
    let mut has_title_block = false;
    let mut title_block = None;
    let mut has_lib_symbols = false;
    let mut embedded_fonts = None;
    let mut lib_symbol_count = 0usize;
    let mut symbol_count = 0usize;
    let mut symbols = Vec::new();
    let mut sheet_count = 0usize;
    let mut sheets = Vec::new();
    let mut junction_count = 0usize;
    let mut junctions = Vec::new();
    let mut no_connect_count = 0usize;
    let mut no_connects = Vec::new();
    let mut bus_entry_count = 0usize;
    let mut bus_entries = Vec::new();
    let mut bus_alias_count = 0usize;
    let mut bus_aliases = Vec::new();
    let mut wire_count = 0usize;
    let mut wires = Vec::new();
    let mut bus_count = 0usize;
    let mut buses = Vec::new();
    let mut image_count = 0usize;
    let mut images = Vec::new();
    let mut text_count = 0usize;
    let mut texts = Vec::new();
    let mut text_box_count = 0usize;
    let mut label_count = 0usize;
    let mut labels = Vec::new();
    let mut global_label_count = 0usize;
    let mut hierarchical_label_count = 0usize;
    let mut netclass_flag_count = 0usize;
    let mut netclass_flags = Vec::new();
    let mut polyline_count = 0usize;
    let mut polylines = Vec::new();
    let mut rectangle_count = 0usize;
    let mut rectangles = Vec::new();
    let mut circle_count = 0usize;
    let mut circles = Vec::new();
    let mut arc_count = 0usize;
    let mut arcs = Vec::new();
    let mut rule_area_count = 0usize;
    let mut rule_areas = Vec::new();
    let mut sheet_instance_count = 0usize;
    let mut sheet_instances = Vec::new();
    let mut symbol_instance_count = 0usize;
    let mut symbol_instances_parsed = Vec::new();
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("uuid") => uuid = second_atom_string(item),
                Some("paper") => {
                    has_paper = true;
                    paper = Some(parse_paper(item).into());
                }
                Some("title_block") => {
                    has_title_block = true;
                    title_block = Some(parse_title_block(item).into());
                }
                Some("lib_symbols") => {
                    has_lib_symbols = true;
                    lib_symbol_count = list_child_head_count(item, "symbol");
                }
                Some("symbol") => {
                    symbol_count += 1;
                    symbols.push(parse_symbol(item));
                }
                Some("sheet") => {
                    sheet_count += 1;
                    sheets.push(parse_sheet(item));
                }
                Some("junction") => {
                    junction_count += 1;
                    junctions.push(parse_junction(item));
                }
                Some("no_connect") => {
                    no_connect_count += 1;
                    no_connects.push(parse_no_connect(item));
                }
                Some("bus_entry") => {
                    bus_entry_count += 1;
                    bus_entries.push(parse_bus_entry(item));
                }
                Some("bus_alias") => {
                    bus_alias_count += 1;
                    bus_aliases.push(parse_bus_alias(item));
                }
                Some("wire") => {
                    wire_count += 1;
                    wires.push(parse_wire(item));
                }
                Some("bus") => {
                    bus_count += 1;
                    buses.push(parse_bus(item));
                }
                Some("image") => {
                    image_count += 1;
                    images.push(parse_image(item));
                }
                Some("text") => {
                    text_count += 1;
                    texts.push(parse_text(item));
                }
                Some("text_box") => text_box_count += 1,
                Some("label") => {
                    label_count += 1;
                    labels.push(parse_label(item, "label"));
                }
                Some("global_label") => {
                    global_label_count += 1;
                    labels.push(parse_label(item, "global_label"));
                }
                Some("hierarchical_label") => {
                    hierarchical_label_count += 1;
                    labels.push(parse_label(item, "hierarchical_label"));
                }
                Some("netclass_flag") => {
                    netclass_flag_count += 1;
                    netclass_flags.push(parse_netclass_flag(item));
                }
                Some("polyline") => {
                    polyline_count += 1;
                    polylines.push(parse_polyline(item));
                }
                Some("rectangle") => {
                    rectangle_count += 1;
                    rectangles.push(parse_rectangle(item));
                }
                Some("circle") => {
                    circle_count += 1;
                    circles.push(parse_circle(item));
                }
                Some("arc") => {
                    arc_count += 1;
                    arcs.push(parse_arc(item));
                }
                Some("rule_area") => {
                    rule_area_count += 1;
                    rule_areas.push(parse_rule_area(item));
                }
                Some("sheet_instances") => {
                    sheet_instance_count = list_child_head_count(item, "path");
                    if let Node::List {
                        items: section_items,
                        ..
                    } = item
                    {
                        sheet_instances = section_items
                            .iter()
                            .skip(1)
                            .filter(|n| head_of(n) == Some("path"))
                            .map(parse_sheet_instance)
                            .collect();
                    }
                }
                Some("symbol_instances") => {
                    symbol_instance_count = list_child_head_count(item, "path");
                    if let Node::List {
                        items: section_items,
                        ..
                    } = item
                    {
                        symbol_instances_parsed = section_items
                            .iter()
                            .skip(1)
                            .filter(|n| head_of(n) == Some("path"))
                            .map(parse_symbol_instance)
                            .collect();
                    }
                }
                Some("embedded_fonts") => {
                    embedded_fonts = second_atom_bool(item);
                }
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    SchematicAst {
        version,
        generator,
        generator_version,
        uuid,
        has_paper,
        paper,
        has_title_block,
        title_block,
        has_lib_symbols,
        embedded_fonts,
        lib_symbol_count,
        symbol_count,
        symbols,
        sheet_count,
        sheets,
        junction_count,
        junctions,
        no_connect_count,
        no_connects,
        bus_entry_count,
        bus_entries,
        bus_alias_count,
        bus_aliases,
        wire_count,
        wires,
        bus_count,
        buses,
        image_count,
        images,
        text_count,
        texts,
        text_box_count,
        label_count,
        labels,
        global_label_count,
        hierarchical_label_count,
        netclass_flag_count,
        netclass_flags,
        polyline_count,
        polylines,
        rectangle_count,
        rectangles,
        circle_count,
        circles,
        arc_count,
        arcs,
        rule_area_count,
        rule_areas,
        sheet_instance_count,
        sheet_instances,
        symbol_instance_count,
        symbol_instances_parsed,
        unknown_nodes,
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_sch"))
    }

    #[test]
    fn read_schematic_and_preserve_lossless() {
        let path = tmp_file("sch_read_ok");
        let src = "(kicad_sch (version 20250114) (generator \"eeschema\") (generator_version \"9.0\") (uuid \"u-1\") (paper \"A4\") (title_block (title \"Demo\") (date \"2026-02-25\") (comment 2 \"c2\") (comment 1 \"c1\")) (lib_symbols (symbol \"Lib:R\")) (symbol (lib_id \"Lib:R\")) (wire (pts (xy 0 0) (xy 1 1))) (sheet_instances (path \"/\" (page \"1\"))) (embedded_fonts no))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SchematicFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20250114));
        assert_eq!(doc.ast().generator.as_deref(), Some("eeschema"));
        assert_eq!(doc.ast().generator_version.as_deref(), Some("9.0"));
        assert_eq!(doc.ast().uuid.as_deref(), Some("u-1"));
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("A4".to_string())
        );
        assert_eq!(doc.ast().lib_symbol_count, 1);
        assert_eq!(doc.ast().symbol_count, 1);
        assert_eq!(doc.ast().wire_count, 1);
        assert_eq!(doc.ast().sheet_instance_count, 1);
        assert_eq!(doc.ast().embedded_fonts, Some(false));
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn captures_unknown_nodes_roundtrip() {
        let path = tmp_file("sch_unknown");
        let src = "(kicad_sch (version 20250114) (generator \"eeschema\") (future_block 1 2) (symbol (lib_id \"Device:R\")))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SchematicFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);

        let out = tmp_file("sch_unknown_out");
        doc.write(&out).expect("write");
        let got = fs::read_to_string(&out).expect("read out");
        assert_eq!(got, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn edit_roundtrip_updates_core_fields() {
        let path = tmp_file("sch_edit");
        let src = "(kicad_sch (version 20241229) (generator \"eeschema\") (paper \"A4\") (title_block (title \"Old\") (date \"2025-01-01\") (rev \"A\") (company \"OldCo\")) (future_token 1 2))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = SchematicFile::read(&path).expect("read");
        doc.set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .set_uuid("uuid-new")
            .set_paper_user(297.0, 210.0, Some("landscape"))
            .set_title("New")
            .set_date("2026-02-25")
            .set_revision("B")
            .set_company("Lords")
            .set_embedded_fonts(true);

        let out = tmp_file("sch_edit_out");
        doc.write(&out).expect("write");
        let reread = SchematicFile::read(&out).expect("reread");

        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(reread.ast().uuid.as_deref(), Some("uuid-new"));
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("User".to_string())
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.width),
            Some(297.0)
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.height),
            Some(210.0)
        );
        assert_eq!(reread.ast().embedded_fonts, Some(true));
        assert_eq!(reread.ast().unknown_nodes.len(), 1);
        assert_eq!(
            reread
                .ast()
                .title_block
                .as_ref()
                .and_then(|t| t.title.clone()),
            Some("New".to_string())
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

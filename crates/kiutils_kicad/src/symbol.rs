use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, mutate_root_and_refresh, remove_property,
    upsert_property_preserve_tail, upsert_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, atom_as_string, head_of, second_atom_bool, second_atom_f64, second_atom_i32,
    second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymProperty {
    pub key: String,
    pub value: String,
    pub id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymPin {
    pub electrical_type: Option<String>,
    pub graphic_style: Option<String>,
    pub at: Option<[f64; 2]>,
    pub angle: Option<f64>,
    pub length: Option<f64>,
    pub name: Option<String>,
    pub number: Option<String>,
    pub hide: bool,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymGraphic {
    pub token: String,
    pub start: Option<[f64; 2]>,
    pub end: Option<[f64; 2]>,
    pub center: Option<[f64; 2]>,
    pub radius: Option<f64>,
    pub stroke_width: Option<f64>,
    pub stroke_type: Option<String>,
    pub fill_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymUnit {
    pub name: Option<String>,
    pub pin_count: usize,
    pub pins: Vec<SymPin>,
    pub graphic_count: usize,
    pub graphics: Vec<SymGraphic>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Symbol {
    pub name: Option<String>,
    pub property_count: usize,
    pub pin_count: usize,
    pub unit_count: usize,
    pub has_embedded_fonts: bool,
    pub properties: Vec<SymProperty>,
    pub pins: Vec<SymPin>,
    pub units: Vec<SymUnit>,
    pub graphics: Vec<SymGraphic>,
    pub graphic_count: usize,
    pub extends: Option<String>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub pin_names_hide: bool,
    pub pin_names_offset: Option<f64>,
    pub pin_numbers_hide: bool,
    pub power: bool,
    pub exclude_from_sim: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolLibAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub symbol_count: usize,
    pub total_property_count: usize,
    pub total_pin_count: usize,
    pub symbols: Vec<Symbol>,
    pub unknown_nodes: Vec<UnknownNode>,
}
#[derive(Debug, Clone)]
pub struct SymbolLibDocument {
    ast: SymbolLibAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

impl SymbolLibDocument {
    pub fn ast(&self) -> &SymbolLibAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut SymbolLibAst {
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

    pub fn rename_symbol<S: Into<String>>(&mut self, from: &str, to: S) -> &mut Self {
        let from = from.to_string();
        let to = to.into();
        self.mutate_root_items(|items| {
            if let Some(idx) = find_symbol_index(items, &from) {
                if let Some(Node::List {
                    items: symbol_items,
                    ..
                }) = items.get_mut(idx)
                {
                    if symbol_items.len() > 1 {
                        let next = atom_quoted(to);
                        if symbol_items[1] == next {
                            false
                        } else {
                            symbol_items[1] = next;
                            true
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    pub fn rename_first_symbol<S: Into<String>>(&mut self, to: S) -> &mut Self {
        let to = to.into();
        self.mutate_root_items(|items| {
            let Some(idx) = items
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, n)| head_of(n) == Some("symbol"))
                .map(|(idx, _)| idx)
            else {
                return false;
            };
            let Some(Node::List {
                items: symbol_items,
                ..
            }) = items.get_mut(idx)
            else {
                return false;
            };
            if symbol_items.len() > 1 {
                let next = atom_quoted(to);
                if symbol_items[1] == next {
                    false
                } else {
                    symbol_items[1] = next;
                    true
                }
            } else {
                false
            }
        })
    }

    pub fn upsert_symbol_property(
        &mut self,
        symbol_name: &str,
        key: &str,
        value: &str,
    ) -> &mut Self {
        let symbol_name = symbol_name.to_string();
        let key = key.to_string();
        let value = value.to_string();
        self.mutate_root_items(|items| {
            if let Some(idx) = find_symbol_index(items, &symbol_name) {
                if let Some(Node::List {
                    items: symbol_items,
                    ..
                }) = items.get_mut(idx)
                {
                    upsert_property_preserve_tail(symbol_items, &key, &value, 2)
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    pub fn remove_symbol_property(&mut self, symbol_name: &str, key: &str) -> &mut Self {
        let symbol_name = symbol_name.to_string();
        let key = key.to_string();
        self.mutate_root_items(|items| {
            if let Some(idx) = find_symbol_index(items, &symbol_name) {
                if let Some(Node::List {
                    items: symbol_items,
                    ..
                }) = items.get_mut(idx)
                {
                    remove_property(symbol_items, &key, 2)
                } else {
                    false
                }
            } else {
                false
            }
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

pub struct SymbolLibFile;

impl SymbolLibFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<SymbolLibDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["kicad_symbol_lib"])?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_version_diagnostics(ast.version);
        Ok(SymbolLibDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

fn parse_ast(cst: &CstDocument) -> SymbolLibAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut symbols = Vec::new();
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("symbol") => symbols.push(parse_symbol(item)),
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    let symbol_count = symbols.len();
    let total_property_count = symbols.iter().map(|s| s.property_count).sum();
    let total_pin_count = symbols.iter().map(|s| s.pin_count).sum();

    SymbolLibAst {
        version,
        generator,
        generator_version,
        symbol_count,
        total_property_count,
        total_pin_count,
        symbols,
        unknown_nodes,
    }
}

fn parse_symbol(node: &Node) -> Symbol {
    let Node::List { items, .. } = node else {
        return Symbol {
            name: None,
            property_count: 0,
            pin_count: 0,
            unit_count: 0,
            has_embedded_fonts: false,
            properties: Vec::new(),
            pins: Vec::new(),
            units: Vec::new(),
            graphics: Vec::new(),
            graphic_count: 0,
            extends: None,
            in_bom: None,
            on_board: None,
            pin_names_hide: false,
            pin_names_offset: None,
            pin_numbers_hide: false,
            power: false,
            exclude_from_sim: None,
        };
    };

    let name = items.get(1).and_then(atom_as_string);
    let mut properties = Vec::new();
    let mut pins = Vec::new();
    let mut units = Vec::new();
    let mut graphics = Vec::new();
    let mut extends = None;
    let mut in_bom = None;
    let mut on_board = None;
    let mut pin_names_hide = false;
    let mut pin_names_offset = None;
    let mut pin_numbers_hide = false;
    let mut power = false;
    let mut exclude_from_sim = None;

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("property") => {
                if let Node::List {
                    items: property_items,
                    ..
                } = child
                {
                    let key = property_items
                        .get(1)
                        .and_then(atom_as_string)
                        .unwrap_or_default();
                    let value = property_items
                        .get(2)
                        .and_then(atom_as_string)
                        .unwrap_or_default();
                    let id = property_items.iter().find_map(|item| {
                        if head_of(item) == Some("id") {
                            second_atom_i32(item)
                        } else {
                            None
                        }
                    });
                    properties.push(SymProperty { key, value, id });
                }
            }
            Some("pin") => pins.push(parse_sym_pin(child)),
            Some("symbol") => units.push(parse_sym_unit(child)),
            Some("polyline") | Some("rectangle") | Some("circle") | Some("arc") | Some("text") => {
                if let Some(token) = head_of(child) {
                    graphics.push(parse_sym_graphic(child, token));
                }
            }
            Some("extends") => extends = second_atom_string(child),
            Some("in_bom") => in_bom = second_atom_bool(child),
            Some("on_board") => on_board = second_atom_bool(child),
            Some("pin_names") => {
                if let Node::List {
                    items: pin_name_items,
                    ..
                } = child
                {
                    pin_names_hide = pin_name_items.iter().any(|item| match item {
                        Node::Atom {
                            atom: Atom::Symbol(s),
                            ..
                        } => s == "hide",
                        _ => false,
                    });
                    pin_names_offset = pin_name_items.iter().find_map(|item| {
                        if head_of(item) == Some("offset") {
                            second_atom_f64(item)
                        } else {
                            None
                        }
                    });
                }
            }
            Some("pin_numbers") => {
                if let Node::List {
                    items: pin_number_items,
                    ..
                } = child
                {
                    pin_numbers_hide = pin_number_items.iter().any(|item| match item {
                        Node::Atom {
                            atom: Atom::Symbol(s),
                            ..
                        } => s == "hide",
                        _ => false,
                    });
                }
            }
            Some("power") => power = true,
            Some("exclude_from_sim") => exclude_from_sim = second_atom_bool(child),
            _ => {}
        }
    }

    let property_count = properties.len();
    let unit_count = units.len();
    let pin_count = count_head_recursive(node, "pin");
    let graphic_count = graphics.len();
    let has_embedded_fonts = items
        .iter()
        .skip(2)
        .any(|child| head_of(child) == Some("embedded_fonts"));

    Symbol {
        name,
        property_count,
        pin_count,
        unit_count,
        has_embedded_fonts,
        properties,
        pins,
        units,
        graphics,
        graphic_count,
        extends,
        in_bom,
        on_board,
        pin_names_hide,
        pin_names_offset,
        pin_numbers_hide,
        power,
        exclude_from_sim,
    }
}

fn parse_sym_pin(node: &Node) -> SymPin {
    let Node::List { items, .. } = node else {
        return SymPin {
            electrical_type: None,
            graphic_style: None,
            at: None,
            angle: None,
            length: None,
            name: None,
            number: None,
            hide: false,
        };
    };

    let electrical_type = items.get(1).and_then(atom_as_string);
    let graphic_style = items.get(2).and_then(atom_as_string);
    let mut at = None;
    let mut angle = None;
    let mut length = None;
    let mut name = None;
    let mut number = None;
    let mut hide = false;

    for child in items.iter().skip(3) {
        match head_of(child) {
            Some("at") => {
                let (xy, a) = parse_sym_xy_and_angle(child);
                at = xy;
                angle = a;
            }
            Some("length") => length = second_atom_f64(child),
            Some("name") => name = second_atom_string(child),
            Some("number") => number = second_atom_string(child),
            Some("hide") => hide = true,
            _ => {
                if matches!(
                    child,
                    Node::Atom {
                        atom: Atom::Symbol(v),
                        ..
                    } if v == "hide"
                ) {
                    hide = true;
                }
            }
        }
    }

    SymPin {
        electrical_type,
        graphic_style,
        at,
        angle,
        length,
        name,
        number,
        hide,
    }
}

fn parse_sym_graphic(node: &Node, token: &str) -> SymGraphic {
    let mut start = None;
    let mut end = None;
    let mut center = None;
    let mut radius = None;
    let mut stroke_width = None;
    let mut stroke_type = None;
    let mut fill_type = None;

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("start") => start = parse_sym_xy(child),
                Some("end") => end = parse_sym_xy(child),
                Some("center") => center = parse_sym_xy(child),
                Some("mid") => center = parse_sym_xy(child),
                Some("radius") => radius = second_atom_f64(child),
                Some("stroke") => {
                    if let Node::List {
                        items: stroke_items,
                        ..
                    } = child
                    {
                        for stroke_child in stroke_items.iter().skip(1) {
                            match head_of(stroke_child) {
                                Some("width") => stroke_width = second_atom_f64(stroke_child),
                                Some("type") => stroke_type = second_atom_string(stroke_child),
                                _ => {}
                            }
                        }
                    }
                }
                Some("fill") => {
                    if let Node::List {
                        items: fill_items, ..
                    } = child
                    {
                        for fill_child in fill_items.iter().skip(1) {
                            if head_of(fill_child) == Some("type") {
                                fill_type = second_atom_string(fill_child);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    SymGraphic {
        token: token.to_string(),
        start,
        end,
        center,
        radius,
        stroke_width,
        stroke_type,
        fill_type,
    }
}

fn parse_sym_unit(node: &Node) -> SymUnit {
    let Node::List { items, .. } = node else {
        return SymUnit {
            name: None,
            pin_count: 0,
            pins: Vec::new(),
            graphic_count: 0,
            graphics: Vec::new(),
        };
    };

    let name = items.get(1).and_then(atom_as_string);
    let mut pins = Vec::new();
    let mut graphics = Vec::new();

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("pin") => pins.push(parse_sym_pin(child)),
            Some("polyline") | Some("rectangle") | Some("circle") | Some("arc") | Some("text") => {
                if let Some(token) = head_of(child) {
                    graphics.push(parse_sym_graphic(child, token));
                }
            }
            _ => {}
        }
    }

    let pin_count = pins.len();
    let graphic_count = graphics.len();

    SymUnit {
        name,
        pin_count,
        pins,
        graphic_count,
        graphics,
    }
}

fn parse_sym_xy(node: &Node) -> Option<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_f64)?;
    let y = items.get(2).and_then(atom_as_f64)?;
    Some([x, y])
}

fn parse_sym_xy_and_angle(node: &Node) -> (Option<[f64; 2]>, Option<f64>) {
    let Node::List { items, .. } = node else {
        return (None, None);
    };

    let xy = match (
        items.get(1).and_then(atom_as_f64),
        items.get(2).and_then(atom_as_f64),
    ) {
        (Some(x), Some(y)) => Some([x, y]),
        _ => None,
    };
    let angle = items.get(3).and_then(atom_as_f64);

    (xy, angle)
}
fn count_head_recursive(node: &Node, target: &str) -> usize {
    match node {
        Node::List { items, .. } => {
            let mut count = 0;
            if head_of(node) == Some(target) {
                count += 1;
            }
            for child in items.iter().skip(1) {
                count += count_head_recursive(child, target);
            }
            count
        }
        Node::Atom { .. } => 0,
    }
}

fn find_symbol_index(items: &[Node], name: &str) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, node)| {
            if head_of(node) != Some("symbol") {
                return false;
            }
            match node {
                Node::List {
                    items: symbol_items,
                    ..
                } => symbol_items.get(1).and_then(atom_as_string).as_deref() == Some(name),
                _ => false,
            }
        })
        .map(|(idx, _)| idx)
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_sym"))
    }

    #[test]
    fn read_symbol_lib_and_preserve_lossless() {
        let path = tmp_file("sym_read_ok");
        let src = "(kicad_symbol_lib (version 20260101) (generator kicad_symbol_editor) (symbol \"R\" (property \"Reference\" \"R\") (symbol \"R_0_0\" (pin passive line (at 0 0 0) (length 1)))))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SymbolLibFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20260101));
        assert_eq!(doc.ast().generator.as_deref(), Some("kicad_symbol_editor"));
        assert_eq!(doc.ast().symbol_count, 1);
        assert_eq!(doc.ast().total_property_count, 1);
        assert_eq!(doc.ast().total_pin_count, 1);
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn captures_unknown_nodes_roundtrip() {
        let path = tmp_file("sym_unknown");
        let src = "(kicad_symbol_lib (version 20260101) (generator kicad_symbol_editor) (future_block 1 2) (symbol \"R\"))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SymbolLibFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);

        let out = tmp_file("sym_unknown_out");
        doc.write(&out).expect("write");
        let got = fs::read_to_string(&out).expect("read out");
        assert_eq!(got, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn edit_roundtrip_updates_symbol_metadata() {
        let path = tmp_file("sym_edit");
        let src = "(kicad_symbol_lib (version 20241209) (generator kicad_symbol_editor)\n  (symbol \"Old\" (property \"Reference\" \"U\") (property \"Value\" \"Old\") (symbol \"Old_0_0\" (pin input line (at 0 0 0) (length 2))))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = SymbolLibFile::read(&path).expect("read");
        doc.set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .rename_symbol("Old", "New")
            .upsert_symbol_property("New", "Value", "NewValue")
            .remove_symbol_property("New", "Reference");

        let out = tmp_file("sym_edit_out");
        doc.write(&out).expect("write");
        let reread = SymbolLibFile::read(&out).expect("reread");

        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(
            reread.ast().symbols.first().and_then(|s| s.name.clone()),
            Some("New".to_string())
        );
        assert_eq!(
            reread.ast().symbols.first().map(|s| s.property_count),
            Some(1)
        );
        assert_eq!(reread.ast().total_pin_count, 1);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument, Node};

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

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolPropertyEntry {
    pub key: Option<String>,
    pub value: Option<String>,
    pub id: Option<i32>,
    pub at: Option<[f64; 3]>,
    pub node: Node,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolPinEntry {
    pub electrical: Option<String>,
    pub style: Option<String>,
    pub at: Option<[f64; 3]>,
    pub length: Option<f64>,
    pub name: Option<String>,
    pub number: Option<String>,
    pub hide: Option<bool>,
    pub node: Node,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolUnitEntry {
    pub name: Option<String>,
    pub pin_count: usize,
    pub graphic_count: usize,
    pub child_heads: Vec<String>,
    pub node: Node,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolDefinition {
    pub name: Option<String>,
    pub is_power: bool,
    pub exclude_from_sim: Option<bool>,
    pub in_bom: Option<bool>,
    pub on_board: Option<bool>,
    pub properties: Vec<SymbolPropertyEntry>,
    pub pins: Vec<SymbolPinEntry>,
    pub units: Vec<SymbolUnitEntry>,
    pub has_embedded_fonts: bool,
    pub child_heads: Vec<String>,
    pub node: Node,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolSummary {
    pub name: Option<String>,
    pub property_count: usize,
    pub pin_count: usize,
    pub unit_count: usize,
    pub has_embedded_fonts: bool,
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
    pub symbols: Vec<SymbolSummary>,
    pub symbol_definitions: Vec<SymbolDefinition>,
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
    let mut symbol_definitions = Vec::new();
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("symbol") => {
                    let detail = parse_symbol_definition(item);
                    symbols.push(SymbolSummary {
                        name: detail.name.clone(),
                        property_count: detail.properties.len(),
                        pin_count: detail.pins.len(),
                        unit_count: detail.units.len(),
                        has_embedded_fonts: detail.has_embedded_fonts,
                    });
                    symbol_definitions.push(detail);
                }
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
        symbol_definitions,
        unknown_nodes,
    }
}

fn parse_symbol_definition(node: &Node) -> SymbolDefinition {
    let Node::List { items, .. } = node else {
        return SymbolDefinition {
            name: None,
            is_power: false,
            exclude_from_sim: None,
            in_bom: None,
            on_board: None,
            properties: Vec::new(),
            pins: Vec::new(),
            units: Vec::new(),
            has_embedded_fonts: false,
            child_heads: Vec::new(),
            node: node.clone(),
        };
    };

    let name = items.get(1).and_then(atom_as_string);
    let mut is_power = false;
    let mut exclude_from_sim = None;
    let mut in_bom = None;
    let mut on_board = None;
    let mut properties = Vec::new();
    let mut units = Vec::new();
    let mut child_heads = Vec::new();
    let mut has_embedded_fonts = false;

    for child in items.iter().skip(2) {
        if let Some(head) = head_of(child) {
            child_heads.push(head.to_string());
        }
        match head_of(child) {
            Some("power") => is_power = true,
            Some("exclude_from_sim") => exclude_from_sim = second_atom_bool(child),
            Some("in_bom") => in_bom = second_atom_bool(child),
            Some("on_board") => on_board = second_atom_bool(child),
            Some("embedded_fonts") => has_embedded_fonts = true,
            Some("property") => properties.push(parse_symbol_property_entry(child)),
            Some("symbol") => units.push(parse_symbol_unit_entry(child)),
            _ => {}
        }
    }

    let mut pins = Vec::new();
    collect_pins_recursive(node, &mut pins);

    SymbolDefinition {
        name,
        is_power,
        exclude_from_sim,
        in_bom,
        on_board,
        properties,
        pins,
        units,
        has_embedded_fonts,
        child_heads,
        node: node.clone(),
    }
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

fn parse_symbol_property_entry(node: &Node) -> SymbolPropertyEntry {
    let Node::List { items, .. } = node else {
        return SymbolPropertyEntry {
            key: None,
            value: None,
            id: None,
            at: None,
            node: node.clone(),
        };
    };

    let at = items
        .iter()
        .skip(3)
        .find(|child| head_of(child) == Some("at"))
        .and_then(parse_at3);

    SymbolPropertyEntry {
        key: items.get(1).and_then(atom_as_string),
        value: items.get(2).and_then(atom_as_string),
        id: items
            .get(3)
            .and_then(atom_as_string)
            .and_then(|v| v.parse::<i32>().ok()),
        at,
        node: node.clone(),
    }
}

fn parse_symbol_pin_entry(node: &Node) -> SymbolPinEntry {
    let Node::List { items, .. } = node else {
        return SymbolPinEntry {
            electrical: None,
            style: None,
            at: None,
            length: None,
            name: None,
            number: None,
            hide: None,
            node: node.clone(),
        };
    };

    let mut at = None;
    let mut length = None;
    let mut name = None;
    let mut number = None;
    let mut hide = None;

    for child in items.iter().skip(1) {
        match head_of(child) {
            Some("at") => at = parse_at3(child),
            Some("length") => length = second_atom_f64(child),
            Some("name") => name = second_atom_string(child),
            Some("number") => number = second_atom_string(child),
            Some("hide") => hide = second_atom_bool(child),
            _ => {}
        }
    }

    SymbolPinEntry {
        electrical: items.get(1).and_then(atom_as_string),
        style: items.get(2).and_then(atom_as_string),
        at,
        length,
        name,
        number,
        hide,
        node: node.clone(),
    }
}

fn parse_symbol_unit_entry(node: &Node) -> SymbolUnitEntry {
    let Node::List { items, .. } = node else {
        return SymbolUnitEntry {
            name: None,
            pin_count: 0,
            graphic_count: 0,
            child_heads: Vec::new(),
            node: node.clone(),
        };
    };

    let child_heads = items
        .iter()
        .skip(2)
        .filter_map(head_of)
        .map(|h| h.to_string())
        .collect::<Vec<_>>();
    let pin_count = count_head_recursive(node, "pin");
    let graphic_count = count_heads_recursive_set(
        node,
        &["polyline", "rectangle", "circle", "arc", "bezier", "text"],
    );

    SymbolUnitEntry {
        name: items.get(1).and_then(atom_as_string),
        pin_count,
        graphic_count,
        child_heads,
        node: node.clone(),
    }
}

fn count_heads_recursive_set(node: &Node, targets: &[&str]) -> usize {
    match node {
        Node::List { items, .. } => {
            let mut count = 0usize;
            if let Some(head) = head_of(node) {
                if targets.contains(&head) {
                    count += 1;
                }
            }
            for child in items.iter().skip(1) {
                count += count_heads_recursive_set(child, targets);
            }
            count
        }
        Node::Atom { .. } => 0,
    }
}

fn collect_pins_recursive(node: &Node, out: &mut Vec<SymbolPinEntry>) {
    match node {
        Node::List { items, .. } => {
            if head_of(node) == Some("pin") {
                out.push(parse_symbol_pin_entry(node));
            }
            for child in items.iter().skip(1) {
                collect_pins_recursive(child, out);
            }
        }
        Node::Atom { .. } => {}
    }
}

fn parse_at3(node: &Node) -> Option<[f64; 3]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_f64)?;
    let y = items.get(2).and_then(atom_as_f64)?;
    let rot = items.get(3).and_then(atom_as_f64).unwrap_or(0.0);
    Some([x, y, rot])
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

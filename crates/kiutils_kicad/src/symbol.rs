use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, mutate_root_and_refresh, remove_property,
    upsert_property_preserve_tail, upsert_scalar,
};
use crate::sexpr_utils::{atom_as_string, head_of, second_atom_i32, second_atom_string};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolSummary {
    pub name: Option<String>,
    pub property_count: usize,
    pub pin_count: usize,
    pub unit_count: usize,
    pub has_embedded_fonts: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SymbolLibAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub symbol_count: usize,
    pub total_property_count: usize,
    pub total_pin_count: usize,
    pub symbols: Vec<SymbolSummary>,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct SymbolLibDocument {
    ast: SymbolLibAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
}

impl SymbolLibDocument {
    pub fn ast(&self) -> &SymbolLibAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut SymbolLibAst {
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
                Some("symbol") => symbols.push(parse_symbol_summary(item)),
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

fn parse_symbol_summary(node: &Node) -> SymbolSummary {
    let Node::List { items, .. } = node else {
        return SymbolSummary {
            name: None,
            property_count: 0,
            pin_count: 0,
            unit_count: 0,
            has_embedded_fonts: false,
        };
    };

    let name = items.get(1).and_then(atom_as_string);
    let property_count = items
        .iter()
        .skip(2)
        .filter(|child| head_of(child) == Some("property"))
        .count();
    let unit_count = items
        .iter()
        .skip(2)
        .filter(|child| head_of(child) == Some("symbol"))
        .count();
    let pin_count = count_head_recursive(node, "pin");
    let has_embedded_fonts = items
        .iter()
        .skip(2)
        .any(|child| head_of(child) == Some("embedded_fonts"));

    SymbolSummary {
        name,
        property_count,
        pin_count,
        unit_count,
        has_embedded_fonts,
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

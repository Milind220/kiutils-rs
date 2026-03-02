use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, list_node, mutate_root_and_refresh,
    upsert_scalar,
};
use crate::sexpr_utils::{atom_as_string, head_of, second_atom_i32};
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum LibTableKind {
    Footprint,
    Symbol,
}

impl LibTableKind {
    fn root_token(self) -> &'static str {
        match self {
            Self::Footprint => "fp_lib_table",
            Self::Symbol => "sym_lib_table",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LibTableLibrarySummary {
    pub name: Option<String>,
    pub library_type: Option<String>,
    pub uri: Option<String>,
    pub options: Option<String>,
    pub descr: Option<String>,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LibTableAst {
    pub kind: LibTableKind,
    pub version: Option<i32>,
    pub libraries: Vec<LibTableLibrarySummary>,
    pub library_count: usize,
    pub disabled_library_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}

pub type FpLibTableAst = LibTableAst;
pub type SymLibTableAst = LibTableAst;

#[derive(Debug, Clone)]
pub struct LibTableDocument {
    ast: LibTableAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

pub type FpLibTableDocument = LibTableDocument;
pub type SymLibTableDocument = LibTableDocument;

impl LibTableDocument {
    pub fn ast(&self) -> &LibTableAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut LibTableAst {
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

    pub fn add_library<N: Into<String>, U: Into<String>>(&mut self, name: N, uri: U) -> &mut Self {
        let node = lib_node(LibNodeInput {
            name: name.into(),
            library_type: "KiCad".to_string(),
            uri: uri.into(),
            options: "".to_string(),
            descr: "".to_string(),
            disabled: false,
        });
        self.mutate_root_items(|items| {
            items.push(node);
            true
        })
    }

    pub fn rename_library<S: Into<String>>(&mut self, from: &str, to: S) -> &mut Self {
        let from = from.to_string();
        let to = to.into();
        self.mutate_root_items(|items| {
            let Some(idx) = find_library_index(items, &from) else {
                return false;
            };
            let Some(Node::List {
                items: lib_items, ..
            }) = items.get_mut(idx)
            else {
                return false;
            };
            if let Some(name_idx) = lib_items.iter().position(|n| head_of(n) == Some("name")) {
                let Some(Node::List {
                    items: name_items, ..
                }) = lib_items.get_mut(name_idx)
                else {
                    return false;
                };
                if name_items.len() > 1 {
                    let next = atom_quoted(to);
                    if name_items[1] == next {
                        false
                    } else {
                        name_items[1] = next;
                        true
                    }
                } else {
                    false
                }
            } else {
                lib_items.insert(1, list_node2("name".to_string(), atom_quoted(to)));
                true
            }
        })
    }

    pub fn remove_library(&mut self, name: &str) -> &mut Self {
        let name = name.to_string();
        self.mutate_root_items(|items| {
            if let Some(idx) = find_library_index(items, &name) {
                items.remove(idx);
                true
            } else {
                false
            }
        })
    }

    pub fn upsert_library_uri<N: AsRef<str>, U: Into<String>>(
        &mut self,
        name: N,
        uri: U,
    ) -> &mut Self {
        let name = name.as_ref().to_string();
        let uri = uri.into();
        self.mutate_root_items(|items| {
            let Some(idx) = find_library_index(items, &name) else {
                items.push(lib_node(LibNodeInput {
                    name,
                    library_type: "KiCad".to_string(),
                    uri,
                    options: "".to_string(),
                    descr: "".to_string(),
                    disabled: false,
                }));
                return true;
            };

            let Some(Node::List {
                items: lib_items, ..
            }) = items.get_mut(idx)
            else {
                return false;
            };
            upsert_scalar(lib_items, "uri", atom_quoted(uri), 1)
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
        let kind = self.ast.kind;
        mutate_root_and_refresh(
            &mut self.cst,
            &mut self.ast,
            &mut self.diagnostics,
            mutate,
            |cst| parse_ast(cst, kind),
            |_cst, _ast| Vec::new(),
        );
        self.ast_dirty = false;
        self
    }
}

pub struct FpLibTableFile;
pub struct SymLibTableFile;

impl FpLibTableFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<FpLibTableDocument, Error> {
        read_kind(path, LibTableKind::Footprint)
    }
}

impl SymLibTableFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<SymLibTableDocument, Error> {
        read_kind(path, LibTableKind::Symbol)
    }
}

fn read_kind<P: AsRef<Path>>(path: P, kind: LibTableKind) -> Result<LibTableDocument, Error> {
    let raw = fs::read_to_string(path)?;
    let cst = parse_one(&raw)?;
    ensure_root_head_any(&cst, &[kind.root_token()])?;
    let ast = parse_ast(&cst, kind);
    Ok(LibTableDocument {
        ast,
        cst,
        diagnostics: Vec::new(),
        ast_dirty: false,
    })
}

fn parse_ast(cst: &CstDocument, kind: LibTableKind) -> LibTableAst {
    let mut version = None;
    let mut libraries = Vec::new();
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("lib") => libraries.push(parse_library_summary(item)),
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    let library_count = libraries.len();
    let disabled_library_count = libraries.iter().filter(|l| l.disabled).count();

    LibTableAst {
        kind,
        version,
        libraries,
        library_count,
        disabled_library_count,
        unknown_nodes,
    }
}

fn parse_library_summary(node: &Node) -> LibTableLibrarySummary {
    let mut name = None;
    let mut library_type = None;
    let mut uri = None;
    let mut options = None;
    let mut descr = None;
    let mut disabled = false;

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("name") => name = second_atom_string(child),
                Some("type") => library_type = second_atom_string(child),
                Some("uri") => uri = second_atom_string(child),
                Some("options") => options = second_atom_string(child),
                Some("descr") => descr = second_atom_string(child),
                Some("disabled") => disabled = true,
                _ => {}
            }
        }
    }

    LibTableLibrarySummary {
        name,
        library_type,
        uri,
        options,
        descr,
        disabled,
    }
}

fn second_atom_string(node: &Node) -> Option<String> {
    match node {
        Node::List { items, .. } => items.get(1).and_then(atom_as_string),
        _ => None,
    }
}

fn find_library_index(items: &[Node], name: &str) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, node)| {
            if head_of(node) != Some("lib") {
                return false;
            }
            match node {
                Node::List {
                    items: lib_items, ..
                } => {
                    lib_items
                        .iter()
                        .find(|n| head_of(n) == Some("name"))
                        .and_then(second_atom_string)
                        .as_deref()
                        == Some(name)
                }
                _ => false,
            }
        })
        .map(|(idx, _)| idx)
}

struct LibNodeInput {
    name: String,
    library_type: String,
    uri: String,
    options: String,
    descr: String,
    disabled: bool,
}

fn list_node2(head: String, value: Node) -> Node {
    list_node(vec![atom_symbol(head), value])
}

fn lib_node(input: LibNodeInput) -> Node {
    let mut items = vec![atom_symbol("lib".to_string())];
    items.push(list_node2("name".to_string(), atom_quoted(input.name)));
    items.push(list_node2(
        "type".to_string(),
        atom_quoted(input.library_type),
    ));
    items.push(list_node2("uri".to_string(), atom_quoted(input.uri)));
    items.push(list_node2(
        "options".to_string(),
        atom_quoted(input.options),
    ));
    items.push(list_node2("descr".to_string(), atom_quoted(input.descr)));
    if input.disabled {
        items.push(list_node(vec![atom_symbol("disabled".to_string())]));
    }
    list_node(items)
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
        std::env::temp_dir().join(format!("{name}_{nanos}.table"))
    }

    #[test]
    fn read_fp_lib_table() {
        let path = tmp_file("fplib_ok");
        let src = "(fp_lib_table\n  (version 7)\n  (lib (name \"A\") (type \"KiCad\") (uri \"x\") (options \"\") (descr \"\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FpLibTableFile::read(&path).expect("read");
        assert_eq!(doc.ast().kind, LibTableKind::Footprint);
        assert_eq!(doc.ast().version, Some(7));
        assert_eq!(doc.ast().library_count, 1);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_sym_lib_table() {
        let path = tmp_file("symlib_ok");
        let src = "(sym_lib_table\n  (version 7)\n  (lib (name \"S\") (type \"KiCad\") (uri \"y\") (options \"\") (descr \"\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SymLibTableFile::read(&path).expect("read");
        assert_eq!(doc.ast().kind, LibTableKind::Symbol);
        assert_eq!(doc.ast().version, Some(7));
        assert_eq!(doc.ast().library_count, 1);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_fp_lib_table_captures_unknown() {
        let path = tmp_file("fplib_unknown");
        let src = "(fp_lib_table\n  (lib (name \"A\") (type \"KiCad\") (uri \"x\") (options \"\") (descr \"\"))\n  (unknown_table_item 1)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FpLibTableFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);
        assert_eq!(
            doc.ast().unknown_nodes[0].head.as_deref(),
            Some("unknown_table_item")
        );

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_renames_and_adds_library() {
        let path = tmp_file("fplib_edit");
        let src = "(fp_lib_table (version 7) (lib (name \"A\") (type \"KiCad\") (uri \"x\") (options \"\") (descr \"\")))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FpLibTableFile::read(&path).expect("read");
        doc.rename_library("A", "B")
            .add_library("C", "${KIPRJMOD}/C");
        let out = tmp_file("fplib_edit_out");
        doc.write(&out).expect("write");
        let reread = FpLibTableFile::read(&out).expect("reread");
        assert_eq!(reread.ast().library_count, 2);
        assert_eq!(
            reread.ast().libraries.first().and_then(|l| l.name.clone()),
            Some("B".to_string())
        );
        assert_eq!(
            reread.ast().libraries.get(1).and_then(|l| l.name.clone()),
            Some("C".to_string())
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn upsert_library_uri_replaces_existing_uri() {
        let path = tmp_file("fplib_upsert_existing");
        let src = "(fp_lib_table (version 7) (lib (name \"A\") (type \"Legacy\") (uri \"x\") (options \"opt=1\") (descr \"legacy\") (disabled)))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FpLibTableFile::read(&path).expect("read");
        doc.upsert_library_uri("A", "${KIPRJMOD}/A.pretty");
        assert_eq!(doc.ast().library_count, 1);
        assert_eq!(
            doc.ast().libraries[0].library_type.as_deref(),
            Some("Legacy")
        );
        assert_eq!(
            doc.ast().libraries[0].uri.as_deref(),
            Some("${KIPRJMOD}/A.pretty")
        );
        assert_eq!(doc.ast().libraries[0].options.as_deref(), Some("opt=1"));
        assert_eq!(doc.ast().libraries[0].descr.as_deref(), Some("legacy"));
        assert!(doc.ast().libraries[0].disabled);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn upsert_library_uri_adds_when_missing() {
        let path = tmp_file("fplib_upsert_missing");
        let src = "(fp_lib_table (version 7))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = FpLibTableFile::read(&path).expect("read");
        doc.upsert_library_uri("A", "${KIPRJMOD}/A.pretty");
        assert_eq!(doc.ast().library_count, 1);
        assert_eq!(doc.ast().libraries[0].name.as_deref(), Some("A"));
        assert_eq!(
            doc.ast().libraries[0].uri.as_deref(),
            Some("${KIPRJMOD}/A.pretty")
        );

        let _ = fs::remove_file(path);
    }
}

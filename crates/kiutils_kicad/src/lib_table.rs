use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FpLibTableAst {
    pub library_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct FpLibTableDocument {
    ast: FpLibTableAst,
    cst: CstDocument,
}

impl FpLibTableDocument {
    pub fn ast(&self) -> &FpLibTableAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut FpLibTableAst {
        &mut self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
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

pub struct FpLibTableFile;

impl FpLibTableFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<FpLibTableDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;

        let root_items = cst
            .nodes
            .first()
            .and_then(|n| match n {
                Node::List { items, .. } => Some(items),
                _ => None,
            })
            .ok_or_else(|| Error::Validation("invalid fp-lib-table root".to_string()))?;

        let head = root_items
            .first()
            .and_then(|n| match n {
                Node::Atom {
                    atom: Atom::Symbol(s),
                    ..
                } => Some(s.as_str()),
                _ => None,
            })
            .ok_or_else(|| Error::Validation("missing fp-lib-table head".to_string()))?;

        if head != "fp_lib_table" {
            return Err(Error::Validation(format!(
                "expected root token `fp_lib_table`, got `{head}`"
            )));
        }

        let library_count = root_items
            .iter()
            .filter(|n| match n {
                Node::List { items, .. } => matches!(
                    items.first(),
                    Some(Node::Atom {
                        atom: Atom::Symbol(s),
                        ..
                    }) if s == "lib"
                ),
                _ => false,
            })
            .count();

        let unknown_nodes = root_items
            .iter()
            .skip(1)
            .filter_map(|n| match n {
                Node::List { items, .. } => match items.first() {
                    Some(Node::Atom {
                        atom: Atom::Symbol(s),
                        ..
                    }) if s == "lib" => None,
                    _ => UnknownNode::from_node(n),
                },
                _ => UnknownNode::from_node(n),
            })
            .collect();

        Ok(FpLibTableDocument {
            ast: FpLibTableAst {
                library_count,
                unknown_nodes,
            },
            cst,
        })
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
        std::env::temp_dir().join(format!("{name}_{nanos}.fp-lib-table"))
    }

    #[test]
    fn read_fp_lib_table() {
        let path = tmp_file("fplib_ok");
        let src = "(fp_lib_table\n  (lib (name \"A\") (type \"KiCad\") (uri \"x\") (options \"\") (descr \"\"))\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FpLibTableFile::read(&path).expect("read");
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
}

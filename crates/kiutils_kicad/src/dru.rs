use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_rootless, Atom, CstDocument, Node};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignRulesAst {
    pub version: Option<i32>,
    pub rule_count: usize,
}

#[derive(Debug, Clone)]
pub struct DesignRulesDocument {
    ast: DesignRulesAst,
    cst: CstDocument,
}

impl DesignRulesDocument {
    pub fn ast(&self) -> &DesignRulesAst {
        &self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        fs::write(path, self.cst.to_lossless_string())?;
        Ok(())
    }
}

pub struct DesignRulesFile;

impl DesignRulesFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<DesignRulesDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_rootless(&raw)?;

        let mut version = None;
        let mut rule_count = 0usize;

        for node in &cst.nodes {
            let Node::List { items, .. } = node else {
                continue;
            };
            let Some(Node::Atom { atom: Atom::Symbol(head), .. }) = items.first() else {
                continue;
            };

            if head == "version" {
                if let Some(Node::Atom {
                    atom: Atom::Symbol(v),
                    ..
                }) = items.get(1)
                {
                    version = v.parse::<i32>().ok();
                }
            }

            if head == "rule" {
                rule_count += 1;
            }
        }

        Ok(DesignRulesDocument {
            ast: DesignRulesAst {
                version,
                rule_count,
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_dru"))
    }

    #[test]
    fn read_rootless_dru() {
        let path = tmp_file("dru_ok");
        let src = "(version 1)\n(rule \"x\" (constraint clearance (min \"0.1mm\")) (condition \"A\"))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = DesignRulesFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(1));
        assert_eq!(doc.ast().rule_count, 1);
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }
}

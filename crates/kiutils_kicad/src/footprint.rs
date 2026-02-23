use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::version::VersionPolicy;
use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootprintAst {
    pub version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct FootprintDocument {
    ast: FootprintAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
}

impl FootprintDocument {
    pub fn ast(&self) -> &FootprintAst {
        &self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        fs::write(path, self.cst.to_lossless_string())?;
        Ok(())
    }
}

pub struct FootprintFile;

impl FootprintFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<FootprintDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_head(&cst, "footprint")?;
        let ast = FootprintAst {
            version: find_version(&cst),
        };
        let diagnostics = validate_version(ast.version)?;
        Ok(FootprintDocument {
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

fn find_version(cst: &CstDocument) -> Option<i32> {
    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items {
            if let Node::List { items: inner, .. } = item {
                if let [
                    Node::Atom {
                        atom: Atom::Symbol(head),
                        ..
                    },
                    Node::Atom {
                        atom: Atom::Symbol(v),
                        ..
                    },
                    ..,
                ] = inner.as_slice()
                {
                    if head == "version" {
                        return v.parse::<i32>().ok();
                    }
                }
            }
        }
    }
    None
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_mod"))
    }

    #[test]
    fn read_footprint_and_preserve_lossless() {
        let path = tmp_file("footprint_read_ok");
        let src = "(footprint \"R_0603\" (version 20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20260101));
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
}

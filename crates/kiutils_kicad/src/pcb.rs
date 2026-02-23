use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcbAst {
    pub version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct PcbDocument {
    ast: PcbAst,
    cst: CstDocument,
}

impl PcbDocument {
    pub fn ast(&self) -> &PcbAst {
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

pub struct PcbFile;

impl PcbFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<PcbDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        let ast = parse_version(&cst);
        Ok(PcbDocument { ast, cst })
    }
}

fn parse_version(cst: &CstDocument) -> PcbAst {
    let mut version = None;

    if let Some(kiutils_sexpr::Node::List { items, .. }) = cst.nodes.first() {
        // Expect (kicad_pcb ... (version N) ...)
        for item in items {
            if let kiutils_sexpr::Node::List { items: inner, .. } = item {
                if let [
                    kiutils_sexpr::Node::Atom {
                        atom: kiutils_sexpr::Atom::Symbol(head),
                        ..
                    },
                    kiutils_sexpr::Node::Atom {
                        atom: kiutils_sexpr::Atom::Symbol(v),
                        ..
                    },
                    ..,
                ] = inner.as_slice()
                {
                    if head == "version" {
                        version = v.parse::<i32>().ok();
                        break;
                    }
                }
            }
        }
    }

    PcbAst { version }
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
}

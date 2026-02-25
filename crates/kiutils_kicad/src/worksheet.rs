use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, list_node, mutate_root_and_refresh, root_head,
    upsert_scalar, upsert_section_child_node, upsert_section_child_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, head_of, second_atom_f64, second_atom_i32, second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorksheetSetupSummary {
    pub text_size: Option<[f64; 2]>,
    pub line_width: Option<f64>,
    pub text_line_width: Option<f64>,
    pub left_margin: Option<f64>,
    pub right_margin: Option<f64>,
    pub top_margin: Option<f64>,
    pub bottom_margin: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WorksheetAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub has_setup: bool,
    pub setup: Option<WorksheetSetupSummary>,
    pub line_count: usize,
    pub rect_count: usize,
    pub tbtext_count: usize,
    pub polygon_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct WorksheetDocument {
    ast: WorksheetAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
}

impl WorksheetDocument {
    pub fn ast(&self) -> &WorksheetAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut WorksheetAst {
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

    pub fn set_setup_line_width(&mut self, line_width: f64) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(
                items,
                "setup",
                1,
                "linewidth",
                atom_symbol(line_width.to_string()),
            )
        })
    }

    pub fn set_setup_text_size(&mut self, width: f64, height: f64) -> &mut Self {
        let node = list_node(vec![
            atom_symbol("textsize".to_string()),
            atom_symbol(width.to_string()),
            atom_symbol(height.to_string()),
        ]);
        self.mutate_root_items(|items| {
            upsert_section_child_node(items, "setup", 1, "textsize", node)
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
            |cst, ast| collect_diagnostics(cst, ast.version),
        );
        self
    }
}

pub struct WorksheetFile;

impl WorksheetFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<WorksheetDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["kicad_wks", "page_layout"])?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_diagnostics(&cst, ast.version);
        Ok(WorksheetDocument {
            ast,
            cst,
            diagnostics,
        })
    }
}

fn collect_diagnostics(cst: &CstDocument, version: Option<i32>) -> Vec<Diagnostic> {
    let mut diagnostics = collect_version_diagnostics(version);
    if root_head(cst) == Some("page_layout") {
        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            code: "legacy_root",
            message: "legacy root token `page_layout` detected; parsing in compatibility mode"
                .to_string(),
            span: None,
            hint: Some("save from newer KiCad to normalize root token to `kicad_wks`".to_string()),
        });
    }
    diagnostics
}

fn parse_ast(cst: &CstDocument) -> WorksheetAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut has_setup = false;
    let mut setup = None;
    let mut line_count = 0usize;
    let mut rect_count = 0usize;
    let mut tbtext_count = 0usize;
    let mut polygon_count = 0usize;
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("setup") => {
                    has_setup = true;
                    setup = Some(parse_setup_summary(item));
                }
                Some("line") => line_count += 1,
                Some("rect") => rect_count += 1,
                Some("tbtext") => tbtext_count += 1,
                Some("polygon") => polygon_count += 1,
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    WorksheetAst {
        version,
        generator,
        generator_version,
        has_setup,
        setup,
        line_count,
        rect_count,
        tbtext_count,
        polygon_count,
        unknown_nodes,
    }
}

fn parse_setup_summary(node: &Node) -> WorksheetSetupSummary {
    let mut text_size = None;
    let mut line_width = None;
    let mut text_line_width = None;
    let mut left_margin = None;
    let mut right_margin = None;
    let mut top_margin = None;
    let mut bottom_margin = None;

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("textsize") => text_size = parse_pair(child),
                Some("linewidth") => line_width = second_atom_f64(child),
                Some("textlinewidth") => text_line_width = second_atom_f64(child),
                Some("left_margin") => left_margin = second_atom_f64(child),
                Some("right_margin") => right_margin = second_atom_f64(child),
                Some("top_margin") => top_margin = second_atom_f64(child),
                Some("bottom_margin") => bottom_margin = second_atom_f64(child),
                _ => {}
            }
        }
    }

    WorksheetSetupSummary {
        text_size,
        line_width,
        text_line_width,
        left_margin,
        right_margin,
        top_margin,
        bottom_margin,
    }
}

fn parse_pair(node: &Node) -> Option<[f64; 2]> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let x = items.get(1).and_then(atom_as_f64)?;
    let y = items.get(2).and_then(atom_as_f64)?;
    Some([x, y])
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_wks"))
    }

    #[test]
    fn read_kicad_wks_and_preserve_lossless() {
        let path = tmp_file("wks_read_ok");
        let src = "(kicad_wks (version 20260101) (generator \"pl_editor\") (generator_version \"9.0\") (setup (textsize 1.5 1.5) (linewidth 0.15) (left_margin 10)) (line (name \"l\")) (rect (name \"r\")) (tbtext \"t\") (polygon (name \"p\")))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = WorksheetFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20260101));
        assert_eq!(doc.ast().generator.as_deref(), Some("pl_editor"));
        assert_eq!(doc.ast().line_count, 1);
        assert_eq!(doc.ast().rect_count, 1);
        assert_eq!(doc.ast().tbtext_count, 1);
        assert_eq!(doc.ast().polygon_count, 1);
        assert!(doc.diagnostics().is_empty());
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn accepts_legacy_page_layout_root() {
        let path = tmp_file("wks_legacy");
        let src = "(page_layout (setup (linewidth 0.2)) (line (name \"l\")))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = WorksheetFile::read(&path).expect("read");
        assert_eq!(doc.ast().line_count, 1);
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_root");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_updates_core_fields() {
        let path = tmp_file("wks_edit");
        let src = "(kicad_wks (version 20250101) (generator \"pl_editor\") (setup (linewidth 0.15)) (line (name \"l\")) (future_wks 1))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = WorksheetFile::read(&path).expect("read");
        doc.set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .set_setup_line_width(0.2)
            .set_setup_text_size(1.7, 1.8);

        let out = tmp_file("wks_edit_out");
        doc.write(&out).expect("write");
        let reread = WorksheetFile::read(&out).expect("reread");
        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(
            reread.ast().setup.as_ref().and_then(|s| s.line_width),
            Some(0.2)
        );
        assert_eq!(
            reread.ast().setup.as_ref().and_then(|s| s.text_size),
            Some([1.7, 1.8])
        );
        assert_eq!(reread.ast().unknown_nodes.len(), 1);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

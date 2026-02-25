use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, Atom, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::sexpr_utils::{
    atom_as_string, head_of, list_child_head_count, second_atom_i32, second_atom_string,
};
use crate::version::VersionPolicy;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FootprintAst {
    pub lib_id: Option<String>,
    pub version: Option<i32>,
    pub tedit: Option<String>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub layer: Option<String>,
    pub descr: Option<String>,
    pub tags: Option<String>,
    pub property_count: usize,
    pub attr_present: bool,
    pub locked_present: bool,
    pub private_layers_present: bool,
    pub net_tie_pad_groups_present: bool,
    pub embedded_fonts_present: bool,
    pub has_embedded_files: bool,
    pub embedded_file_count: usize,
    pub clearance: Option<String>,
    pub solder_mask_margin: Option<String>,
    pub solder_paste_margin: Option<String>,
    pub solder_paste_margin_ratio: Option<String>,
    pub duplicate_pad_numbers_are_jumpers: Option<bool>,
    pub pad_count: usize,
    pub model_count: usize,
    pub zone_count: usize,
    pub group_count: usize,
    pub fp_line_count: usize,
    pub fp_rect_count: usize,
    pub fp_circle_count: usize,
    pub fp_arc_count: usize,
    pub fp_poly_count: usize,
    pub fp_curve_count: usize,
    pub fp_text_count: usize,
    pub fp_text_box_count: usize,
    pub dimension_count: usize,
    pub graphic_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
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

    pub fn ast_mut(&mut self) -> &mut FootprintAst {
        &mut self.ast
    }

    pub fn cst(&self) -> &CstDocument {
        &self.cst
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
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

pub struct FootprintFile;

impl FootprintFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<FootprintDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        let legacy_module_root = ensure_head(&cst)?;
        let ast = parse_ast(&cst);
        let mut diagnostics = validate_version(ast.version)?;
        if legacy_module_root {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: "legacy_root",
                message:
                    "legacy root token `module` detected; parsing in compatibility mode"
                        .to_string(),
                span: None,
                hint: Some(
                    "save from newer KiCad to normalize root token to `footprint`".to_string(),
                ),
            });
        }
        Ok(FootprintDocument {
            ast,
            cst,
            diagnostics,
        })
    }
}

fn ensure_head(cst: &CstDocument) -> Result<bool, Error> {
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
        Some("footprint") => Ok(false),
        Some("module") => Ok(true),
        Some(h) => Err(Error::Validation(format!(
            "expected root token `footprint` or legacy `module`, got `{h}`"
        ))),
        None => Err(Error::Validation("missing root token".to_string())),
    }
}

fn parse_ast(cst: &CstDocument) -> FootprintAst {
    let mut lib_id = None;
    let mut version = None;
    let mut tedit = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut layer = None;
    let mut descr = None;
    let mut tags = None;
    let mut property_count = 0usize;
    let mut attr_present = false;
    let mut locked_present = false;
    let mut private_layers_present = false;
    let mut net_tie_pad_groups_present = false;
    let mut embedded_fonts_present = false;
    let mut has_embedded_files = false;
    let mut embedded_file_count = 0usize;
    let mut clearance = None;
    let mut solder_mask_margin = None;
    let mut solder_paste_margin = None;
    let mut solder_paste_margin_ratio = None;
    let mut duplicate_pad_numbers_are_jumpers = None;
    let mut pad_count = 0usize;
    let mut model_count = 0usize;
    let mut zone_count = 0usize;
    let mut group_count = 0usize;
    let mut fp_line_count = 0usize;
    let mut fp_rect_count = 0usize;
    let mut fp_circle_count = 0usize;
    let mut fp_arc_count = 0usize;
    let mut fp_poly_count = 0usize;
    let mut fp_curve_count = 0usize;
    let mut fp_text_count = 0usize;
    let mut fp_text_box_count = 0usize;
    let mut dimension_count = 0usize;
    let mut graphic_count = 0usize;
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        lib_id = items.get(1).and_then(atom_as_string);
        for item in items.iter().skip(2) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("tedit") => tedit = second_atom_string(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("layer") => layer = second_atom_string(item),
                Some("descr") => descr = second_atom_string(item),
                Some("tags") => tags = second_atom_string(item),
                Some("property") => property_count += 1,
                Some("attr") => attr_present = true,
                Some("locked") => locked_present = true,
                Some("private_layers") => private_layers_present = true,
                Some("net_tie_pad_groups") => net_tie_pad_groups_present = true,
                Some("embedded_fonts") => embedded_fonts_present = true,
                Some("embedded_files") => {
                    has_embedded_files = true;
                    embedded_file_count = list_child_head_count(item, "file");
                }
                Some("clearance") => clearance = second_atom_string(item),
                Some("solder_mask_margin") => solder_mask_margin = second_atom_string(item),
                Some("solder_paste_margin") => solder_paste_margin = second_atom_string(item),
                Some("solder_paste_margin_ratio") => {
                    solder_paste_margin_ratio = second_atom_string(item)
                }
                Some("duplicate_pad_numbers_are_jumpers") => {
                    duplicate_pad_numbers_are_jumpers = second_atom_string(item).and_then(|s| {
                        match s.as_str() {
                            "yes" => Some(true),
                            "no" => Some(false),
                            _ => None,
                        }
                    })
                }
                Some("pad") => pad_count += 1,
                Some("model") => model_count += 1,
                Some("zone") => zone_count += 1,
                Some("group") => group_count += 1,
                Some("fp_line") => {
                    fp_line_count += 1;
                    graphic_count += 1;
                }
                Some("fp_rect") => {
                    fp_rect_count += 1;
                    graphic_count += 1;
                }
                Some("fp_circle") => {
                    fp_circle_count += 1;
                    graphic_count += 1;
                }
                Some("fp_arc") => {
                    fp_arc_count += 1;
                    graphic_count += 1;
                }
                Some("fp_poly") => {
                    fp_poly_count += 1;
                    graphic_count += 1;
                }
                Some("fp_curve") => {
                    fp_curve_count += 1;
                    graphic_count += 1;
                }
                Some("fp_text") => {
                    fp_text_count += 1;
                    graphic_count += 1;
                }
                Some("fp_text_box") => {
                    fp_text_box_count += 1;
                    graphic_count += 1;
                }
                Some("dimension") => dimension_count += 1,
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    FootprintAst {
        lib_id,
        version,
        tedit,
        generator,
        generator_version,
        layer,
        descr,
        tags,
        property_count,
        attr_present,
        locked_present,
        private_layers_present,
        net_tie_pad_groups_present,
        embedded_fonts_present,
        has_embedded_files,
        embedded_file_count,
        clearance,
        solder_mask_margin,
        solder_paste_margin,
        solder_paste_margin_ratio,
        duplicate_pad_numbers_are_jumpers,
        pad_count,
        model_count,
        zone_count,
        group_count,
        fp_line_count,
        fp_rect_count,
        fp_circle_count,
        fp_arc_count,
        fp_poly_count,
        fp_curve_count,
        fp_text_count,
        fp_text_box_count,
        dimension_count,
        graphic_count,
        unknown_nodes,
    }
}

fn validate_version(version: Option<i32>) -> Result<Vec<Diagnostic>, Error> {
    let policy = VersionPolicy::default();
    let mut diagnostics = Vec::new();

    if let Some(v) = version {
        if policy.reject_older && !policy.accepts(v) {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: "legacy_format",
                message: format!(
                    "version {v} is older than v9 target; parsing in compatibility mode"
                ),
                span: None,
                hint: Some(
                    "parser support for pre-v9 token variants is best-effort".to_string(),
                ),
            });
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
        assert_eq!(doc.ast().lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().version, Some(20260101));
        assert_eq!(doc.ast().generator.as_deref(), Some("pcbnew"));
        assert!(doc.ast().unknown_nodes.is_empty());
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

    #[test]
    fn read_footprint_warns_on_legacy_version() {
        let path = tmp_file("footprint_legacy");
        fs::write(
            &path,
            "(footprint \"R\" (version 20221018) (generator pcbnew))\n",
        )
        .expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_format");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_accepts_legacy_module_root() {
        let path = tmp_file("footprint_module_root");
        let src = "(module R_0603 (layer F.Cu) (tedit 5F0C7995) (attr smd))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().lib_id.as_deref(), Some("R_0603"));
        assert_eq!(doc.ast().tedit.as_deref(), Some("5F0C7995"));
        assert!(doc.ast().attr_present);
        assert_eq!(doc.diagnostics().len(), 1);
        assert_eq!(doc.diagnostics()[0].code, "legacy_root");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_captures_unknown_nodes() {
        let path = tmp_file("footprint_unknown");
        let src =
            "(footprint \"R\" (version 20260101) (generator pcbnew) (future_shape foo bar))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);
        assert_eq!(doc.ast().unknown_nodes[0].head.as_deref(), Some("future_shape"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_footprint_parses_top_level_counts() {
        let path = tmp_file("footprint_counts");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew) (generator_version \"10.0\") (layer \"F.Cu\")\n  (descr \"demo\")\n  (tags \"a b\")\n  (property \"Reference\" \"R?\")\n  (property \"Value\" \"X\")\n  (attr smd)\n  (private_layers \"In1.Cu\")\n  (net_tie_pad_groups \"1,2\")\n  (solder_mask_margin 0.02)\n  (solder_paste_margin -0.01)\n  (solder_paste_margin_ratio -0.2)\n  (duplicate_pad_numbers_are_jumpers yes)\n  (fp_text reference \"R1\" (at 0 0) (layer \"F.SilkS\"))\n  (fp_line (start 0 0) (end 1 1) (layer \"F.SilkS\"))\n  (pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\" \"F.Mask\"))\n  (model \"foo.step\")\n  (zone)\n  (group (id \"g1\"))\n  (dimension)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().lib_id.as_deref(), Some("X"));
        assert_eq!(doc.ast().generator_version.as_deref(), Some("10.0"));
        assert_eq!(doc.ast().layer.as_deref(), Some("F.Cu"));
        assert_eq!(doc.ast().property_count, 2);
        assert!(doc.ast().attr_present);
        assert!(!doc.ast().locked_present);
        assert!(doc.ast().private_layers_present);
        assert!(doc.ast().net_tie_pad_groups_present);
        assert!(!doc.ast().embedded_fonts_present);
        assert!(!doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 0);
        assert_eq!(doc.ast().clearance, None);
        assert_eq!(doc.ast().solder_mask_margin.as_deref(), Some("0.02"));
        assert_eq!(doc.ast().solder_paste_margin.as_deref(), Some("-0.01"));
        assert_eq!(doc.ast().solder_paste_margin_ratio.as_deref(), Some("-0.2"));
        assert_eq!(doc.ast().duplicate_pad_numbers_are_jumpers, Some(true));
        assert_eq!(doc.ast().fp_text_count, 1);
        assert_eq!(doc.ast().fp_line_count, 1);
        assert_eq!(doc.ast().graphic_count, 2);
        assert_eq!(doc.ast().pad_count, 1);
        assert_eq!(doc.ast().model_count, 1);
        assert_eq!(doc.ast().zone_count, 1);
        assert_eq!(doc.ast().group_count, 1);
        assert_eq!(doc.ast().dimension_count, 1);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_embedded_fonts_regression() {
        let path = tmp_file("footprint_embedded_fonts");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew) (embedded_fonts no))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().embedded_fonts_present);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_locked_regression() {
        let path = tmp_file("footprint_locked");
        let src = "(footprint \"X\" (locked) (version 20260101) (generator pcbnew))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().locked_present);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_solder_margins_and_jumpers_regression() {
        let path = tmp_file("footprint_margins_jumpers");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew)\n  (clearance 0.15)\n  (solder_mask_margin 0.03)\n  (solder_paste_margin -0.02)\n  (solder_paste_margin_ratio -0.3)\n  (duplicate_pad_numbers_are_jumpers no)\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert_eq!(doc.ast().clearance.as_deref(), Some("0.15"));
        assert_eq!(doc.ast().solder_mask_margin.as_deref(), Some("0.03"));
        assert_eq!(doc.ast().solder_paste_margin.as_deref(), Some("-0.02"));
        assert_eq!(doc.ast().solder_paste_margin_ratio.as_deref(), Some("-0.3"));
        assert_eq!(doc.ast().duplicate_pad_numbers_are_jumpers, Some(false));
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }

    #[test]
    fn parses_embedded_files_regression() {
        let path = tmp_file("footprint_embedded_files");
        let src = "(footprint \"X\" (version 20260101) (generator pcbnew)\n  (embedded_files\n    (file \"A\" \"base64\")\n    (file \"B\" \"base64\")\n  )\n)\n";
        fs::write(&path, src).expect("write fixture");

        let doc = FootprintFile::read(&path).expect("read");
        assert!(doc.ast().has_embedded_files);
        assert_eq!(doc.ast().embedded_file_count, 2);
        assert!(doc.ast().unknown_nodes.is_empty());

        let _ = fs::remove_file(path);
    }
}

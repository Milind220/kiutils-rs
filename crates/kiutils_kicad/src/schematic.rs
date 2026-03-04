use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_one, CstDocument, Node};

use crate::diagnostic::Diagnostic;
use crate::sections::{parse_paper, parse_title_block, ParsedPaper, ParsedTitleBlock};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, ensure_root_head_any, find_property_index, mutate_root_and_refresh,
    paper_standard_node, paper_user_node, remove_property, upsert_node,
    upsert_property_preserve_tail, upsert_scalar, upsert_section_child_scalar,
};
use crate::sexpr_utils::{
    atom_as_string, head_of, list_child_head_count, second_atom_bool, second_atom_i32,
    second_atom_string,
};
use crate::version_diag::collect_version_diagnostics;
use crate::{Error, UnknownNode, WriteMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicPaperSummary {
    pub kind: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub orientation: Option<String>,
}

impl From<ParsedPaper> for SchematicPaperSummary {
    fn from(value: ParsedPaper) -> Self {
        Self {
            kind: value.kind,
            width: value.width,
            height: value.height,
            orientation: value.orientation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicTitleBlockSummary {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<String>,
}

impl From<ParsedTitleBlock> for SchematicTitleBlockSummary {
    fn from(value: ParsedTitleBlock) -> Self {
        Self {
            title: value.title,
            date: value.date,
            revision: value.revision,
            company: value.company,
            comments: value.comments,
        }
    }
}

/// Summary of a symbol instance embedded in a schematic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicSymbolInfo {
    pub reference: Option<String>,
    pub lib_id: Option<String>,
    pub value: Option<String>,
    pub footprint: Option<String>,
    /// All properties as (key, value) pairs.
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SchematicAst {
    pub version: Option<i32>,
    pub generator: Option<String>,
    pub generator_version: Option<String>,
    pub uuid: Option<String>,
    pub has_paper: bool,
    pub paper: Option<SchematicPaperSummary>,
    pub has_title_block: bool,
    pub title_block: Option<SchematicTitleBlockSummary>,
    pub has_lib_symbols: bool,
    pub embedded_fonts: Option<bool>,
    pub lib_symbol_count: usize,
    pub symbol_count: usize,
    pub sheet_count: usize,
    pub junction_count: usize,
    pub no_connect_count: usize,
    pub bus_entry_count: usize,
    pub bus_alias_count: usize,
    pub wire_count: usize,
    pub bus_count: usize,
    pub image_count: usize,
    pub text_count: usize,
    pub text_box_count: usize,
    pub label_count: usize,
    pub global_label_count: usize,
    pub hierarchical_label_count: usize,
    pub netclass_flag_count: usize,
    pub polyline_count: usize,
    pub rectangle_count: usize,
    pub circle_count: usize,
    pub arc_count: usize,
    pub rule_area_count: usize,
    pub sheet_instance_count: usize,
    pub symbol_instance_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}

#[derive(Debug, Clone)]
pub struct SchematicDocument {
    ast: SchematicAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

impl SchematicDocument {
    pub fn ast(&self) -> &SchematicAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut SchematicAst {
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

    pub fn set_uuid<S: Into<String>>(&mut self, uuid: S) -> &mut Self {
        self.mutate_root_items(|items| upsert_scalar(items, "uuid", atom_quoted(uuid.into()), 1))
    }

    pub fn set_paper_standard<S: Into<String>>(
        &mut self,
        kind: S,
        orientation: Option<&str>,
    ) -> &mut Self {
        let node = paper_standard_node(kind.into(), orientation.map(|v| v.to_string()));
        self.mutate_root_items(|items| upsert_node(items, "paper", node, 1))
    }

    pub fn set_paper_user(
        &mut self,
        width: f64,
        height: f64,
        orientation: Option<&str>,
    ) -> &mut Self {
        let node = paper_user_node(width, height, orientation.map(|v| v.to_string()));
        self.mutate_root_items(|items| upsert_node(items, "paper", node, 1))
    }

    pub fn set_title<S: Into<String>>(&mut self, title: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(items, "title_block", 1, "title", atom_quoted(title.into()))
        })
    }

    pub fn set_date<S: Into<String>>(&mut self, date: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(items, "title_block", 1, "date", atom_quoted(date.into()))
        })
    }

    pub fn set_revision<S: Into<String>>(&mut self, revision: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(
                items,
                "title_block",
                1,
                "rev",
                atom_quoted(revision.into()),
            )
        })
    }

    pub fn set_company<S: Into<String>>(&mut self, company: S) -> &mut Self {
        self.mutate_root_items(|items| {
            upsert_section_child_scalar(
                items,
                "title_block",
                1,
                "company",
                atom_quoted(company.into()),
            )
        })
    }

    pub fn set_embedded_fonts(&mut self, enabled: bool) -> &mut Self {
        let value = if enabled { "yes" } else { "no" };
        self.mutate_root_items(|items| {
            upsert_scalar(items, "embedded_fonts", atom_symbol(value.to_string()), 1)
        })
    }

    /// Return filenames of sub-sheets referenced by `(sheet ...)` nodes.
    ///
    /// The filenames come from the `Sheetfile` property on each sheet node
    /// and are relative to the directory containing this schematic.
    pub fn sheet_filenames(&self) -> Vec<String> {
        let items = match self.cst.nodes.first() {
            Some(Node::List { items, .. }) => items,
            _ => return Vec::new(),
        };
        items
            .iter()
            .skip(1)
            .filter(|node| head_of(node) == Some("sheet"))
            .filter_map(|node| {
                let Node::List {
                    items: sheet_items, ..
                } = node
                else {
                    return None;
                };
                // Look for (property "Sheetfile" "filename.kicad_sch" ...)
                find_property_index(sheet_items, "Sheetfile", 1).and_then(|idx| {
                    if let Some(Node::List {
                        items: prop_items, ..
                    }) = sheet_items.get(idx)
                    {
                        prop_items.get(2).and_then(atom_as_string)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    /// Return info for all symbol instances in the schematic.
    pub fn symbol_instances(&self) -> Vec<SchematicSymbolInfo> {
        let items = match self.cst.nodes.first() {
            Some(Node::List { items, .. }) => items,
            _ => return Vec::new(),
        };
        items
            .iter()
            .skip(1)
            .filter(|node| head_of(node) == Some("symbol"))
            .map(parse_schematic_symbol_info)
            .collect()
    }

    /// Upsert a property on every symbol instance matching `reference`.
    pub fn upsert_symbol_instance_property<R: Into<String>, K: Into<String>, V: Into<String>>(
        &mut self,
        reference: R,
        key: K,
        value: V,
    ) -> &mut Self {
        let reference = reference.into();
        let key = key.into();
        let value = value.into();
        self.mutate_root_items(|items| {
            let indices = find_schematic_symbol_indices_by_reference(items, &reference);
            let mut changed = false;
            for idx in indices {
                if let Some(Node::List {
                    items: sym_items, ..
                }) = items.get_mut(idx)
                {
                    if upsert_property_preserve_tail(sym_items, &key, &value, 1) {
                        changed = true;
                    }
                }
            }
            changed
        })
    }

    /// Remove a property from every symbol instance matching `reference`.
    pub fn remove_symbol_instance_property<R: Into<String>, K: Into<String>>(
        &mut self,
        reference: R,
        key: K,
    ) -> &mut Self {
        let reference = reference.into();
        let key = key.into();
        self.mutate_root_items(|items| {
            let indices = find_schematic_symbol_indices_by_reference(items, &reference);
            let mut changed = false;
            for idx in indices {
                if let Some(Node::List {
                    items: sym_items, ..
                }) = items.get_mut(idx)
                {
                    if remove_property(sym_items, &key, 1) {
                        changed = true;
                    }
                }
            }
            changed
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

pub struct SchematicFile;

impl SchematicFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<SchematicDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_one(&raw)?;
        ensure_root_head_any(&cst, &["kicad_sch"])?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_version_diagnostics(ast.version);
        Ok(SchematicDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

/// Find indices of root-level `(symbol ...)` nodes whose "Reference" property matches.
fn find_schematic_symbol_indices_by_reference(items: &[Node], reference: &str) -> Vec<usize> {
    items
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, node)| {
            if head_of(node) != Some("symbol") {
                return false;
            }
            let Node::List {
                items: sym_items, ..
            } = node
            else {
                return false;
            };
            if let Some(prop_idx) = find_property_index(sym_items, "Reference", 1) {
                if let Some(Node::List {
                    items: prop_items, ..
                }) = sym_items.get(prop_idx)
                {
                    return prop_items.get(2).and_then(atom_as_string).as_deref()
                        == Some(reference);
                }
            }
            false
        })
        .map(|(idx, _)| idx)
        .collect()
}

/// Extract property value from a symbol node's items.
fn get_property_value(sym_items: &[Node], key: &str) -> Option<String> {
    find_property_index(sym_items, key, 1).and_then(|idx| {
        if let Some(Node::List {
            items: prop_items, ..
        }) = sym_items.get(idx)
        {
            prop_items.get(2).and_then(atom_as_string)
        } else {
            None
        }
    })
}

fn parse_schematic_symbol_info(node: &Node) -> SchematicSymbolInfo {
    let Node::List { items, .. } = node else {
        return SchematicSymbolInfo {
            reference: None,
            lib_id: None,
            value: None,
            footprint: None,
            properties: Vec::new(),
        };
    };

    let lib_id = items
        .iter()
        .skip(1)
        .find(|n| head_of(n) == Some("lib_id"))
        .and_then(second_atom_string);

    let reference = get_property_value(items, "Reference");
    let value = get_property_value(items, "Value");
    let footprint = get_property_value(items, "Footprint");

    let properties: Vec<(String, String)> = items
        .iter()
        .skip(1)
        .filter(|n| head_of(n) == Some("property"))
        .filter_map(|n| {
            let Node::List {
                items: prop_items, ..
            } = n
            else {
                return None;
            };
            let key = prop_items.get(1).and_then(atom_as_string)?;
            let val = prop_items
                .get(2)
                .and_then(atom_as_string)
                .unwrap_or_default();
            Some((key, val))
        })
        .collect();

    SchematicSymbolInfo {
        reference,
        lib_id,
        value,
        footprint,
        properties,
    }
}

fn parse_ast(cst: &CstDocument) -> SchematicAst {
    let mut version = None;
    let mut generator = None;
    let mut generator_version = None;
    let mut uuid = None;
    let mut has_paper = false;
    let mut paper = None;
    let mut has_title_block = false;
    let mut title_block = None;
    let mut has_lib_symbols = false;
    let mut embedded_fonts = None;
    let mut lib_symbol_count = 0usize;
    let mut symbol_count = 0usize;
    let mut sheet_count = 0usize;
    let mut junction_count = 0usize;
    let mut no_connect_count = 0usize;
    let mut bus_entry_count = 0usize;
    let mut bus_alias_count = 0usize;
    let mut wire_count = 0usize;
    let mut bus_count = 0usize;
    let mut image_count = 0usize;
    let mut text_count = 0usize;
    let mut text_box_count = 0usize;
    let mut label_count = 0usize;
    let mut global_label_count = 0usize;
    let mut hierarchical_label_count = 0usize;
    let mut netclass_flag_count = 0usize;
    let mut polyline_count = 0usize;
    let mut rectangle_count = 0usize;
    let mut circle_count = 0usize;
    let mut arc_count = 0usize;
    let mut rule_area_count = 0usize;
    let mut sheet_instance_count = 0usize;
    let mut symbol_instance_count = 0usize;
    let mut unknown_nodes = Vec::new();

    if let Some(Node::List { items, .. }) = cst.nodes.first() {
        for item in items.iter().skip(1) {
            match head_of(item) {
                Some("version") => version = second_atom_i32(item),
                Some("generator") => generator = second_atom_string(item),
                Some("generator_version") => generator_version = second_atom_string(item),
                Some("uuid") => uuid = second_atom_string(item),
                Some("paper") => {
                    has_paper = true;
                    paper = Some(parse_paper(item).into());
                }
                Some("title_block") => {
                    has_title_block = true;
                    title_block = Some(parse_title_block(item).into());
                }
                Some("lib_symbols") => {
                    has_lib_symbols = true;
                    lib_symbol_count = list_child_head_count(item, "symbol");
                }
                Some("symbol") => symbol_count += 1,
                Some("sheet") => sheet_count += 1,
                Some("junction") => junction_count += 1,
                Some("no_connect") => no_connect_count += 1,
                Some("bus_entry") => bus_entry_count += 1,
                Some("bus_alias") => bus_alias_count += 1,
                Some("wire") => wire_count += 1,
                Some("bus") => bus_count += 1,
                Some("image") => image_count += 1,
                Some("text") => text_count += 1,
                Some("text_box") => text_box_count += 1,
                Some("label") => label_count += 1,
                Some("global_label") => global_label_count += 1,
                Some("hierarchical_label") => hierarchical_label_count += 1,
                Some("netclass_flag") => netclass_flag_count += 1,
                Some("polyline") => polyline_count += 1,
                Some("rectangle") => rectangle_count += 1,
                Some("circle") => circle_count += 1,
                Some("arc") => arc_count += 1,
                Some("rule_area") => rule_area_count += 1,
                Some("sheet_instances") => {
                    sheet_instance_count = list_child_head_count(item, "path");
                }
                Some("symbol_instances") => {
                    symbol_instance_count = list_child_head_count(item, "path");
                }
                Some("embedded_fonts") => {
                    embedded_fonts = second_atom_bool(item);
                }
                _ => {
                    if let Some(unknown) = UnknownNode::from_node(item) {
                        unknown_nodes.push(unknown);
                    }
                }
            }
        }
    }

    SchematicAst {
        version,
        generator,
        generator_version,
        uuid,
        has_paper,
        paper,
        has_title_block,
        title_block,
        has_lib_symbols,
        embedded_fonts,
        lib_symbol_count,
        symbol_count,
        sheet_count,
        junction_count,
        no_connect_count,
        bus_entry_count,
        bus_alias_count,
        wire_count,
        bus_count,
        image_count,
        text_count,
        text_box_count,
        label_count,
        global_label_count,
        hierarchical_label_count,
        netclass_flag_count,
        polyline_count,
        rectangle_count,
        circle_count,
        arc_count,
        rule_area_count,
        sheet_instance_count,
        symbol_instance_count,
        unknown_nodes,
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
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_sch"))
    }

    #[test]
    fn read_schematic_and_preserve_lossless() {
        let path = tmp_file("sch_read_ok");
        let src = "(kicad_sch (version 20250114) (generator \"eeschema\") (generator_version \"9.0\") (uuid \"u-1\") (paper \"A4\") (title_block (title \"Demo\") (date \"2026-02-25\") (comment 2 \"c2\") (comment 1 \"c1\")) (lib_symbols (symbol \"Lib:R\")) (symbol (lib_id \"Lib:R\")) (wire (pts (xy 0 0) (xy 1 1))) (sheet_instances (path \"/\" (page \"1\"))) (embedded_fonts no))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SchematicFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(20250114));
        assert_eq!(doc.ast().generator.as_deref(), Some("eeschema"));
        assert_eq!(doc.ast().generator_version.as_deref(), Some("9.0"));
        assert_eq!(doc.ast().uuid.as_deref(), Some("u-1"));
        assert_eq!(
            doc.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("A4".to_string())
        );
        assert_eq!(doc.ast().lib_symbol_count, 1);
        assert_eq!(doc.ast().symbol_count, 1);
        assert_eq!(doc.ast().wire_count, 1);
        assert_eq!(doc.ast().sheet_instance_count, 1);
        assert_eq!(doc.ast().embedded_fonts, Some(false));
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn captures_unknown_nodes_roundtrip() {
        let path = tmp_file("sch_unknown");
        let src = "(kicad_sch (version 20250114) (generator \"eeschema\") (future_block 1 2) (symbol (lib_id \"Device:R\")))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = SchematicFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);

        let out = tmp_file("sch_unknown_out");
        doc.write(&out).expect("write");
        let got = fs::read_to_string(&out).expect("read out");
        assert_eq!(got, src);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn edit_roundtrip_updates_core_fields() {
        let path = tmp_file("sch_edit");
        let src = "(kicad_sch (version 20241229) (generator \"eeschema\") (paper \"A4\") (title_block (title \"Old\") (date \"2025-01-01\") (rev \"A\") (company \"OldCo\")) (future_token 1 2))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = SchematicFile::read(&path).expect("read");
        doc.set_version(20260101)
            .set_generator("kiutils")
            .set_generator_version("dev")
            .set_uuid("uuid-new")
            .set_paper_user(297.0, 210.0, Some("landscape"))
            .set_title("New")
            .set_date("2026-02-25")
            .set_revision("B")
            .set_company("Lords")
            .set_embedded_fonts(true);

        let out = tmp_file("sch_edit_out");
        doc.write(&out).expect("write");
        let reread = SchematicFile::read(&out).expect("reread");

        assert_eq!(reread.ast().version, Some(20260101));
        assert_eq!(reread.ast().generator.as_deref(), Some("kiutils"));
        assert_eq!(reread.ast().generator_version.as_deref(), Some("dev"));
        assert_eq!(reread.ast().uuid.as_deref(), Some("uuid-new"));
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.kind.clone()),
            Some("User".to_string())
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.width),
            Some(297.0)
        );
        assert_eq!(
            reread.ast().paper.as_ref().and_then(|p| p.height),
            Some(210.0)
        );
        assert_eq!(reread.ast().embedded_fonts, Some(true));
        assert_eq!(reread.ast().unknown_nodes.len(), 1);
        assert_eq!(
            reread
                .ast()
                .title_block
                .as_ref()
                .and_then(|t| t.title.clone()),
            Some("New".to_string())
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }
}

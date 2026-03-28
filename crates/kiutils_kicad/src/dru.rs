use std::fs;
use std::path::Path;

use kiutils_sexpr::{parse_rootless, Atom, CstDocument, Node};

use crate::diagnostic::{Diagnostic, Severity};
use crate::sexpr_edit::{
    atom_quoted, atom_symbol, child_index, list_node, mutate_nodes_and_refresh_rootless,
    upsert_scalar,
};
use crate::sexpr_utils::{
    atom_as_f64, atom_as_string, head_of, second_atom_f64, second_atom_i32, second_atom_string,
};
use crate::{Error, UnknownNode, WriteMode};

/// A single constraint within a design rule.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DruConstraint {
    pub constraint_type: Option<String>,
    pub min: Option<f64>,
    pub opt: Option<f64>,
    pub max: Option<f64>,
    pub value: Option<f64>,
}

/// A severity override for a specific violation type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DruSeverity {
    pub violation_type: Option<String>,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DesignRule {
    pub name: Option<String>,
    pub constraint_count: usize,
    pub condition: Option<String>,
    pub layer: Option<String>,
    pub constraints: Vec<DruConstraint>,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DesignRulesAst {
    pub version: Option<i32>,
    pub rules: Vec<DesignRule>,
    pub rule_count: usize,
    pub total_constraint_count: usize,
    pub rules_with_condition_count: usize,
    pub rules_with_layer_count: usize,
    pub severities: Vec<DruSeverity>,
    pub severity_count: usize,
    pub unknown_nodes: Vec<UnknownNode>,
}
#[derive(Debug, Clone)]
pub struct DesignRulesDocument {
    ast: DesignRulesAst,
    cst: CstDocument,
    diagnostics: Vec<Diagnostic>,
    ast_dirty: bool,
}

impl DesignRulesDocument {
    pub fn ast(&self) -> &DesignRulesAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut DesignRulesAst {
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
        let version_node = list_node(vec![
            atom_symbol("version".to_string()),
            atom_symbol(version.to_string()),
        ]);
        self.mutate_nodes(|nodes| {
            if let Some(idx) = child_index(nodes, "version", 0) {
                if nodes[idx] == version_node {
                    false
                } else {
                    nodes[idx] = version_node;
                    true
                }
            } else {
                nodes.insert(0, version_node);
                true
            }
        })
    }

    pub fn add_rule<S: Into<String>>(&mut self, name: S) -> &mut Self {
        let name = name.into();
        let node = list_node(vec![atom_symbol("rule".to_string()), atom_quoted(name)]);
        self.mutate_nodes(|nodes| {
            nodes.push(node);
            true
        })
    }

    pub fn rename_rule<S: Into<String>>(&mut self, from: &str, to: S) -> &mut Self {
        let from = from.to_string();
        let to = to.into();
        self.mutate_nodes(|nodes| {
            let Some(idx) = find_rule_index(nodes, &from) else {
                return false;
            };
            let Some(Node::List { items, .. }) = nodes.get_mut(idx) else {
                return false;
            };
            if items.len() < 2 {
                return false;
            }
            let next = atom_quoted(to);
            if items[1] == next {
                false
            } else {
                items[1] = next;
                true
            }
        })
    }

    pub fn rename_first_rule<S: Into<String>>(&mut self, to: S) -> &mut Self {
        let to = to.into();
        self.mutate_nodes(|nodes| {
            let Some(idx) = nodes
                .iter()
                .enumerate()
                .find(|(_, node)| head_of(node) == Some("rule"))
                .map(|(idx, _)| idx)
            else {
                return false;
            };
            let Some(Node::List { items, .. }) = nodes.get_mut(idx) else {
                return false;
            };
            if items.len() < 2 {
                return false;
            }
            let next = atom_quoted(to);
            if items[1] == next {
                false
            } else {
                items[1] = next;
                true
            }
        })
    }

    pub fn upsert_rule_condition<S: Into<String>>(
        &mut self,
        rule_name: &str,
        condition: S,
    ) -> &mut Self {
        let rule_name = rule_name.to_string();
        let condition = condition.into();
        self.mutate_nodes(|nodes| {
            let Some(idx) = find_rule_index(nodes, &rule_name) else {
                return false;
            };
            let Some(Node::List { items, .. }) = nodes.get_mut(idx) else {
                return false;
            };
            upsert_scalar(items, "condition", atom_quoted(condition), 2)
        })
    }

    pub fn remove_rule_condition(&mut self, rule_name: &str) -> &mut Self {
        let rule_name = rule_name.to_string();
        self.mutate_nodes(|nodes| {
            let Some(idx) = find_rule_index(nodes, &rule_name) else {
                return false;
            };
            let Some(Node::List { items, .. }) = nodes.get_mut(idx) else {
                return false;
            };
            if let Some(cond_idx) = child_index(items, "condition", 2) {
                items.remove(cond_idx);
                true
            } else {
                false
            }
        })
    }

    pub fn upsert_rule_layer<S: Into<String>>(&mut self, rule_name: &str, layer: S) -> &mut Self {
        let rule_name = rule_name.to_string();
        let layer = layer.into();
        self.mutate_nodes(|nodes| {
            let Some(idx) = find_rule_index(nodes, &rule_name) else {
                return false;
            };
            let Some(Node::List { items, .. }) = nodes.get_mut(idx) else {
                return false;
            };
            upsert_scalar(items, "layer", atom_symbol(layer), 2)
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

    fn mutate_nodes<F>(&mut self, mutate: F) -> &mut Self
    where
        F: FnOnce(&mut Vec<Node>) -> bool,
    {
        mutate_nodes_and_refresh_rootless(
            &mut self.cst,
            &mut self.ast,
            &mut self.diagnostics,
            mutate,
            parse_ast,
            |_cst, ast| collect_diagnostics(ast.version),
        );
        self.ast_dirty = false;
        self
    }
}

pub struct DesignRulesFile;

impl DesignRulesFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<DesignRulesDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let cst = parse_rootless(&raw)?;
        let ast = parse_ast(&cst);
        let diagnostics = collect_diagnostics(ast.version);

        Ok(DesignRulesDocument {
            ast,
            cst,
            diagnostics,
            ast_dirty: false,
        })
    }
}

fn parse_ast(cst: &CstDocument) -> DesignRulesAst {
    let mut version = None;
    let mut rules = Vec::new();
    let mut severities = Vec::new();
    let mut unknown_nodes = Vec::new();

    for node in &cst.nodes {
        match head_of(node) {
            Some("version") => version = second_atom_i32(node),
            Some("rule") => rules.push(parse_rule(node)),
            Some("severity") => severities.push(parse_dru_severity(node)),
            _ => {
                if let Some(unknown) = UnknownNode::from_node(node) {
                    unknown_nodes.push(unknown);
                }
            }
        }
    }

    let rule_count = rules.len();
    let total_constraint_count = rules.iter().map(|r| r.constraint_count).sum();
    let rules_with_condition_count = rules.iter().filter(|r| r.condition.is_some()).count();
    let rules_with_layer_count = rules.iter().filter(|r| r.layer.is_some()).count();
    let severity_count = severities.len();

    DesignRulesAst {
        version,
        rules,
        rule_count,
        total_constraint_count,
        rules_with_condition_count,
        rules_with_layer_count,
        severities,
        severity_count,
        unknown_nodes,
    }
}

fn parse_rule(node: &Node) -> DesignRule {
    let Node::List { items, .. } = node else {
        return DesignRule {
            name: None,
            constraint_count: 0,
            condition: None,
            layer: None,
            constraints: Vec::new(),
            severity: None,
        };
    };

    let name = items.get(1).and_then(atom_as_string);
    let mut constraints = Vec::new();
    let mut condition = None;
    let mut layer = None;
    let mut severity = None;

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("constraint") => constraints.push(parse_dru_constraint(child)),
            Some("condition") => condition = second_atom_string(child),
            Some("layer") => layer = second_atom_string(child),
            Some("severity") => severity = second_atom_string(child),
            _ => {}
        }
    }

    let constraint_count = constraints.len();

    DesignRule {
        name,
        constraint_count,
        condition,
        layer,
        constraints,
        severity,
    }
}
fn parse_dru_constraint(node: &Node) -> DruConstraint {
    let Node::List { items, .. } = node else {
        return DruConstraint {
            constraint_type: None,
            min: None,
            opt: None,
            max: None,
            value: None,
        };
    };

    let constraint_type = items.get(1).and_then(|node| match node {
        Node::Atom {
            atom: Atom::Symbol(value),
            ..
        }
        | Node::Atom {
            atom: Atom::Quoted(value),
            ..
        } => Some(value.clone()),
        _ => None,
    });

    let parse_scalar = |child: &Node| {
        let Node::List { items, .. } = child else {
            return None;
        };
        items
            .get(1)
            .and_then(atom_as_f64)
            .or_else(|| second_atom_f64(child))
    };

    let mut min = None;
    let mut opt = None;
    let mut max = None;
    let mut value = None;

    for child in items.iter().skip(2) {
        match head_of(child) {
            Some("min") => min = parse_scalar(child),
            Some("opt") => opt = parse_scalar(child),
            Some("max") => max = parse_scalar(child),
            Some("value") => value = parse_scalar(child),
            _ => {}
        }
    }

    DruConstraint {
        constraint_type,
        min,
        opt,
        max,
        value,
    }
}

fn parse_dru_severity(node: &Node) -> DruSeverity {
    let Node::List { items, .. } = node else {
        return DruSeverity {
            violation_type: None,
            severity: None,
        };
    };

    DruSeverity {
        violation_type: items.get(1).and_then(atom_as_string),
        severity: items.get(2).and_then(atom_as_string),
    }
}

fn find_rule_index(nodes: &[Node], name: &str) -> Option<usize> {
    nodes
        .iter()
        .enumerate()
        .find(|(_, node)| {
            if head_of(node) != Some("rule") {
                return false;
            }
            match node {
                Node::List { items, .. } => {
                    items.get(1).and_then(atom_as_string).as_deref() == Some(name)
                }
                _ => false,
            }
        })
        .map(|(idx, _)| idx)
}

fn collect_diagnostics(version: Option<i32>) -> Vec<Diagnostic> {
    match version {
        Some(1) => Vec::new(),
        Some(other) => vec![Diagnostic {
            severity: Severity::Warning,
            code: "unsupported_version",
            message: format!(
                "unsupported design-rules version `{other}`; parsing in compatibility mode"
            ),
            span: None,
            hint: Some("expected `(version 1)` for KiCad v9/v10".to_string()),
        }],
        None => vec![Diagnostic {
            severity: Severity::Warning,
            code: "missing_version",
            message: "missing design-rules version token".to_string(),
            span: None,
            hint: Some("add top-level `(version 1)`".to_string()),
        }],
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
        let src =
            "(version 1)\n(rule \"x\" (constraint clearance (min \"0.1mm\")) (condition \"A\"))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = DesignRulesFile::read(&path).expect("read");
        assert_eq!(doc.ast().version, Some(1));
        assert_eq!(doc.ast().rule_count, 1);
        assert_eq!(doc.ast().total_constraint_count, 1);
        assert_eq!(doc.ast().rules_with_condition_count, 1);
        assert!(doc.ast().unknown_nodes.is_empty());
        assert!(doc.diagnostics().is_empty());
        assert_eq!(doc.cst().to_lossless_string(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_rootless_dru_captures_unknown_rule_item() {
        let path = tmp_file("dru_unknown");
        let src = "(version 1)\n(mystery xyz)\n(rule \"x\" (constraint clearance (min \"0.1mm\")) (condition \"A\"))\n";
        fs::write(&path, src).expect("write fixture");

        let doc = DesignRulesFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_nodes.len(), 1);
        assert_eq!(doc.ast().unknown_nodes[0].head.as_deref(), Some("mystery"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn edit_roundtrip_updates_rule_metadata() {
        let path = tmp_file("dru_edit");
        let src =
            "(version 1)\n(rule \"old\" (constraint clearance (min 0.1mm)) (condition \"A\"))\n";
        fs::write(&path, src).expect("write fixture");

        let mut doc = DesignRulesFile::read(&path).expect("read");
        doc.set_version(1)
            .rename_rule("old", "new")
            .upsert_rule_condition("new", "A.NetClass == 'DDR4'")
            .upsert_rule_layer("new", "outer");

        let out = tmp_file("dru_edit_out");
        doc.write(&out).expect("write");
        let reread = DesignRulesFile::read(&out).expect("reread");

        assert_eq!(reread.ast().version, Some(1));
        assert_eq!(reread.ast().rule_count, 1);
        assert_eq!(
            reread.ast().rules.first().and_then(|r| r.name.clone()),
            Some("new".to_string())
        );
        assert_eq!(
            reread.ast().rules.first().and_then(|r| r.layer.clone()),
            Some("outer".to_string())
        );
        assert_eq!(
            reread.ast().rules.first().and_then(|r| r.condition.clone()),
            Some("A.NetClass == 'DDR4'".to_string())
        );

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn warns_when_version_missing_or_unsupported() {
        let path_missing = tmp_file("dru_missing");
        fs::write(&path_missing, "(rule \"x\")\n").expect("write fixture");
        let missing = DesignRulesFile::read(&path_missing).expect("read");
        assert_eq!(missing.diagnostics().len(), 1);
        assert_eq!(missing.diagnostics()[0].code, "missing_version");

        let path_bad = tmp_file("dru_bad");
        fs::write(&path_bad, "(version 2)\n(rule \"x\")\n").expect("write fixture");
        let bad = DesignRulesFile::read(&path_bad).expect("read");
        assert_eq!(bad.diagnostics().len(), 1);
        assert_eq!(bad.diagnostics()[0].code, "unsupported_version");

        let _ = fs::remove_file(path_missing);
        let _ = fs::remove_file(path_bad);
    }
}

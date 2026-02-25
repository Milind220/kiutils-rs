use kiutils_sexpr::{parse_one, parse_rootless, Atom, CstDocument, Node, Span};

use crate::diagnostic::Diagnostic;
use crate::sexpr_utils::{atom_as_string, head_of};
use crate::Error;

pub(crate) fn span_zero() -> Span {
    Span { start: 0, end: 0 }
}

pub(crate) fn atom_symbol<S: Into<String>>(value: S) -> Node {
    Node::Atom {
        atom: Atom::Symbol(value.into()),
        span: span_zero(),
    }
}

pub(crate) fn atom_quoted<S: Into<String>>(value: S) -> Node {
    Node::Atom {
        atom: Atom::Quoted(value.into()),
        span: span_zero(),
    }
}

pub(crate) fn list_node(items: Vec<Node>) -> Node {
    Node::List {
        items,
        span: span_zero(),
    }
}

pub(crate) fn root_head(cst: &CstDocument) -> Option<&str> {
    cst.nodes
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
        })
}

pub(crate) fn ensure_root_head_any(cst: &CstDocument, expected: &[&str]) -> Result<String, Error> {
    let Some(head) = root_head(cst) else {
        return Err(Error::Validation("missing root token".to_string()));
    };

    if expected.iter().any(|e| *e == head) {
        Ok(head.to_string())
    } else {
        Err(Error::Validation(format!(
            "expected root token {}, got `{head}`",
            expected
                .iter()
                .map(|e| format!("`{e}`"))
                .collect::<Vec<_>>()
                .join(" or ")
        )))
    }
}

pub(crate) fn root_items_mut(cst: &mut CstDocument) -> Option<&mut Vec<Node>> {
    match cst.nodes.first_mut() {
        Some(Node::List { items, .. }) => Some(items),
        _ => None,
    }
}

pub(crate) fn child_index(items: &[Node], head: &str, skip: usize) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .skip(skip)
        .find(|(_, node)| head_of(node) == Some(head))
        .map(|(idx, _)| idx)
}

pub(crate) fn upsert_node(items: &mut Vec<Node>, head: &str, node: Node, skip: usize) -> bool {
    if let Some(idx) = child_index(items, head, skip) {
        if items[idx] == node {
            false
        } else {
            items[idx] = node;
            true
        }
    } else {
        items.push(node);
        true
    }
}

pub(crate) fn upsert_scalar(items: &mut Vec<Node>, head: &str, value: Node, skip: usize) -> bool {
    upsert_node(
        items,
        head,
        list_node(vec![atom_symbol(head.to_string()), value]),
        skip,
    )
}

pub(crate) fn upsert_section_child_scalar(
    items: &mut Vec<Node>,
    section_head: &str,
    section_skip: usize,
    child_head: &str,
    child_value: Node,
) -> bool {
    let node = list_node(vec![atom_symbol(child_head.to_string()), child_value]);
    upsert_section_child_node(items, section_head, section_skip, child_head, node)
}

pub(crate) fn upsert_section_child_node(
    items: &mut Vec<Node>,
    section_head: &str,
    section_skip: usize,
    child_head: &str,
    child_node: Node,
) -> bool {
    let section_idx = if let Some(idx) = child_index(items, section_head, section_skip) {
        idx
    } else {
        items.push(list_node(vec![atom_symbol(section_head.to_string())]));
        items.len() - 1
    };

    let Some(Node::List {
        items: section_items,
        ..
    }) = items.get_mut(section_idx)
    else {
        return false;
    };
    upsert_node(section_items, child_head, child_node, 1)
}

pub(crate) fn property_node(key: &str, value: &str) -> Node {
    list_node(vec![
        atom_symbol("property".to_string()),
        atom_quoted(key.to_string()),
        atom_quoted(value.to_string()),
    ])
}

pub(crate) fn find_property_index(items: &[Node], key: &str, skip: usize) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .skip(skip)
        .find(|(_, node)| {
            if head_of(node) != Some("property") {
                return false;
            }
            match node {
                Node::List {
                    items: prop_items, ..
                } => prop_items.get(1).and_then(atom_as_string).as_deref() == Some(key),
                _ => false,
            }
        })
        .map(|(idx, _)| idx)
}

pub(crate) fn upsert_property_preserve_tail(
    items: &mut Vec<Node>,
    key: &str,
    value: &str,
    skip: usize,
) -> bool {
    if let Some(idx) = find_property_index(items, key, skip) {
        match items.get_mut(idx) {
            Some(Node::List {
                items: prop_items, ..
            }) => {
                let replacement = atom_quoted(value.to_string());
                if prop_items.len() > 2 {
                    if prop_items[2] == replacement {
                        false
                    } else {
                        prop_items[2] = replacement;
                        true
                    }
                } else {
                    let full = property_node(key, value);
                    if items[idx] == full {
                        false
                    } else {
                        items[idx] = full;
                        true
                    }
                }
            }
            _ => false,
        }
    } else {
        items.push(property_node(key, value));
        true
    }
}

pub(crate) fn remove_property(items: &mut Vec<Node>, key: &str, skip: usize) -> bool {
    if let Some(idx) = find_property_index(items, key, skip) {
        items.remove(idx);
        true
    } else {
        false
    }
}

pub(crate) fn paper_standard_node(kind: String, orientation: Option<String>) -> Node {
    let mut nodes = vec![atom_symbol("paper".to_string()), atom_quoted(kind)];
    if let Some(orientation) = orientation {
        nodes.push(atom_symbol(orientation));
    }
    list_node(nodes)
}

pub(crate) fn paper_user_node(width: f64, height: f64, orientation: Option<String>) -> Node {
    let mut nodes = vec![
        atom_symbol("paper".to_string()),
        atom_quoted("User".to_string()),
        atom_symbol(width.to_string()),
        atom_symbol(height.to_string()),
    ];
    if let Some(orientation) = orientation {
        nodes.push(atom_symbol(orientation));
    }
    list_node(nodes)
}

pub(crate) fn canonicalize_and_reparse(cst: &mut CstDocument) {
    let canonical = cst.to_canonical_string();
    if let Ok(parsed) = parse_one(&canonical) {
        *cst = parsed;
    } else {
        cst.raw = canonical;
    }
}

fn node_eq_ignoring_span(a: &Node, b: &Node) -> bool {
    match (a, b) {
        (Node::Atom { atom: aa, .. }, Node::Atom { atom: ba, .. }) => aa == ba,
        (Node::List { items: ai, .. }, Node::List { items: bi, .. }) => {
            ai.len() == bi.len()
                && ai
                    .iter()
                    .zip(bi.iter())
                    .all(|(an, bn)| node_eq_ignoring_span(an, bn))
        }
        _ => false,
    }
}

fn span_of(node: &Node) -> Span {
    match node {
        Node::List { span, .. } | Node::Atom { span, .. } => *span,
    }
}

fn inline_canonical(node: &Node) -> String {
    let mut text = CstDocument {
        raw: String::new(),
        nodes: vec![node.clone()],
    }
    .to_canonical_string();
    if text.ends_with('\n') {
        text.pop();
    }
    text
}

fn replace_span(src: &str, span: Span, replacement: &str) -> Option<String> {
    if span.start > span.end || span.end > src.len() {
        return None;
    }
    let prefix = src.get(..span.start)?;
    let suffix = src.get(span.end..)?;
    Some(format!("{prefix}{replacement}{suffix}"))
}

fn patch_single_root_child_replacement_raw(
    before: &CstDocument,
    after: &CstDocument,
) -> Option<String> {
    let (before_items, after_items) = match (before.nodes.first(), after.nodes.first()) {
        (Some(Node::List { items: b, .. }), Some(Node::List { items: a, .. })) => (b, a),
        _ => return None,
    };
    if before_items.len() != after_items.len() || before_items.is_empty() {
        return None;
    }
    if !node_eq_ignoring_span(&before_items[0], &after_items[0]) {
        return None;
    }

    let mut changed_idx = None;
    for idx in 1..before_items.len() {
        if !node_eq_ignoring_span(&before_items[idx], &after_items[idx]) {
            if changed_idx.is_some() {
                return None;
            }
            changed_idx = Some(idx);
        }
    }

    let idx = changed_idx?;
    let span = span_of(&before_items[idx]);
    let replacement = inline_canonical(&after_items[idx]);
    replace_span(&before.raw, span, &replacement)
}

pub(crate) fn mutate_root_and_refresh<T, FM, FA, FD>(
    cst: &mut CstDocument,
    ast: &mut T,
    diagnostics: &mut Vec<Diagnostic>,
    mutate: FM,
    parse_ast: FA,
    collect_diagnostics: FD,
) where
    FM: FnOnce(&mut Vec<Node>) -> bool,
    FA: Fn(&CstDocument) -> T,
    FD: Fn(&CstDocument, &T) -> Vec<Diagnostic>,
{
    let before = cst.clone();
    let changed = root_items_mut(cst).map(mutate).unwrap_or(false);
    if !changed {
        return;
    }

    if let Some(patched_raw) = patch_single_root_child_replacement_raw(&before, cst) {
        if let Ok(parsed) = parse_one(&patched_raw) {
            *cst = parsed;
        } else {
            canonicalize_and_reparse(cst);
        }
    } else {
        canonicalize_and_reparse(cst);
    }
    *ast = parse_ast(cst);
    *diagnostics = collect_diagnostics(cst, ast);
}

pub(crate) fn canonicalize_and_reparse_rootless(cst: &mut CstDocument) {
    let canonical = cst.to_canonical_string();
    if let Ok(parsed) = parse_rootless(&canonical) {
        *cst = parsed;
    } else {
        cst.raw = canonical;
    }
}

pub(crate) fn mutate_nodes_and_refresh_rootless<T, FM, FA, FD>(
    cst: &mut CstDocument,
    ast: &mut T,
    diagnostics: &mut Vec<Diagnostic>,
    mutate: FM,
    parse_ast: FA,
    collect_diagnostics: FD,
) where
    FM: FnOnce(&mut Vec<Node>) -> bool,
    FA: Fn(&CstDocument) -> T,
    FD: Fn(&CstDocument, &T) -> Vec<Diagnostic>,
{
    let before = cst.clone();
    let changed = mutate(&mut cst.nodes);
    if !changed {
        return;
    }

    if before.nodes.len() == cst.nodes.len() {
        let mut changed_idx = None;
        for idx in 0..before.nodes.len() {
            if !node_eq_ignoring_span(&before.nodes[idx], &cst.nodes[idx]) {
                if changed_idx.is_some() {
                    changed_idx = None;
                    break;
                }
                changed_idx = Some(idx);
            }
        }

        if let Some(idx) = changed_idx {
            let span = span_of(&before.nodes[idx]);
            let replacement = inline_canonical(&cst.nodes[idx]);
            if let Some(patched_raw) = replace_span(&before.raw, span, &replacement) {
                if let Ok(parsed) = parse_rootless(&patched_raw) {
                    *cst = parsed;
                } else {
                    canonicalize_and_reparse_rootless(cst);
                }
            } else {
                canonicalize_and_reparse_rootless(cst);
            }
        } else {
            canonicalize_and_reparse_rootless(cst);
        }
    } else {
        canonicalize_and_reparse_rootless(cst);
    }
    *ast = parse_ast(cst);
    *diagnostics = collect_diagnostics(cst, ast);
}

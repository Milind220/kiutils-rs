use kiutils_sexpr::{parse_one, Atom, CstDocument, Node, Span};

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

pub(crate) fn upsert_child_scalar(items: &mut Vec<Node>, head: &str, value: Node) -> bool {
    upsert_scalar(items, head, value, 1)
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

pub(crate) fn canonicalize_and_reparse(cst: &mut CstDocument) {
    let canonical = cst.to_canonical_string();
    if let Ok(parsed) = parse_one(&canonical) {
        *cst = parsed;
    } else {
        cst.raw = canonical;
    }
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
    let changed = root_items_mut(cst).map(mutate).unwrap_or(false);
    if !changed {
        return;
    }

    canonicalize_and_reparse(cst);
    *ast = parse_ast(cst);
    *diagnostics = collect_diagnostics(cst, ast);
}

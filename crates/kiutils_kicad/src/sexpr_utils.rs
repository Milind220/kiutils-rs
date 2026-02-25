use kiutils_sexpr::{Atom, Node};

pub(crate) fn head_of(node: &Node) -> Option<&str> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let Some(Node::Atom {
        atom: Atom::Symbol(head),
        ..
    }) = items.first()
    else {
        return None;
    };
    Some(head.as_str())
}

pub(crate) fn atom_as_string(node: &Node) -> Option<String> {
    match node {
        Node::Atom {
            atom: Atom::Symbol(v),
            ..
        } => Some(v.clone()),
        Node::Atom {
            atom: Atom::Quoted(v),
            ..
        } => Some(v.clone()),
        _ => None,
    }
}

pub(crate) fn atom_as_i32(node: &Node) -> Option<i32> {
    atom_as_string(node).and_then(|s| s.parse::<i32>().ok())
}

pub(crate) fn second_atom_string(node: &Node) -> Option<String> {
    let Node::List { items, .. } = node else {
        return None;
    };
    items.get(1).and_then(atom_as_string)
}

pub(crate) fn second_atom_i32(node: &Node) -> Option<i32> {
    second_atom_string(node).and_then(|s| s.parse::<i32>().ok())
}

pub(crate) fn atom_as_f64(node: &Node) -> Option<f64> {
    atom_as_string(node).and_then(|s| s.parse::<f64>().ok())
}

pub(crate) fn second_atom_f64(node: &Node) -> Option<f64> {
    second_atom_string(node).and_then(|s| s.parse::<f64>().ok())
}

pub(crate) fn second_atom_bool(node: &Node) -> Option<bool> {
    match second_atom_string(node).as_deref() {
        Some("yes") => Some(true),
        Some("no") => Some(false),
        _ => None,
    }
}

pub(crate) fn list_child_head_count(node: &Node, head: &str) -> usize {
    let Node::List { items, .. } = node else {
        return 0;
    };
    items
        .iter()
        .filter(|child| matches!(head_of(child), Some(h) if h == head))
        .count()
}

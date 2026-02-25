use kiutils_sexpr::Node;

use crate::sexpr_utils::{atom_as_f64, atom_as_i32, atom_as_string, head_of};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedPaper {
    pub kind: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub orientation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedTitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company: Option<String>,
    pub comments: Vec<String>,
}

pub(crate) fn parse_paper(node: &Node) -> ParsedPaper {
    let Node::List { items, .. } = node else {
        return ParsedPaper {
            kind: None,
            width: None,
            height: None,
            orientation: None,
        };
    };

    let kind = items.get(1).and_then(atom_as_string);
    let width = items.get(2).and_then(atom_as_f64);
    let height = items.get(3).and_then(atom_as_f64);
    let orientation = if width.is_some() || height.is_some() {
        items.get(4).and_then(atom_as_string)
    } else {
        items.get(2).and_then(atom_as_string)
    };

    ParsedPaper {
        kind,
        width,
        height,
        orientation,
    }
}

pub(crate) fn parse_title_block(node: &Node) -> ParsedTitleBlock {
    let mut title = None;
    let mut date = None;
    let mut revision = None;
    let mut company = None;
    let mut comments: Vec<(i32, String)> = Vec::new();

    if let Node::List { items, .. } = node {
        for child in items.iter().skip(1) {
            match head_of(child) {
                Some("title") => title = child_as_value(child),
                Some("date") => date = child_as_value(child),
                Some("rev") => revision = child_as_value(child),
                Some("company") => company = child_as_value(child),
                Some("comment") => {
                    if let Some((idx, text)) = parse_comment(child) {
                        comments.push((idx, text));
                    }
                }
                _ => {}
            }
        }
    }

    comments.sort_by_key(|(idx, _)| *idx);
    let comments = comments.into_iter().map(|(_, text)| text).collect();

    ParsedTitleBlock {
        title,
        date,
        revision,
        company,
        comments,
    }
}

fn child_as_value(node: &Node) -> Option<String> {
    let Node::List { items, .. } = node else {
        return None;
    };
    items.get(1).and_then(atom_as_string)
}

fn parse_comment(node: &Node) -> Option<(i32, String)> {
    let Node::List { items, .. } = node else {
        return None;
    };
    let idx = items.get(1).and_then(atom_as_i32)?;
    let text = items.get(2).and_then(atom_as_string)?;
    Some((idx, text))
}

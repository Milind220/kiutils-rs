use kiutils_sexpr::{Atom, Node, Span as CstSpan};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnknownNode {
    pub head: Option<String>,
    pub span: CstSpan,
    pub node: Node,
}

impl UnknownNode {
    pub fn from_node(node: &Node) -> Option<Self> {
        match node {
            Node::List { items, span } => {
                let head = items.first().and_then(|n| match n {
                    Node::Atom {
                        atom: Atom::Symbol(s),
                        ..
                    } => Some(s.clone()),
                    _ => None,
                });
                Some(Self {
                    head,
                    span: *span,
                    node: node.clone(),
                })
            }
            Node::Atom { span, .. } => Some(Self {
                head: None,
                span: *span,
                node: node.clone(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UnknownField {
    pub key: String,
    pub value: Value,
}

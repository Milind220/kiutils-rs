use thiserror::Error;

/// Byte-span in the original input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

/// Atom value in an S-expression node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Atom {
    /// Unquoted token.
    Symbol(String),
    /// Quoted token with escape sequences unescaped.
    Quoted(String),
}

/// CST node (list or atom) with source span.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node {
    /// Parenthesized list.
    List { items: Vec<Node>, span: Span },
    /// Atomic token.
    Atom { atom: Atom, span: Span },
}

/// Parsed CST document plus the original source buffer.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CstDocument {
    /// Original input bytes interpreted as UTF-8 text.
    pub raw: String,
    /// Top-level parsed nodes.
    pub nodes: Vec<Node>,
}

impl CstDocument {
    /// Returns the original input unchanged.
    pub fn to_lossless_string(&self) -> &str {
        &self.raw
    }

    /// Returns a normalized, pretty-minimal representation.
    pub fn to_canonical_string(&self) -> String {
        let mut out = String::with_capacity(self.raw.len());
        for (idx, node) in self.nodes.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            fmt_node(node, &mut out);
        }
        out.push('\n');
        out
    }
}

fn fmt_node(node: &Node, out: &mut String) {
    match node {
        Node::List { items, .. } => {
            out.push('(');
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(' ');
                }
                fmt_node(item, out);
            }
            out.push(')');
        }
        Node::Atom { atom, .. } => match atom {
            Atom::Symbol(s) => out.push_str(s),
            Atom::Quoted(s) => {
                out.push('"');
                for ch in s.chars() {
                    if ch == '"' || ch == '\\' {
                        out.push('\\');
                    }
                    out.push(ch);
                }
                out.push('"');
            }
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// Exactly one top-level form required.
    SingleRoot,
    /// Zero or more top-level forms allowed.
    RootlessMany,
}

/// Parse failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    /// Input ended before a token/list completed.
    #[error("unexpected eof")]
    UnexpectedEof,
    /// Unexpected token at byte offset.
    #[error("unexpected token at byte {0}")]
    UnexpectedToken(usize),
    /// `SingleRoot` mode received zero or multiple roots.
    #[error("expected single root, got {0}")]
    ExpectedSingleRoot(usize),
    /// Input exceeded the parser nesting guard.
    #[error("maximum nesting depth exceeded at byte {0}")]
    MaxNestingExceeded(usize),
}

const MAX_NESTING_DEPTH: usize = 2048;

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            s: src.as_bytes(),
            i: 0,
        }
    }

    fn bump_ws(&mut self) {
        while self.i < self.s.len() {
            let b = self.s[self.i];
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn parse_node(&mut self) -> Result<Node, ParseError> {
        self.parse_node_with_depth(0)
    }

    fn parse_node_with_depth(&mut self, depth: usize) -> Result<Node, ParseError> {
        self.bump_ws();
        if self.i >= self.s.len() {
            return Err(ParseError::UnexpectedEof);
        }
        match self.s[self.i] {
            b'(' => self.parse_list(depth),
            b')' => Err(ParseError::UnexpectedToken(self.i)),
            b'"' => self.parse_quoted(),
            _ => self.parse_symbol(),
        }
    }

    fn parse_list(&mut self, depth: usize) -> Result<Node, ParseError> {
        if depth >= MAX_NESTING_DEPTH {
            return Err(ParseError::MaxNestingExceeded(self.i));
        }
        let start = self.i;
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.bump_ws();
            if self.i >= self.s.len() {
                return Err(ParseError::UnexpectedEof);
            }
            if self.s[self.i] == b')' {
                self.i += 1;
                return Ok(Node::List {
                    items,
                    span: Span { start, end: self.i },
                });
            }
            items.push(self.parse_node_with_depth(depth + 1)?);
        }
    }

    fn parse_quoted(&mut self) -> Result<Node, ParseError> {
        let start = self.i;
        self.i += 1;
        let mut out = Vec::<u8>::new();
        while self.i < self.s.len() {
            let b = self.s[self.i];
            self.i += 1;
            match b {
                b'\\' => {
                    if self.i >= self.s.len() {
                        return Err(ParseError::UnexpectedEof);
                    }
                    let next = self.s[self.i];
                    self.i += 1;
                    out.push(next);
                }
                b'"' => {
                    let text =
                        String::from_utf8(out).map_err(|_| ParseError::UnexpectedToken(start))?;
                    return Ok(Node::Atom {
                        atom: Atom::Quoted(text),
                        span: Span { start, end: self.i },
                    });
                }
                _ => out.push(b),
            }
        }
        Err(ParseError::UnexpectedEof)
    }

    fn parse_symbol(&mut self) -> Result<Node, ParseError> {
        let start = self.i;
        while self.i < self.s.len() {
            let b = self.s[self.i];
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')') {
                break;
            }
            self.i += 1;
        }
        if start == self.i {
            return Err(ParseError::UnexpectedToken(self.i));
        }
        let sym = String::from_utf8(self.s[start..self.i].to_vec())
            .map_err(|_| ParseError::UnexpectedToken(start))?;
        Ok(Node::Atom {
            atom: Atom::Symbol(sym),
            span: Span { start, end: self.i },
        })
    }
}

/// Parses rootless/multi-form S-expression input.
pub fn parse_rootless(input: &str) -> Result<CstDocument, ParseError> {
    parse_with_mode(input, ParseMode::RootlessMany)
}

/// Parses exactly one top-level S-expression root.
pub fn parse_one(input: &str) -> Result<CstDocument, ParseError> {
    parse_with_mode(input, ParseMode::SingleRoot)
}

fn parse_with_mode(input: &str, mode: ParseMode) -> Result<CstDocument, ParseError> {
    let mut p = P::new(input);
    let mut nodes = Vec::new();

    loop {
        p.bump_ws();
        if p.i >= p.s.len() {
            break;
        }
        nodes.push(p.parse_node()?);
    }

    if matches!(mode, ParseMode::SingleRoot) && nodes.len() != 1 {
        return Err(ParseError::ExpectedSingleRoot(nodes.len()));
    }

    Ok(CstDocument {
        raw: input.to_string(),
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_root() {
        let doc = parse_one("(kicad_pcb (version 20260101))").expect("parse");
        assert_eq!(doc.nodes.len(), 1);
        assert_eq!(doc.to_lossless_string(), "(kicad_pcb (version 20260101))");
    }

    #[test]
    fn parse_rootless_dru_like() {
        let doc = parse_rootless("(version 1)\n(rule \"x\" (condition \"A\"))\n").expect("parse");
        assert_eq!(doc.nodes.len(), 2);
    }

    #[test]
    fn single_root_rejects_many() {
        let err = parse_one("(a)(b)").expect_err("must fail");
        assert_eq!(err, ParseError::ExpectedSingleRoot(2));
    }

    #[test]
    fn canonical_prints_normalized() {
        let doc = parse_one("(kicad_pcb   (version 20260101)   )").expect("parse");
        assert_eq!(
            doc.to_canonical_string(),
            "(kicad_pcb (version 20260101))\n"
        );
    }

    #[test]
    fn parse_deeply_nested_input_hits_depth_limit() {
        let depth = MAX_NESTING_DEPTH + 16;
        let mut src = String::with_capacity(depth * 2 + 1);
        for _ in 0..depth {
            src.push('(');
        }
        src.push('x');
        for _ in 0..depth {
            src.push(')');
        }

        let err = parse_one(&src).expect_err("must reject excessive nesting");
        assert!(matches!(err, ParseError::MaxNestingExceeded(_)));
    }

    #[test]
    fn parse_quoted_preserves_utf8_content() {
        let doc = parse_one("(x \"café 日本語 🚀\")").expect("parse");
        let Node::List { items, .. } = &doc.nodes[0] else {
            panic!("expected list");
        };
        let Node::Atom {
            atom: Atom::Quoted(s),
            ..
        } = &items[1]
        else {
            panic!("expected quoted atom");
        };
        assert_eq!(s, "café 日本語 🚀");
    }

    #[test]
    fn parse_quoted_preserves_utf8_with_escaped_quote() {
        let doc = parse_one("(x \"café \\\"日本語\\\"\")").expect("parse");
        let Node::List { items, .. } = &doc.nodes[0] else {
            panic!("expected list");
        };
        let Node::Atom {
            atom: Atom::Quoted(s),
            ..
        } = &items[1]
        else {
            panic!("expected quoted atom");
        };
        assert_eq!(s, "café \"日本語\"");
    }
}

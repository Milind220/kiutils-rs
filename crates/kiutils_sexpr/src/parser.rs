use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    Symbol(String),
    Quoted(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    List { items: Vec<Node>, span: Span },
    Atom { atom: Atom, span: Span },
}

#[derive(Debug, Clone)]
pub struct CstDocument {
    pub raw: String,
    pub nodes: Vec<Node>,
}

impl CstDocument {
    pub fn to_lossless_string(&self) -> &str {
        &self.raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    SingleRoot,
    RootlessMany,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("unexpected eof")]
    UnexpectedEof,
    #[error("unexpected token at byte {0}")]
    UnexpectedToken(usize),
    #[error("expected single root, got {0}")]
    ExpectedSingleRoot(usize),
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn new(src: &'a str) -> Self {
        Self { s: src.as_bytes(), i: 0 }
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
        self.bump_ws();
        if self.i >= self.s.len() {
            return Err(ParseError::UnexpectedEof);
        }
        match self.s[self.i] {
            b'(' => self.parse_list(),
            b')' => Err(ParseError::UnexpectedToken(self.i)),
            b'"' => self.parse_quoted(),
            _ => self.parse_symbol(),
        }
    }

    fn parse_list(&mut self) -> Result<Node, ParseError> {
        let start = self.i;
        self.i += 1; // '('
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
                    span: Span {
                        start,
                        end: self.i,
                    },
                });
            }
            items.push(self.parse_node()?);
        }
    }

    fn parse_quoted(&mut self) -> Result<Node, ParseError> {
        let start = self.i;
        self.i += 1; // opening quote
        let mut out = String::new();
        while self.i < self.s.len() {
            let b = self.s[self.i];
            self.i += 1;
            match b {
                b'\\' => {
                    if self.i >= self.s.len() {
                        return Err(ParseError::UnexpectedEof);
                    }
                    let next = self.s[self.i] as char;
                    self.i += 1;
                    out.push(next);
                }
                b'"' => {
                    return Ok(Node::Atom {
                        atom: Atom::Quoted(out),
                        span: Span {
                            start,
                            end: self.i,
                        },
                    });
                }
                _ => out.push(b as char),
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
            span: Span {
                start,
                end: self.i,
            },
        })
    }
}

pub fn parse_rootless(input: &str) -> Result<CstDocument, ParseError> {
    parse_with_mode(input, ParseMode::RootlessMany)
}

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
}

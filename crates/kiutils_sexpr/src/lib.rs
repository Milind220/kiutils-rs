#![warn(missing_docs)]
//! # kiutils-sexpr
//!
//! Lossless S-expression CST parser/printer used by `kiutils-kicad`.
//!
//! ## Design goals
//! - Preserve original bytes for exact lossless writes (`CstDocument::to_lossless_string`)
//! - Offer deterministic normalized output (`CstDocument::to_canonical_string`)
//! - Keep explicit byte spans for diagnostics and precise edits
//! - Support both single-root and rootless multi-form inputs
//!
//! ## Quickstart
//! ```rust
//! use kiutils_sexpr::{parse_one, Node};
//!
//! let doc = parse_one("(kicad_pcb (version 20260101))").unwrap();
//! let Node::List { items, .. } = &doc.nodes[0] else { unreachable!() };
//! assert_eq!(items.len(), 2);
//! assert_eq!(doc.to_lossless_string(), "(kicad_pcb (version 20260101))");
//! ```
//!
//! Rootless mode for multi-form files (for example, `.kicad_dru` style snippets):
//! ```rust
//! use kiutils_sexpr::parse_rootless;
//!
//! let doc = parse_rootless("(version 1)\n(rule \"x\")\n").unwrap();
//! assert_eq!(doc.nodes.len(), 2);
//! ```

mod parser;

/// Parse and CST data model.
pub use parser::{parse_one, parse_rootless, Atom, CstDocument, Node, ParseError, ParseMode, Span};

# Introduction

`kiutils-rs` is a Rust workspace for **lossless KiCad file parsing/editing**.

## Workspace layers

| Crate | Role |
| --- | --- |
| `kiutils_sexpr` | Lossless S-expression CST parser/printer |
| `kiutils_kicad` | Typed KiCad document layer |
| `kiutils-rs` (`kiutils_rs`) | Stable public API for applications |

## Design goals

- Lossless by default (`WriteMode::Lossless`)
- Canonical output when requested (`WriteMode::Canonical`)
- Forward-compatible unknown token capture (`UnknownNode`, `UnknownField`)
- Typed mutation helpers for common edit flows

## Version scope

- Primary target: KiCad v10
- Secondary target: KiCad v9

## Install

```bash
cargo add kiutils-rs
```

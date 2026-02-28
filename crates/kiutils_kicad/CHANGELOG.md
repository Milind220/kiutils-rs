# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/Milind220/kiutils-rs/compare/kiutils_kicad-v0.1.0...kiutils_kicad-v0.1.1) - 2026-02-28

### Fixed

- *(publish)* restore published crate IDs

### Other

- add mdBook guide site and Pages deploy workflow ([#14](https://github.com/Milind220/kiutils-rs/pull/14))
- Merge pull request #13 from Milind220/codex/cargo-discovery-metadata

## [0.1.0](https://github.com/Milind220/kiutils-rs/releases/tag/kiutils_kicad-v0.1.0) - 2026-02-26

### Added

- *(kicad)* add sym-lib-table and worksheet roundtrip support
- *(dru)* add full design-rules roundtrip editing API
- *(kicad)* add schematic roundtrip support and shared section helpers
- *(kicad)* add symbol roundtrip support and shared edit helpers
- *(pcb)* add chainable read-modify-write API
- *(footprint)* add chainable read-modify-write API
- *(kicad)* improve pcb/footprint compatibility parsing
- expose pcb general and title metadata in cli
- parse pcb general paper and title block summaries
- show enriched pcb metadata in inspect cli
- enrich pcb ast with footprint and route metadata
- expose fp-lib-table version in cli
- parse fp-lib-table version token
- expose footprint clearance in cli
- parse footprint clearance token
- expose footprint embedded files in cli
- parse footprint embedded files summary
- expose footprint solder margin fields in cli
- parse footprint solder margin settings
- expose footprint lock metadata in inspect cli
- enrich footprint inspect output
- expand footprint ast parsing with regression coverage
- add pcb graphic summaries
- add pcb dimension target and group summaries
- parse pcb setup and property summaries
- parse pcb generated blocks and richer zone summaries
- parse pcb trace and zone summaries
- add pcb graphic token breakdown counters
- parse pcb footprint summaries in board ast
- parse pcb layers and nets into typed ast
- parse pcb top-level tokens and counts
- add kiutils-inspect CLI for parser output inspection
- add serde and parallel feature support
- capture unknown nodes and fields across typed AST
- add canonical write mode across sexpr and kicad docs
- add kicad file readers and version policy checks
- bootstrap two-crate workspace and sync pcb reader

### Fixed

- *(pcb)* report nested unknown tokens in known sections
- *(project)* validate meta_version i32 range
- *(pcb)* avoid fake text on non-text gr_* graphics
- *(kicad)* preserve unknown node tails on targeted setters
- *(kicad)* reject silent ast_mut writes
- *(kicad)* avoid dirty writes for no-op setters
- *(kicad)* avoid full canonical rewrite on single-node edits
- *(kicad)* resolve cursor bugbot review findings
- parse pcb title block comment text values
- parse footprint locked token and regression test
- parse pcb dimension type child token (regression)
- parse pcb embedded_files token (regression test)

### Other

- apply rustfmt cleanup
- *(cli)* unify inspect output field emitters
- *(kicad)* deduplicate s-expression helpers in parsers
- add typed API module docs and end-to-end example
- add fixture-based integration coverage for all file types

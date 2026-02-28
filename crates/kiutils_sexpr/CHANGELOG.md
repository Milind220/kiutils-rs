# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/Milind220/kiutils-rs/compare/kiutils_sexpr-v0.1.0...kiutils_sexpr-v0.1.1) - 2026-02-28

### Fixed

- *(publish)* restore published crate IDs
- *(sexpr)* document public parse mode and node fields

### Other

- Merge pull request #13 from Milind220/codex/cargo-discovery-metadata

## [0.1.0](https://github.com/Milind220/kiutils-rs/releases/tag/kiutils_sexpr-v0.1.0) - 2026-02-26

### Added

- add serde and parallel feature support
- add canonical write mode across sexpr and kicad docs
- bootstrap two-crate workspace and sync pcb reader

### Fixed

- *(sexpr)* enforce parser nesting depth limit
- *(sexpr)* preserve utf-8 in quoted string parsing

### Other

- apply rustfmt cleanup

use std::fs;
use std::path::{Path, PathBuf};

use kiutils_kicad::{SchematicFile, SymbolLibFile};
use kiutils_sexpr::{Atom, Node};

fn demos_root() -> Option<PathBuf> {
    let path = PathBuf::from("/Users/milindsharma/Engineering/demos");
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

fn collect_files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))? {
        let entry = entry.map_err(|e| format!("read_dir entry {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, ext, out)?;
            continue;
        }
        if path.extension().and_then(|v| v.to_str()) == Some(ext) {
            out.push(path);
        }
    }
    Ok(())
}

fn top_level_count(doc: &kiutils_kicad::SchematicDocument) -> usize {
    let Some(Node::List { items, .. }) = doc.cst().nodes.first() else {
        return 0;
    };
    items
        .iter()
        .skip(1)
        .filter(|n| {
            matches!(
                n,
                Node::List {
                    items,
                    ..
                } if matches!(
                    items.first(),
                    Some(Node::Atom {
                        atom: Atom::Symbol(_),
                        ..
                    })
                )
            )
        })
        .count()
}

#[test]
fn schematic_real_file_exposes_component_details() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("generated")
        .join("schematics")
        .join("simulation")
        .join("subsheets")
        .join("mainsheet.kicad_sch");
    let doc = SchematicFile::read(&path).expect("parse mainsheet");

    assert!(!doc.ast().top_level_nodes.is_empty());
    assert_eq!(doc.ast().top_level_nodes.len(), top_level_count(&doc));

    let comps = doc.ast().components(false);
    assert!(comps.iter().any(|c| c.reference == "R2"));

    let r2 = comps
        .iter()
        .find(|c| c.reference == "R2")
        .expect("R2 component");
    assert_eq!(r2.lib_id.as_deref(), Some("Device:R"));
    assert_eq!(r2.value.as_deref(), Some("1K"));
}

#[test]
fn schematic_demos_corpus_has_full_top_level_coverage() {
    let Some(root) = demos_root() else {
        return;
    };

    let mut files = Vec::new();
    collect_files(&root, "kicad_sch", &mut files).expect("collect demos schematics");
    files.sort();
    assert!(!files.is_empty(), "expected .kicad_sch files in demos");

    for path in files {
        let doc = SchematicFile::read(&path).expect("parse schematic");
        assert_eq!(
            doc.ast().top_level_nodes.len(),
            top_level_count(&doc),
            "top-level node mismatch: {}",
            path.display()
        );
        assert!(
            doc.ast().unknown_nodes.is_empty(),
            "unknown top-level nodes found in {}",
            path.display()
        );
        for node in &doc.ast().top_level_nodes {
            assert!(!node.head.is_empty(), "missing head in {}", path.display());
        }
    }
}

#[test]
fn symbol_real_file_exposes_detailed_symbol_definitions() {
    let path = Path::new("/Users/milindsharma/Engineering/demos/video/libs/video_schlib.kicad_sym");
    if !path.exists() {
        return;
    }
    let doc = SymbolLibFile::read(path).expect("parse symbol lib");

    assert!(!doc.ast().symbol_definitions.is_empty());
    let first = &doc.ast().symbol_definitions[0];
    assert!(first.name.is_some());
    assert!(!first.child_heads.is_empty());
}

#[test]
fn symbol_demos_corpus_has_full_symbol_coverage() {
    let Some(root) = demos_root() else {
        return;
    };

    let mut files = Vec::new();
    collect_files(&root, "kicad_sym", &mut files).expect("collect demos symbols");
    files.sort();
    assert!(!files.is_empty(), "expected .kicad_sym files in demos");

    for path in files {
        let doc = SymbolLibFile::read(&path).expect("parse symbol");
        assert!(
            doc.ast().unknown_nodes.is_empty(),
            "unknown top-level nodes found in {}",
            path.display()
        );
        assert_eq!(
            doc.ast().symbol_count,
            doc.ast().symbol_definitions.len(),
            "symbol definition mismatch: {}",
            path.display()
        );
        let total_props: usize = doc
            .ast()
            .symbol_definitions
            .iter()
            .map(|s| s.properties.len())
            .sum();
        let total_pins: usize = doc
            .ast()
            .symbol_definitions
            .iter()
            .map(|s| s.pins.len())
            .sum();
        assert_eq!(
            total_props,
            doc.ast().total_property_count,
            "property count mismatch: {}",
            path.display()
        );
        assert_eq!(
            total_pins,
            doc.ast().total_pin_count,
            "pin count mismatch: {}",
            path.display()
        );
    }
}

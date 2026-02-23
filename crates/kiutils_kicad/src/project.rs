use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::{Error, UnknownField, WriteMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProjectAst {
    pub meta_version: Option<i32>,
    pub pinned_footprint_libs: Vec<String>,
    pub unknown_fields: Vec<UnknownField>,
}

#[derive(Debug, Clone)]
pub struct ProjectDocument {
    ast: ProjectAst,
    raw: String,
    json: Value,
}

impl ProjectDocument {
    pub fn ast(&self) -> &ProjectAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut ProjectAst {
        &mut self.ast
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn json(&self) -> &Value {
        &self.json
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        self.write_mode(path, WriteMode::Lossless)
    }

    pub fn write_mode<P: AsRef<Path>>(&self, path: P, mode: WriteMode) -> Result<(), Error> {
        match mode {
            WriteMode::Lossless => fs::write(path, &self.raw)?,
            WriteMode::Canonical => {
                let json = serde_json::to_string_pretty(&self.json)
                    .map_err(|e| Error::Validation(format!("json serialization failed: {e}")))?;
                fs::write(path, format!("{json}\n"))?;
            }
        }
        Ok(())
    }
}

pub struct ProjectFile;

impl ProjectFile {
    pub fn read<P: AsRef<Path>>(path: P) -> Result<ProjectDocument, Error> {
        let raw = fs::read_to_string(path)?;
        let json: Value = serde_json::from_str(&raw)
            .map_err(|e| Error::Validation(format!("invalid .kicad_pro json: {e}")))?;

        let meta_version = json
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|m| m.get("version"))
            .and_then(Value::as_i64)
            .map(|v| v as i32);

        let pinned_footprint_libs = json
            .get("libraries")
            .and_then(Value::as_object)
            .and_then(|l| l.get("pinned_footprint_libs"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let known_top_level = ["meta", "libraries", "board", "sheets", "boards", "text_variables"];
        let unknown_fields = json
            .as_object()
            .map(|o| {
                o.iter()
                    .filter(|(k, _)| !known_top_level.contains(&k.as_str()))
                    .map(|(k, v)| UnknownField {
                        key: k.clone(),
                        value: v.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(ProjectDocument {
            ast: ProjectAst {
                meta_version,
                pinned_footprint_libs,
                unknown_fields,
            },
            raw,
            json,
        })
    }
}

pub type ProjectExtra = BTreeMap<String, Value>;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_pro"))
    }

    #[test]
    fn read_project_json() {
        let path = tmp_file("pro_ok");
        let src = r#"{
  "meta": { "version": 3 },
  "libraries": { "pinned_footprint_libs": ["A", "B"] },
  "board": { "foo": true }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let doc = ProjectFile::read(&path).expect("read");
        assert_eq!(doc.ast().meta_version, Some(3));
        assert_eq!(doc.ast().pinned_footprint_libs, vec!["A", "B"]);
        assert!(doc.ast().unknown_fields.is_empty());
        assert_eq!(doc.raw(), src);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_project_captures_unknown_top_level_fields() {
        let path = tmp_file("pro_unknown");
        let src = r#"{
  "meta": { "version": 3 },
  "libraries": { "pinned_footprint_libs": ["A"] },
  "custom_top": { "x": 1 }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let doc = ProjectFile::read(&path).expect("read");
        assert_eq!(doc.ast().unknown_fields.len(), 1);
        assert_eq!(doc.ast().unknown_fields[0].key, "custom_top");

        let _ = fs::remove_file(path);
    }
}

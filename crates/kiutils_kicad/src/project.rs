use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Map;
use serde_json::Value;

use crate::{Error, UnknownField, WriteMode};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProjectAst {
    pub meta_version: Option<i32>,
    pub pinned_symbol_libs: Vec<String>,
    pub pinned_footprint_libs: Vec<String>,
    pub unknown_fields: Vec<UnknownField>,
}

#[derive(Debug, Clone)]
pub struct ProjectDocument {
    ast: ProjectAst,
    raw: String,
    json: Value,
    ast_dirty: bool,
}

impl ProjectDocument {
    pub fn ast(&self) -> &ProjectAst {
        &self.ast
    }

    pub fn ast_mut(&mut self) -> &mut ProjectAst {
        self.ast_dirty = true;
        &mut self.ast
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn json(&self) -> &Value {
        &self.json
    }

    pub fn set_pinned_symbol_libs<I, S>(&mut self, libs: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let was_ast_dirty = self.ast_dirty;
        let libs = libs.into_iter().map(Into::into).collect::<Vec<_>>();
        self.set_library_array("pinned_symbol_libs", &libs);
        self.refresh_ast_from_json();
        self.ast_dirty = was_ast_dirty;
        self
    }

    pub fn set_pinned_footprint_libs<I, S>(&mut self, libs: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let was_ast_dirty = self.ast_dirty;
        let libs = libs.into_iter().map(Into::into).collect::<Vec<_>>();
        self.set_library_array("pinned_footprint_libs", &libs);
        self.refresh_ast_from_json();
        self.ast_dirty = was_ast_dirty;
        self
    }

    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        self.write_mode(path, WriteMode::Lossless)
    }

    pub fn write_mode<P: AsRef<Path>>(&self, path: P, mode: WriteMode) -> Result<(), Error> {
        if self.ast_dirty {
            return Err(Error::Validation(
                "ast_mut changes are not serializable; use document setter APIs".to_string(),
            ));
        }
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

    fn set_library_array(&mut self, key: &str, libs: &[String]) {
        if !self.json.is_object() {
            self.json = Value::Object(Map::new());
        }
        if let Some(root) = self.json.as_object_mut() {
            let libraries = root
                .entry("libraries".to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if !libraries.is_object() {
                *libraries = Value::Object(Map::new());
            }
            if let Some(libraries) = libraries.as_object_mut() {
                libraries.insert(
                    key.to_string(),
                    Value::Array(libs.iter().cloned().map(Value::String).collect()),
                );
            }
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.json) {
            self.raw = format!("{json}\n");
        }
    }

    fn refresh_ast_from_json(&mut self) {
        let meta_version = self
            .json
            .get("meta")
            .and_then(Value::as_object)
            .and_then(|m| m.get("version"))
            .and_then(Value::as_i64)
            .and_then(|v| i32::try_from(v).ok());

        let pinned_footprint_libs = self
            .json
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

        let pinned_symbol_libs = self
            .json
            .get("libraries")
            .and_then(Value::as_object)
            .and_then(|l| l.get("pinned_symbol_libs"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let known_top_level = [
            "meta",
            "libraries",
            "board",
            "sheets",
            "boards",
            "text_variables",
        ];
        let unknown_fields = self
            .json
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

        self.ast = ProjectAst {
            meta_version,
            pinned_symbol_libs,
            pinned_footprint_libs,
            unknown_fields,
        };
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
            .map(i32::try_from)
            .transpose()
            .map_err(|_| Error::Validation("meta.version is out of i32 range".to_string()))?;

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

        let pinned_symbol_libs = json
            .get("libraries")
            .and_then(Value::as_object)
            .and_then(|l| l.get("pinned_symbol_libs"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let known_top_level = [
            "meta",
            "libraries",
            "board",
            "sheets",
            "boards",
            "text_variables",
        ];
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
                pinned_symbol_libs,
                pinned_footprint_libs,
                unknown_fields,
            },
            raw,
            json,
            ast_dirty: false,
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
  "libraries": {
    "pinned_symbol_libs": ["S1", "S2"],
    "pinned_footprint_libs": ["A", "B"]
  },
  "board": { "foo": true }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let doc = ProjectFile::read(&path).expect("read");
        assert_eq!(doc.ast().meta_version, Some(3));
        assert_eq!(doc.ast().pinned_symbol_libs, vec!["S1", "S2"]);
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

    #[test]
    fn setters_update_project_libraries_and_allow_write() {
        let path = tmp_file("pro_setters");
        let src = r#"{
  "meta": { "version": 3 },
  "libraries": { "pinned_footprint_libs": ["A"] }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let mut doc = ProjectFile::read(&path).expect("read");
        doc.set_pinned_symbol_libs(vec!["SYM_A", "SYM_B"])
            .set_pinned_footprint_libs(vec!["FP_A", "FP_B"]);
        assert_eq!(doc.ast().pinned_symbol_libs, vec!["SYM_A", "SYM_B"]);
        assert_eq!(doc.ast().pinned_footprint_libs, vec!["FP_A", "FP_B"]);
        assert_eq!(
            doc.json()
                .get("libraries")
                .and_then(Value::as_object)
                .and_then(|l| l.get("pinned_symbol_libs"))
                .and_then(Value::as_array)
                .map(|x| x.len()),
            Some(2)
        );

        let out = tmp_file("pro_setters_out");
        doc.write(&out).expect("write should work");
        let reread = ProjectFile::read(&out).expect("reread");
        assert_eq!(reread.ast().pinned_symbol_libs, vec!["SYM_A", "SYM_B"]);
        assert_eq!(reread.ast().pinned_footprint_libs, vec!["FP_A", "FP_B"]);

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn setters_do_not_clear_ast_mut_dirty_guard() {
        let path = tmp_file("pro_setter_does_not_clear_dirty");
        let src = r#"{
  "meta": { "version": 3 },
  "libraries": { "pinned_footprint_libs": ["A"] }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let mut doc = ProjectFile::read(&path).expect("read");
        doc.ast_mut().meta_version = Some(4);
        doc.set_pinned_symbol_libs(vec!["SYM_A"]);
        assert_eq!(doc.ast().meta_version, Some(3));

        let out = tmp_file("pro_setter_does_not_clear_dirty_out");
        let err = doc.write(&out).expect_err("write should fail");
        match err {
            Error::Validation(msg) => {
                assert!(msg.contains("ast_mut changes are not serializable"));
            }
            _ => panic!("expected validation error"),
        }

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn ast_mut_write_returns_validation_error() {
        let path = tmp_file("pro_ast_mut_write_error");
        let src = r#"{
  "meta": { "version": 3 },
  "libraries": { "pinned_footprint_libs": ["A"] }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let mut doc = ProjectFile::read(&path).expect("read");
        doc.ast_mut().meta_version = Some(4);

        let out = tmp_file("pro_ast_mut_write_error_out");
        let err = doc.write(&out).expect_err("write should fail");
        match err {
            Error::Validation(msg) => {
                assert!(msg.contains("ast_mut changes are not serializable"));
            }
            _ => panic!("expected validation error"),
        }

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(out);
    }

    #[test]
    fn read_project_rejects_out_of_range_meta_version() {
        let path = tmp_file("pro_meta_version_oob");
        let src = r#"{
  "meta": { "version": 9223372036854775807 },
  "libraries": { "pinned_footprint_libs": ["A"] }
}
"#;
        fs::write(&path, src).expect("write fixture");

        let err = ProjectFile::read(&path).expect_err("read should fail");
        match err {
            Error::Validation(msg) => assert!(msg.contains("meta.version is out of i32 range")),
            _ => panic!("expected validation error"),
        }

        let _ = fs::remove_file(path);
    }
}

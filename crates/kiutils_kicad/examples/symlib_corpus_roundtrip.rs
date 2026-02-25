use std::fs;
use std::path::{Path, PathBuf};

use kiutils_kicad::SymLibTableFile;

fn usage() -> String {
    "usage: symlib_corpus_roundtrip <input_dir> <output_dir>".to_string()
}

fn collect_symlib_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_symlib_files(&path, files)?;
        } else if path.file_name().and_then(|s| s.to_str()) == Some("sym-lib-table") {
            files.push(path);
        }
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let input_dir = args.next().map(PathBuf::from).ok_or_else(usage)?;
    let output_dir = args.next().map(PathBuf::from).ok_or_else(usage)?;

    let mut files = Vec::new();
    collect_symlib_files(&input_dir, &mut files)?;
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "no sym-lib-table files found under {}",
            input_dir.display()
        ));
    }

    let mut ok = 0usize;
    let mut failed = 0usize;
    for path in files {
        let rel = path
            .strip_prefix(&input_dir)
            .map_err(|e| format!("strip_prefix {}: {e}", path.display()))?;
        let out_path = output_dir.join(rel);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all {}: {e}", parent.display()))?;
        }

        let result = (|| -> Result<(), String> {
            let doc = SymLibTableFile::read(&path).map_err(|e| format!("read: {e}"))?;
            doc.write(&out_path).map_err(|e| format!("write: {e}"))?;
            let _ = SymLibTableFile::read(&out_path).map_err(|e| format!("reread: {e}"))?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                ok += 1;
                println!("ok: {}", path.display());
            }
            Err(err) => {
                failed += 1;
                eprintln!("fail: {} -> {}", path.display(), err);
            }
        }
    }

    println!("summary: ok={ok} failed={failed}");
    if failed > 0 {
        return Err(format!("{failed} files failed"));
    }
    Ok(())
}

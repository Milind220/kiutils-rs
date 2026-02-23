use std::path::{Path, PathBuf};

use crate::{Error, PcbDocument, PcbFile};

pub fn read_pcbs(paths: &[PathBuf]) -> Vec<Result<PcbDocument, Error>> {
    read_pcbs_impl(paths)
}

#[cfg(feature = "parallel")]
fn read_pcbs_impl(paths: &[PathBuf]) -> Vec<Result<PcbDocument, Error>> {
    use rayon::prelude::*;
    paths.par_iter().map(PcbFile::read).collect()
}

#[cfg(not(feature = "parallel"))]
fn read_pcbs_impl(paths: &[PathBuf]) -> Vec<Result<PcbDocument, Error>> {
    paths.iter().map(PcbFile::read).collect()
}

pub fn read_pcbs_from_refs<P: AsRef<Path>>(paths: &[P]) -> Vec<Result<PcbDocument, Error>> {
    let owned = paths
        .iter()
        .map(|p| p.as_ref().to_path_buf())
        .collect::<Vec<_>>();
    read_pcbs(&owned)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn tmp_file(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}_{nanos}.kicad_pcb"))
    }

    #[test]
    fn batch_reads_multiple_pcbs() {
        let p1 = tmp_file("batch1");
        let p2 = tmp_file("batch2");
        fs::write(&p1, "(kicad_pcb (version 20260101))\n").expect("write p1");
        fs::write(&p2, "(kicad_pcb (version 20260101))\n").expect("write p2");

        let results = read_pcbs(&[p1.clone(), p2.clone()]);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.is_ok()));

        let _ = fs::remove_file(p1);
        let _ = fs::remove_file(p2);
    }
}

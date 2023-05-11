use eyre::Result;
use std::path::{Path, PathBuf};

pub fn find_data_files(base_path: &Path) -> Result<PathBuf> {
    let found = recurse_dirs(base_path.to_path_buf(), 0)?;
    found.ok_or_else(|| {
        eyre::eyre!(
            "No UMLS RRF files found in or under {}",
            base_path.display()
        )
    })
}

fn recurse_dirs(base_dir: PathBuf, current_depth: usize) -> Result<Option<PathBuf>> {
    if contains_rrf_files(&base_dir)? {
        return Ok(Some(base_dir));
    }

    if current_depth >= 2 {
        return Ok(None);
    }

    let contents = std::fs::read_dir(&base_dir)?;
    for entry in contents.flatten() {
        if entry.metadata()?.is_dir() {
            if let Some(path) = recurse_dirs(entry.path(), current_depth + 1)? {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

fn contains_rrf_files(path: &Path) -> Result<bool> {
    let contents = std::fs::read_dir(path)?;
    let found = contents
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy().ends_with("RRF.gz"));

    Ok(found)
}

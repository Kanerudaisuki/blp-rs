use std::path::{Path, PathBuf};

pub fn resolve_output_path(input: &Path, provided: Option<&PathBuf>, ext: &str) -> PathBuf {
    if let Some(p) = provided {
        return p.clone();
    }
    input.with_extension(ext)
}

use directories::ProjectDirs;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

fn conf_file() -> io::Result<PathBuf> {
    let proj = ProjectDirs::from("com.blp", "blp-rs", "blp-rs").ok_or_else(|| io::Error::new(io::ErrorKind::Other, "ProjectDirs not available"))?;
    let dir = proj.config_dir(); // .../blp-rs/
    fs::create_dir_all(dir)?;
    Ok(dir.join("last_dir.txt"))
}

pub fn load_last_dir() -> Option<PathBuf> {
    let path = conf_file().ok()?;
    let s = fs::read_to_string(path).ok()?;
    let p = PathBuf::from(s.trim());
    if p.is_dir() { Some(p) } else { None }
}

pub fn save_last_dir(dir: &Path) -> io::Result<()> {
    if let Ok(abs) = dir.canonicalize() {
        let file = conf_file()?;
        fs::write(file, abs.to_string_lossy().as_ref())?;
    }
    Ok(())
}

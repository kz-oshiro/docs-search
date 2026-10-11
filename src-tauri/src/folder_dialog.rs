use std::path::{Path, PathBuf};

pub(crate) fn initial_directory(input: Option<&str>, executable: Option<&Path>) -> Option<PathBuf> {
    input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(Path::new)
        .filter(|path| path.is_dir())
        .or_else(|| executable.and_then(Path::parent))
        .map(Path::to_path_buf)
}

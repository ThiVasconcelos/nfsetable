//! JSON files in the data folder: names that cannot escape it, atomic writes, and deletes that do
//! not mind a missing file. Callers add their own pt-BR messages.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// `<dir>/<name>.json` when `name` is a short lowercase slug (`[a-z0-9-]`, 1 to `max_len`
/// characters), so it can never point outside the folder.
pub fn slug_path(dir: &Path, name: &str, max_len: usize) -> Option<PathBuf> {
    let valid = !name.is_empty()
        && name.len() <= max_len
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    valid.then(|| dir.join(format!("{name}.json")))
}

/// Writes `contents` to `path` through a temporary file and a rename, so a crash never leaves a
/// half-written file behind. The folder must exist.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Deletes `path`; a file that does not exist is not an error.
pub fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_stay_in_the_folder() {
        let dir = Path::new("dados");
        assert_eq!(
            slug_path(dir, "notes-principal", 40),
            Some(dir.join("notes-principal.json"))
        );
        for bad in ["", "../x", "a/b", "Maiuscula", "com espaço", "ponto.json"] {
            assert_eq!(slug_path(dir, bad, 40), None, "{bad}");
        }
        assert_eq!(slug_path(dir, "abc", 2), None);
    }

    #[test]
    fn writes_atomically_and_removes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.json");
        write_atomic(&path, "{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"a\":1}");
        assert!(!dir.path().join("doc.json.tmp").exists());
        remove_if_exists(&path).unwrap();
        remove_if_exists(&path).unwrap();
        assert!(!path.exists());
    }
}

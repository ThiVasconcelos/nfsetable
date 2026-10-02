//! Small JSON documents the frontend keeps between sessions (e.g. the tax planning: projections,
//! costs and settings), one file per name in the app data folder.

use crate::files;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_NAME_LEN: usize = 40;
/// Guard against runaway writes; the planning document is a few kilobytes.
const MAX_BYTES: usize = 2 * 1024 * 1024;

/// Reads a stored document; `None` when it was never written. A corrupt file is reported, not
/// silently replaced, so the user does not lose what they typed.
pub fn read(dir: &Path, name: &str) -> Result<Option<serde_json::Value>, String> {
    let path = file_of(dir, name)?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Não foi possível ler os dados salvos: {e}")),
    };
    serde_json::from_str(&text).map(Some).map_err(|e| {
        format!(
            "Os dados salvos em {} estão corrompidos: {e}",
            path.display()
        )
    })
}

/// Writes a document atomically (temporary file, then rename).
pub fn write(dir: &Path, name: &str, value: &serde_json::Value) -> Result<(), String> {
    let path = file_of(dir, name)?;
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    if json.len() > MAX_BYTES {
        return Err("Os dados são grandes demais para salvar.".to_string());
    }
    fs::create_dir_all(dir).map_err(|e| format!("Não foi possível criar a pasta de dados: {e}"))?;
    files::write_atomic(&path, &json).map_err(|e| format!("Não foi possível salvar os dados: {e}"))
}

/// Deletes a stored document; deleting one that does not exist is not an error.
pub fn delete(dir: &Path, name: &str) -> Result<(), String> {
    let path = file_of(dir, name)?;
    files::remove_if_exists(&path).map_err(|e| format!("Não foi possível apagar os dados: {e}"))
}

/// Path of a document; names are short lowercase slugs so they can never escape the folder.
fn file_of(dir: &Path, name: &str) -> Result<PathBuf, String> {
    files::slug_path(dir, name, MAX_NAME_LEN).ok_or_else(|| "Nome de dados inválido.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn writes_reads_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("store");
        assert_eq!(read(&store, "planning").unwrap(), None);

        let value = json!({ "version": 1, "costs": [{ "name": "Contador", "cents": 25000 }] });
        write(&store, "planning", &value).unwrap();
        assert_eq!(read(&store, "planning").unwrap(), Some(value));

        write(&store, "planning", &json!({ "version": 2 })).unwrap();
        assert_eq!(
            read(&store, "planning").unwrap(),
            Some(json!({ "version": 2 }))
        );
        assert!(!store.join("planning.json.tmp").exists());
    }

    #[test]
    fn rejects_bad_names_and_huge_values() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["", "../x", "Planning", "a b", "x.json", &"a".repeat(41)] {
            assert!(read(dir.path(), name).is_err(), "{name}");
            assert!(write(dir.path(), name, &json!(1)).is_err(), "{name}");
        }
        let huge = json!("x".repeat(MAX_BYTES + 1));
        assert!(write(dir.path(), "planning", &huge).is_err());
    }

    #[test]
    fn deletes_documents() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "notes-acme", &json!({ "version": 1 })).unwrap();
        delete(dir.path(), "notes-acme").unwrap();
        assert_eq!(read(dir.path(), "notes-acme").unwrap(), None);
        delete(dir.path(), "notes-acme").unwrap();
        assert!(delete(dir.path(), "../x").is_err());
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("planning.json"), "{ not json").unwrap();
        assert!(read(dir.path(), "planning").is_err());
    }
}

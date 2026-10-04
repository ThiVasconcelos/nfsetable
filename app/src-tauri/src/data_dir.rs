//! Where the persistent data lives (the user's profiles and the stored documents such as the tax
//! planning): the app data folder by default, or a folder chosen by the user, e.g. inside a
//! synced drive for backup. The choice itself is kept in `config.json` in the default folder.

use crate::files;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "config.json";
/// Subfolders with the user's data, copied when the data folder changes.
const DATA_SUBDIRS: [&str; 2] = ["profiles", "store"];

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(default)]
    data_dir: Option<PathBuf>,
}

/// The data folder to use at startup, plus a pt-BR warning when the chosen one is unavailable
/// (e.g. a disconnected drive): then the default folder is used until it comes back.
pub fn resolve(default_dir: &Path) -> (PathBuf, Option<String>) {
    let config = match load(default_dir) {
        Ok(config) => config,
        Err(problem) => {
            return (
                default_dir.to_path_buf(),
                Some(format!(
                    "Não foi possível ler a escolha da pasta de dados ({CONFIG_FILE}: {problem}); \
                     usando a pasta padrão. Escolha a pasta de novo nas configurações."
                )),
            )
        }
    };
    match config.data_dir {
        None => (default_dir.to_path_buf(), None),
        Some(dir) if dir.is_dir() => (dir, None),
        Some(dir) => (
            default_dir.to_path_buf(),
            Some(format!(
                "A pasta de dados escolhida ({}) não está disponível; usando a pasta padrão até \
                 ela voltar.",
                dir.display()
            )),
        ),
    }
}

/// Switches the data folder (`None` = back to the default). With `copy`, the current profiles
/// and stored documents are copied into the new folder first (replacing files with the same
/// name); without it, whatever is already there is used. Returns the new folder.
pub fn switch(
    default_dir: &Path,
    current: &Path,
    target: Option<&Path>,
    copy: bool,
) -> Result<PathBuf, String> {
    let new_dir = target.unwrap_or(default_dir).to_path_buf();
    ensure_writable(&new_dir)?;
    if copy && !same_dir(&new_dir, current) {
        copy_data(current, &new_dir)?;
    }
    let chosen = target.filter(|dir| !same_dir(dir, default_dir));
    save(default_dir, chosen)?;
    Ok(new_dir)
}

/// The saved choice; a missing file means none, an unreadable or corrupt one is an error.
fn load(default_dir: &Path) -> Result<Config, String> {
    let json = match fs::read_to_string(default_dir.join(CONFIG_FILE)) {
        Ok(json) => json,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e.to_string()),
    };
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

fn save(default_dir: &Path, data_dir: Option<&Path>) -> Result<(), String> {
    let config = Config {
        data_dir: data_dir.map(Path::to_path_buf),
    };
    let json = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::create_dir_all(default_dir)
        .map_err(|e| format!("Não foi possível salvar a configuração: {e}"))?;
    files::write_atomic(&default_dir.join(CONFIG_FILE), &json)
        .map_err(|e| format!("Não foi possível salvar a configuração: {e}"))
}

/// Creates the folder if needed and checks that files can be written there.
fn ensure_writable(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|e| format!("Não foi possível usar a pasta {}: {e}", dir.display()))?;
    let probe = dir.join(".nfsetable-teste");
    fs::write(&probe, b"ok")
        .map_err(|e| format!("Não foi possível gravar na pasta {}: {e}", dir.display()))?;
    let _ = fs::remove_file(&probe);
    Ok(())
}

/// Copies `profiles/*.json` and `store/*.json` from `from` to `to`.
fn copy_data(from: &Path, to: &Path) -> Result<(), String> {
    for sub in DATA_SUBDIRS {
        let Ok(entries) = fs::read_dir(from.join(sub)) else {
            continue;
        };
        let target = to.join(sub);
        fs::create_dir_all(&target)
            .map_err(|e| format!("Não foi possível copiar os dados: {e}"))?;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") && path.is_file() {
                let name = entry.file_name();
                fs::copy(&path, target.join(&name))
                    .map_err(|e| format!("Não foi possível copiar {}: {e}", path.display()))?;
            }
        }
    }
    Ok(())
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn default_until_a_folder_is_chosen() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join("default");
        assert_eq!(resolve(&default), (default.clone(), None));
    }

    #[test]
    fn switch_with_copy_then_back_to_default() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join("default");
        let drive = root.path().join("drive").join("nfsetable");
        write(&default.join("store/planning.json"), r#"{"version":1}"#);
        write(&default.join("profiles/recibos-1.json"), "{}");
        write(&default.join("store/notes.txt"), "not copied");

        let new_dir = switch(&default, &default, Some(&drive), true).unwrap();
        assert_eq!(new_dir, drive);
        assert_eq!(
            fs::read_to_string(drive.join("store/planning.json")).unwrap(),
            r#"{"version":1}"#
        );
        assert!(drive.join("profiles/recibos-1.json").is_file());
        assert!(!drive.join("store/notes.txt").exists());
        assert!(!drive.join(".nfsetable-teste").exists());
        assert_eq!(resolve(&default), (drive.clone(), None));

        // Back to the default, using what is there.
        let back = switch(&default, &drive, None, false).unwrap();
        assert_eq!(back, default);
        assert_eq!(resolve(&default), (default.clone(), None));
    }

    #[test]
    fn switch_without_copy_keeps_the_folder_contents() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join("default");
        let other = root.path().join("other");
        write(&default.join("store/planning.json"), "1");
        write(&other.join("store/planning.json"), "2");
        switch(&default, &default, Some(&other), false).unwrap();
        assert_eq!(
            fs::read_to_string(other.join("store/planning.json")).unwrap(),
            "2"
        );
    }

    #[test]
    fn unavailable_folder_falls_back_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join("default");
        let gone = root.path().join("gone");
        switch(&default, &default, Some(&gone), false).unwrap();
        fs::remove_dir_all(&gone).unwrap();
        let (dir, warning) = resolve(&default);
        assert_eq!(dir, default);
        assert!(warning.unwrap().contains("não está disponível"));
    }

    #[test]
    fn corrupt_config_falls_back_with_a_warning() {
        let root = tempfile::tempdir().unwrap();
        let default = root.path().join("default");
        fs::create_dir_all(&default).unwrap();
        fs::write(default.join(CONFIG_FILE), "{\"dataDir\": \"D:/dados\"").unwrap();
        let (dir, warning) = resolve(&default);
        assert_eq!(dir, default);
        let warning = warning.expect("a corrupt choice is reported");
        assert!(warning.contains("config.json"), "{warning}");
        // No file at all: the default folder, silently.
        fs::remove_file(default.join(CONFIG_FILE)).unwrap();
        assert_eq!(resolve(&default), (default.clone(), None));
    }
}

//! User profiles saved as JSON files (one per profile) in the app data folder.

use crate::files;
use nfsetable_core::profile::validate_profile;
use nfsetable_core::text::normalize;
use nfsetable_core::Profile;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_ID_LEN: usize = 80;
const MAX_SLUG_LEN: usize = 40;

/// Loads every valid profile of `dir`, sorted by name. Unreadable or invalid files are skipped.
pub fn load_all(dir: &Path) -> Vec<Profile> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut profiles: Vec<Profile> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            let json = fs::read_to_string(&path).ok()?;
            let mut profile: Profile = serde_json::from_str(&json).ok()?;
            profile.builtin = false;
            validate_profile(&profile).ok()?;
            Some(profile)
        })
        .collect();
    profiles.sort_by(|a, b| {
        normalize(&a.name)
            .cmp(&normalize(&b.name))
            .then(a.id.cmp(&b.id))
    });
    profiles
}

/// Validates and writes a profile, assigning an id when it has none. Returns the saved profile.
pub fn save(dir: &Path, mut profile: Profile) -> Result<Profile, String> {
    profile.builtin = false;
    profile.name = profile.name.trim().to_string();
    validate_profile(&profile).map_err(|e| e.to_string())?;
    if profile.id.trim().is_empty() {
        profile.id = new_id(&profile.name);
    }
    let path = file_of(dir, &profile.id)?;

    fs::create_dir_all(dir)
        .map_err(|e| format!("Não foi possível criar a pasta de perfis: {e}"))?;
    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    files::write_atomic(&path, &json)
        .map_err(|e| format!("Não foi possível salvar o perfil: {e}"))?;
    Ok(profile)
}

/// Deletes a saved profile. Deleting a profile that does not exist is not an error.
pub fn delete(dir: &Path, id: &str) -> Result<(), String> {
    let path = file_of(dir, id)?;
    files::remove_if_exists(&path).map_err(|e| format!("Não foi possível excluir o perfil: {e}"))
}

/// Path of a profile file; rejects ids that could escape the folder.
fn file_of(dir: &Path, id: &str) -> Result<PathBuf, String> {
    files::slug_path(dir, id, MAX_ID_LEN)
        .ok_or_else(|| "Identificador de perfil inválido.".to_string())
}

/// `"Prefeitura de Exemplo"` -> `"prefeitura-de-exemplo-<time>"`: readable and unique enough for
/// profiles created by hand.
fn new_id(name: &str) -> String {
    let mut slug = String::new();
    for c in normalize(name).chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
        if slug.len() >= MAX_SLUG_LEN {
            break;
        }
    }
    let slug = slug.trim_matches('-');
    let slug = if slug.is_empty() { "perfil" } else { slug };
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{slug}-{}", to_base36(millis))
}

fn to_base36(mut n: u128) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsetable_core::{Rect, Rule};
    use std::collections::BTreeMap;

    fn profile(name: &str) -> Profile {
        let mut fields = BTreeMap::new();
        fields.insert(
            "net_value".to_string(),
            vec![Rule::Region {
                rect: Rect {
                    x: 0.5,
                    y: 0.5,
                    w: 0.1,
                    h: 0.02,
                },
                page: 0,
                anchor: None,
            }],
        );
        Profile {
            id: String::new(),
            name: name.to_string(),
            builtin: true,
            fingerprint: Vec::new(),
            name_patterns: Vec::new(),
            doc_type: None,
            kind: None,
            fields,
        }
    }

    #[test]
    fn saves_loads_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let saved = save(dir.path(), profile("  Prefeitura de São Exemplo ")).unwrap();
        assert!(saved.id.starts_with("prefeitura-de-sao-exemplo-"));
        assert_eq!(saved.name, "Prefeitura de São Exemplo");
        assert!(!saved.builtin);

        let other = save(dir.path(), profile("Área Azul")).unwrap();
        let loaded = load_all(dir.path());
        assert_eq!(
            loaded.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Área Azul", "Prefeitura de São Exemplo"]
        );

        // Saving again with the same id overwrites.
        let mut renamed = other.clone();
        renamed.name = "Zona Leste".to_string();
        save(dir.path(), renamed).unwrap();
        assert_eq!(load_all(dir.path()).len(), 2);

        delete(dir.path(), &saved.id).unwrap();
        delete(dir.path(), &saved.id).unwrap();
        assert_eq!(load_all(dir.path()).len(), 1);
    }

    #[test]
    fn rejects_unsafe_ids_and_invalid_profiles() {
        let dir = tempfile::tempdir().unwrap();
        assert!(delete(dir.path(), "../outside").is_err());
        assert!(delete(dir.path(), "").is_err());
        let mut bad = profile("x");
        bad.id = "..\\evil".to_string();
        assert!(save(dir.path(), bad).is_err());
        assert!(save(dir.path(), profile("   ")).is_err());
    }

    #[test]
    fn skips_broken_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("broken.json"), "{ not json").unwrap();
        fs::write(dir.path().join("notes.txt"), "hello").unwrap();
        save(dir.path(), profile("Ok")).unwrap();
        assert_eq!(load_all(dir.path()).len(), 1);
        assert!(load_all(&dir.path().join("missing")).is_empty());
    }
}

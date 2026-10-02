//! Extraction results kept between runs, so opening the app does not read every PDF again. They
//! live in `<data folder>/cache/results.json` (compact JSON), keyed by the UI's cache key of each
//! file (its SHA-256 plus its normalized name, since profiles may match by name), and are valid
//! only for one key: the engine's `EXTRACTION_VERSION`, this build and the profiles fingerprint.
//! Any change of key starts an empty cache.

use crate::files;
use nfsetable_core::{profiles_fingerprint, DocResult, Profile, Status, EXTRACTION_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, UNIX_EPOCH};

/// While a reading goes on, the results are written at most this often.
const SAVE_EVERY: Duration = Duration::from_secs(5);

/// Key under which results extracted with `profiles` by this build are valid.
pub fn key(profiles: &[Profile]) -> String {
    format!(
        "{EXTRACTION_VERSION}:{}:{}:{}",
        env!("CARGO_PKG_VERSION"),
        build_id(),
        profiles_fingerprint(profiles)
    )
}

/// Size and modification time of the running executable: every new build (a development build,
/// an update) gets a new key, so results of another build are never reused.
fn build_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let meta = std::env::current_exe().and_then(fs::metadata);
        let modified = meta
            .as_ref()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_millis());
        let len = meta.map_or(0, |m| m.len());
        format!("{len}-{modified}")
    })
}

#[derive(Default, Serialize, Deserialize)]
struct CacheFile {
    key: String,
    results: HashMap<String, DocResult>,
}

pub struct ResultCache {
    path: PathBuf,
    file: CacheFile,
    /// Changed since the last save.
    dirty: bool,
    last_save: Instant,
}

impl ResultCache {
    /// The cache stored in the data folder `dir`; an unreadable or corrupt file is an empty cache.
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("cache").join("results.json");
        let file = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        ResultCache {
            path,
            file,
            dirty: false,
            last_save: Instant::now(),
        }
    }

    /// Drops every result when `key` differs from the one they were read with.
    fn align(&mut self, key: &str) {
        if self.file.key != key {
            self.file = CacheFile {
                key: key.to_string(),
                results: HashMap::new(),
            };
            self.dirty = true;
        }
    }

    /// The known results among the cache keys `ids` (their `path` is empty).
    pub fn get(&mut self, key: &str, ids: &[String]) -> HashMap<String, DocResult> {
        self.align(key);
        ids.iter()
            .filter_map(|id| self.file.results.get(id).map(|r| (id.clone(), r.clone())))
            .collect()
    }

    /// Keeps `result` under the cache key `id`. Errors are not kept (a locked or missing file may
    /// read fine next time), nor files without a key (the scan could not read them).
    pub fn insert(&mut self, key: &str, id: &str, result: &DocResult) {
        if id.is_empty() || result.status == Status::Error {
            return;
        }
        self.align(key);
        let kept = DocResult {
            path: String::new(),
            ..result.clone()
        };
        self.file.results.insert(id.to_string(), kept);
        self.dirty = true;
    }

    /// Something changed and the last save was a while ago (for saves during a long reading).
    pub fn save_due(&self) -> bool {
        self.dirty && self.last_save.elapsed() >= SAVE_EVERY
    }

    /// What to write when something changed: the file and its contents.
    pub fn pending_save(&mut self) -> Option<(PathBuf, String)> {
        if !self.dirty {
            return None;
        }
        let json = serde_json::to_string(&self.file).ok()?;
        self.dirty = false;
        self.last_save = Instant::now();
        Some((self.path.clone(), json))
    }

    /// Marks the cache as changed again after a failed write.
    pub fn save_failed(&mut self) {
        self.dirty = true;
    }
}

/// Writes a `pending_save` result.
pub fn write(path: &Path, json: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    files::write_atomic(path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsetable_core::{builtin_profiles, DocKind};
    use std::collections::BTreeMap;

    fn doc(path: &str, status: Status) -> DocResult {
        DocResult {
            path: path.to_string(),
            status,
            message: None,
            doc_type: "NFS-e".to_string(),
            kind: DocKind::Revenue,
            fields: BTreeMap::new(),
            page_count: 1,
        }
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|h| h.to_string()).collect()
    }

    #[test]
    fn keeps_results_between_runs() {
        let dir = tempfile::tempdir().unwrap();
        let key = key(&builtin_profiles());
        let mut cache = ResultCache::load(dir.path());
        cache.insert(&key, "aaa:a.pdf", &doc("C:/Notas/a.pdf", Status::Ok));
        let (path, json) = cache.pending_save().unwrap();
        write(&path, &json).unwrap();
        assert!(cache.pending_save().is_none(), "nothing new to write");

        let mut again = ResultCache::load(dir.path());
        let found = again.get(&key, &ids(&["aaa:a.pdf", "bbb:b.pdf"]));
        assert_eq!(found.len(), 1);
        assert_eq!(found["aaa:a.pdf"].path, "", "the path comes from the scan");
        assert_eq!(found["aaa:a.pdf"].doc_type, "NFS-e");
    }

    #[test]
    fn other_profiles_start_empty() {
        let dir = tempfile::tempdir().unwrap();
        let mut cache = ResultCache::load(dir.path());
        let builtin = key(&builtin_profiles());
        cache.insert(&builtin, "aaa:a.pdf", &doc("a.pdf", Status::Ok));
        assert!(cache.get(&key(&[]), &ids(&["aaa:a.pdf"])).is_empty());
        assert!(
            cache.get(&builtin, &ids(&["aaa:a.pdf"])).is_empty(),
            "dropped for good"
        );
    }

    #[test]
    fn skips_errors_and_files_without_key() {
        let dir = tempfile::tempdir().unwrap();
        let key = key(&builtin_profiles());
        let mut cache = ResultCache::load(dir.path());
        cache.insert(&key, "aaa:a.pdf", &doc("a.pdf", Status::Error));
        cache.insert(&key, "", &doc("b.pdf", Status::Ok));
        cache.insert(&key, "ccc:c.pdf", &doc("c.pdf", Status::NoText));
        let found = cache.get(&key, &ids(&["aaa:a.pdf", "", "ccc:c.pdf"]));
        assert_eq!(found.keys().collect::<Vec<_>>(), vec!["ccc:c.pdf"]);
    }

    #[test]
    fn saves_during_a_long_reading_only_after_a_while() {
        let dir = tempfile::tempdir().unwrap();
        let key = key(&builtin_profiles());
        let mut cache = ResultCache::load(dir.path());
        assert!(!cache.save_due(), "nothing to save");
        cache.insert(&key, "aaa:a.pdf", &doc("a.pdf", Status::Ok));
        assert!(!cache.save_due(), "just loaded");
        cache.last_save -= SAVE_EVERY;
        assert!(cache.save_due());
        cache.pending_save().unwrap();
        assert!(!cache.save_due());
    }

    #[test]
    fn corrupt_file_is_an_empty_cache() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("cache")).unwrap();
        fs::write(dir.path().join("cache").join("results.json"), "{ quebrado").unwrap();
        let mut cache = ResultCache::load(dir.path());
        assert!(cache
            .get(&key(&builtin_profiles()), &ids(&["aaa:a.pdf"]))
            .is_empty());
    }
}

//! Files found by the last scan of each source, kept in `<data folder>/cache/scans.json` (compact
//! JSON) and given back to the next scan as `ScanOptions::known`: a source that cannot be reached
//! (drive disconnected, folder moved) lists them again, with the values of their earlier readings,
//! and a file that cannot be read now keeps its hash. Only files with a hash are kept, since no
//! earlier reading applies to the others. Sources of every company share it, by path.

use nfsetable_core::{ScanResult, ScannedFile, Source};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ScanCache {
    path: PathBuf,
    /// Files of the last scan of each source, by the source's path as the UI gives it.
    sources: HashMap<String, Vec<ScannedFile>>,
}

impl ScanCache {
    /// The scans kept in the data folder `dir`; an unreadable or corrupt file is an empty cache.
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("cache").join("scans.json");
        let sources = fs::read_to_string(&path)
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();
        ScanCache { path, sources }
    }

    /// The files kept for `sources`.
    pub fn known(&self, sources: &[Source]) -> Vec<ScannedFile> {
        sources
            .iter()
            .filter_map(|s| self.sources.get(&s.path))
            .flatten()
            .cloned()
            .collect()
    }

    /// Keeps the files of `scan` for each of its sources; returns what to write when that changed
    /// anything (the file and its contents).
    pub fn update(&mut self, scan: &ScanResult) -> Option<(PathBuf, String)> {
        let mut by_source: HashMap<&str, Vec<ScannedFile>> = scan
            .sources
            .iter()
            .map(|s| (s.path.as_str(), Vec::new()))
            .collect();
        for file in scan.files.iter().filter(|f| !f.hash.is_empty()) {
            if let Some(files) = by_source.get_mut(file.source.as_str()) {
                // What the next scan reads of a known file: copies and availability are its own.
                files.push(ScannedFile {
                    duplicate_of: None,
                    available: false,
                    ..file.clone()
                });
            }
        }
        let mut changed = false;
        for (source, files) in by_source {
            if self.sources.get(source) != Some(&files) {
                self.sources.insert(source.to_string(), files);
                changed = true;
            }
        }
        if !changed {
            return None;
        }
        let json = serde_json::to_string(&self.sources).ok()?;
        Some((self.path.clone(), json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsetable_core::{scan, ScanOptions};

    fn options(sources: &[&Path]) -> ScanOptions {
        ScanOptions {
            sources: sources
                .iter()
                .map(|p| Source {
                    path: p.to_string_lossy().into_owned(),
                    recursive: false,
                })
                .collect(),
            exclude: vec![],
            known: vec![],
        }
    }

    /// Scans `sources` with what `cache` knows of them and keeps the result, like `scan_sources`.
    fn scan_with(cache: &mut ScanCache, sources: &[&Path]) -> ScanResult {
        let mut opts = options(sources);
        opts.known = cache.known(&opts.sources);
        let result = scan(&opts);
        if let Some((path, json)) = cache.update(&result) {
            crate::results::write(&path, &json).unwrap();
        }
        result
    }

    #[test]
    fn a_disconnected_folder_lists_its_files_after_a_restart() {
        let (data, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let notes = root.path().join("notas");
        fs::create_dir(&notes).unwrap();
        fs::write(notes.join("a.pdf"), b"%PDF a").unwrap();
        let first = scan_with(&mut ScanCache::load(data.path()), &[&notes]);
        assert!(first.files[0].available);

        // The app opens again with the folder out of reach.
        fs::rename(&notes, root.path().join("longe")).unwrap();
        let mut cache = ScanCache::load(data.path());
        let offline = scan_with(&mut cache, &[&notes]);
        assert_eq!(offline.files.len(), 1);
        assert_eq!(offline.files[0].hash, first.files[0].hash);
        assert!(!offline.files[0].available);
        assert!(
            cache.update(&offline).is_none(),
            "nothing new to write while the folder is away"
        );

        // And once more: the files are still known.
        let again = scan_with(&mut ScanCache::load(data.path()), &[&notes]);
        assert_eq!(again.files, offline.files);
    }

    #[test]
    fn keeps_only_files_with_a_hash_by_their_source() {
        let (data, root) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (one, two) = (root.path().join("um"), root.path().join("dois"));
        for dir in [&one, &two] {
            fs::create_dir(dir).unwrap();
            fs::write(dir.join("nota.pdf"), dir.to_string_lossy().as_bytes()).unwrap();
        }
        let mut cache = ScanCache::load(data.path());
        let result = scan_with(&mut cache, &[&one, &two]);
        let source = two.to_string_lossy();
        let of_two = result.files.iter().find(|f| f.source == source).unwrap();
        let expected = ScannedFile {
            duplicate_of: None,
            available: false,
            ..of_two.clone()
        };
        assert_eq!(cache.known(&options(&[&two]).sources), vec![expected]);

        // Files the scan could not identify are not kept.
        let mut unreadable = result.clone();
        unreadable.files.iter_mut().for_each(|f| f.hash.clear());
        assert!(cache.update(&unreadable).is_some());
        assert!(cache.known(&options(&[&one, &two]).sources).is_empty());
    }
}

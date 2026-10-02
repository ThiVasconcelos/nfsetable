//! Lists the PDF files of the chosen sources (folders or single files). Does not need PDFium.

use crate::model::{ScanOptions, ScanResult, ScannedFile, SourceInfo};
use crate::text::normalize;
use sha2::{Digest, Sha256};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Scans the sources and returns the PDF files found.
///
/// - Only `*.pdf` files (case-insensitive extension) are listed.
/// - Folders are walked recursively only when `recursive` is set.
/// - A file reachable from several sources is listed once (canonical paths are deduplicated).
/// - Files whose NAME contains one of `exclude` (case- and accent-insensitive) go to `excluded`.
/// - Files are sorted by folder, then by file name without extension (so "nota.pdf" comes before
///   "nota (1).pdf"); byte-identical files get `duplicate_of` = path of the first one.
/// - Unreadable folders or files never abort the scan.
pub fn scan(options: &ScanOptions) -> ScanResult {
    let mut collector = Collector::new(&options.exclude);
    let mut sources = Vec::with_capacity(options.sources.len());

    for source in &options.sources {
        let root = PathBuf::from(&source.path);
        let metadata = fs::metadata(&root).ok();
        let exists = metadata.is_some();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        let mut file_count = 0u32;

        if is_dir {
            let max_depth = if source.recursive { usize::MAX } else { 1 };
            let walker = WalkDir::new(&root).min_depth(1).max_depth(max_depth);
            // Entries that fail (e.g. permission denied on a subfolder) are skipped.
            for entry in walker.into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() && collector.visit(path) {
                    file_count += 1;
                }
            }
        } else if exists && collector.visit(&root) {
            file_count += 1;
        }

        sources.push(SourceInfo {
            path: source.path.clone(),
            is_dir,
            exists,
            file_count,
        });
    }

    let (mut found, mut excluded) = (collector.found, collector.excluded);
    found.sort_by_cached_key(|path| sort_key(path));
    excluded.sort_by_cached_key(|path| sort_key(path));

    let mut first_by_hash: HashMap<String, String> = HashMap::new();
    let files = found
        .into_iter()
        .map(|path| {
            let path_str = path_to_string(&path);
            let hash = hash_file(&path).unwrap_or_default();
            let duplicate_of = if hash.is_empty() {
                None
            } else {
                match first_by_hash.entry(hash.clone()) {
                    Entry::Occupied(first) => Some(first.get().clone()),
                    Entry::Vacant(slot) => {
                        slot.insert(path_str.clone());
                        None
                    }
                }
            };
            ScannedFile {
                name: file_name(&path),
                dir: path.parent().map(path_to_string).unwrap_or_default(),
                size: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                path: path_str,
                hash,
                duplicate_of,
            }
        })
        .collect();

    ScanResult {
        sources,
        files,
        excluded: excluded.iter().map(|p| path_to_string(p)).collect(),
    }
}

struct Collector {
    exclude: Vec<String>,
    seen: HashSet<PathBuf>,
    seen_excluded: HashSet<PathBuf>,
    found: Vec<PathBuf>,
    excluded: Vec<PathBuf>,
}

impl Collector {
    fn new(exclude: &[String]) -> Self {
        Collector {
            exclude: exclude
                .iter()
                .map(|e| normalize(e))
                .filter(|e| !e.is_empty())
                .collect(),
            seen: HashSet::new(),
            seen_excluded: HashSet::new(),
            found: Vec::new(),
            excluded: Vec::new(),
        }
    }

    /// Records a candidate file. Returns true when it is a PDF that passed the exclude filter
    /// (even if another source already listed it).
    fn visit(&mut self, path: &Path) -> bool {
        if !is_pdf(path) {
            return false;
        }
        let path = display_path(path);
        if self.is_excluded(&path) {
            if self.seen_excluded.insert(path.clone()) {
                self.excluded.push(path);
            }
            return false;
        }
        if self.seen.insert(path.clone()) {
            self.found.push(path);
        }
        true
    }

    fn is_excluded(&self, path: &Path) -> bool {
        if self.exclude.is_empty() {
            return false;
        }
        let name = normalize(&file_name(path));
        self.exclude.iter().any(|e| name.contains(e.as_str()))
    }
}

/// Folder, then lowercase file stem, then the full path as a tie breaker.
fn sort_key(path: &Path) -> (PathBuf, String, PathBuf) {
    (
        path.parent().map(Path::to_path_buf).unwrap_or_default(),
        path.file_stem()
            .map(|stem| stem.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
        path.to_path_buf(),
    )
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.to_string_lossy().eq_ignore_ascii_case("pdf"))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The file name of `path` as text (lossy), empty when it has none.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Canonical absolute path (symlinks resolved, consistent separators), without the Windows
/// verbatim prefix. Falls back to the given path when it cannot be canonicalized.
fn display_path(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(canonical) => strip_verbatim(canonical),
        Err(_) => path.to_path_buf(),
    }
}

#[cfg(windows)]
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path;
    };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        let bytes = rest.as_bytes();
        if bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\'
        {
            return PathBuf::from(rest);
        }
    }
    path
}

#[cfg(not(windows))]
fn strip_verbatim(path: PathBuf) -> PathBuf {
    path
}

/// SHA-256 of the file content, lowercase hex.
fn hash_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

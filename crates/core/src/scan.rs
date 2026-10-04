//! Lists the PDF files of the chosen sources (folders or single files). Does not need PDFium.

use crate::model::{ScanOptions, ScanResult, ScannedFile, SourceInfo};
use crate::text::normalize;
use sha2::{Digest, Sha256};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, Metadata};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

/// Unreadable paths reported per source (enough to say what is missing).
const MAX_UNREADABLE: usize = 20;

/// Size and modification time of a file, as listed (no extra open per file).
#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    size: u64,
    modified: SystemTime,
}

impl Stamp {
    fn of(metadata: &Metadata) -> Option<Stamp> {
        Some(Stamp {
            size: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

/// Hashes of the files scanned so far in this process, by path: a rescan does not read a file
/// again while its size and modification time are the same.
static KNOWN_HASHES: LazyLock<Mutex<HashMap<PathBuf, (Stamp, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Scans the sources and returns the PDF files found.
///
/// - Only `*.pdf` files (case-insensitive extension) are listed.
/// - Folders are walked recursively only when `recursive` is set.
/// - A file reachable from several sources is listed once (each source folder is canonicalized
///   and the files below it take its form, so the same file gets the same path).
/// - Files whose NAME contains one of `exclude` (case- and accent-insensitive) go to `excluded`.
/// - Files are sorted by folder, then by file name without extension (so "nota.pdf" comes before
///   "nota (1).pdf"); byte-identical files get `duplicate_of` = path of the first one.
/// - Unreadable folders or files never abort the scan.
/// - Hashes are remembered for the process (see `KNOWN_HASHES`), so a rescan of unchanged files
///   only lists the folders.
pub fn scan(options: &ScanOptions) -> ScanResult {
    let mut collector = Collector::new(&options.exclude);
    let mut sources = Vec::with_capacity(options.sources.len());

    for source in &options.sources {
        let root = PathBuf::from(&source.path);
        let metadata = fs::metadata(&root).ok();
        let exists = metadata.is_some();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        let mut file_count = 0u32;
        let mut unreadable = Vec::new();

        if is_dir {
            let max_depth = if source.recursive { usize::MAX } else { 1 };
            let base = display_path(&root);
            let walker = WalkDir::new(&root).min_depth(1).max_depth(max_depth);
            // Entries that fail (e.g. permission denied on a subfolder) are skipped and reported.
            for entry in walker {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        if unreadable.len() < MAX_UNREADABLE {
                            let path = error.path().unwrap_or(&root);
                            unreadable.push(path_to_string(path));
                        }
                        continue;
                    }
                };
                if !is_pdf(entry.path()) {
                    continue;
                }
                let Some(stamp) = listed_file(&entry) else {
                    continue;
                };
                let path = match entry.path().strip_prefix(&root) {
                    Ok(relative) => base.join(relative),
                    Err(_) => display_path(entry.path()),
                };
                if collector.visit(path, stamp) {
                    file_count += 1;
                }
            }
        } else if exists && is_pdf(&root) {
            let stamp = metadata.as_ref().and_then(Stamp::of);
            if collector.visit(display_path(&root), stamp) {
                file_count += 1;
            }
        }

        sources.push(SourceInfo {
            path: source.path.clone(),
            is_dir,
            exists,
            file_count,
            unreadable,
        });
    }

    let (mut found, mut excluded) = (collector.found, collector.excluded);
    found.sort_by_cached_key(|(path, _)| sort_key(path));
    excluded.sort_by_cached_key(|path| sort_key(path));

    let mut first_by_hash: HashMap<String, String> = HashMap::new();
    let files = found
        .into_iter()
        .map(|(path, stamp)| {
            let path_str = path_to_string(&path);
            let hash = known_hash(&path, stamp);
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
                size: stamp.map_or(0, |s| s.size),
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
    found: Vec<(PathBuf, Option<Stamp>)>,
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

    /// Records a PDF at its display path. Returns true when it passed the exclude filter (even
    /// if another source already listed it).
    fn visit(&mut self, path: PathBuf, stamp: Option<Stamp>) -> bool {
        if self.is_excluded(&path) {
            if self.seen_excluded.insert(path.clone()) {
                self.excluded.push(path);
            }
            return false;
        }
        if self.seen.insert(path.clone()) {
            self.found.push((path, stamp));
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

/// The stamp of a listed regular file (a symlink counts when it points to one), `None` for
/// folders and anything else. Uses what the directory listing already knows where it can.
fn listed_file(entry: &DirEntry) -> Option<Option<Stamp>> {
    let kind = entry.file_type();
    if kind.is_symlink() {
        let metadata = fs::metadata(entry.path()).ok()?;
        return metadata.is_file().then(|| Stamp::of(&metadata));
    }
    kind.is_file()
        .then(|| entry.metadata().ok().as_ref().and_then(Stamp::of))
}

/// The SHA-256 of `path`, reused from an earlier scan while its stamp is the same; empty when it
/// cannot be read (and then not remembered, so the next scan tries again).
fn known_hash(path: &Path, stamp: Option<Stamp>) -> String {
    let mut known = KNOWN_HASHES.lock().unwrap_or_else(PoisonError::into_inner);
    if let (Some(stamp), Some((seen, hash))) = (stamp, known.get(path)) {
        if *seen == stamp {
            return hash.clone();
        }
    }
    drop(known);
    let hash = hash_file(path).unwrap_or_default();
    if let (Some(stamp), false) = (stamp, hash.is_empty()) {
        known = KNOWN_HASHES.lock().unwrap_or_else(PoisonError::into_inner);
        known.insert(path.to_path_buf(), (stamp, hash.clone()));
    }
    hash
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Source;
    use std::time::Duration;

    fn scan_dir(dir: &Path) -> ScanResult {
        scan(&ScanOptions {
            sources: vec![Source {
                path: dir.to_string_lossy().into_owned(),
                recursive: true,
            }],
            exclude: vec![],
        })
    }

    #[test]
    fn unchanged_files_are_not_read_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nota.pdf");
        fs::write(&path, b"%PDF-1.4 conteudo A").unwrap();
        let file = File::options().write(true).open(&path).unwrap();
        let stamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1_790_000_000);
        file.set_modified(stamp).unwrap();
        let first = scan_dir(dir.path()).files[0].hash.clone();

        // Same size and modification time: the remembered hash is used (the file is not read).
        fs::write(&path, b"%PDF-1.4 conteudo B").unwrap();
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(stamp)
            .unwrap();
        assert_eq!(scan_dir(dir.path()).files[0].hash, first);

        // A new modification time: read again.
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(stamp + Duration::from_secs(60))
            .unwrap();
        let again = scan_dir(dir.path()).files[0].hash.clone();
        assert_ne!(again, first);
        assert_eq!(
            again,
            format!("{:x}", Sha256::digest(b"%PDF-1.4 conteudo B"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn reports_folders_it_cannot_read() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let closed = dir.path().join("fechada");
        fs::create_dir(&closed).unwrap();
        fs::write(closed.join("nota.pdf"), b"%PDF").unwrap();
        fs::write(dir.path().join("aberta.pdf"), b"%PDF aberta").unwrap();
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read_dir(&closed).is_ok() {
            // Running as root: permissions are not enforced.
            fs::set_permissions(&closed, fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }
        let result = scan_dir(dir.path());
        fs::set_permissions(&closed, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.sources[0].unreadable.len(), 1);
        assert!(result.sources[0].unreadable[0].ends_with("fechada"));
    }

    #[test]
    fn lists_only_pdf_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("nota.PDF"), b"%PDF").unwrap();
        fs::write(dir.path().join("planilha.xlsx"), b"x").unwrap();
        fs::create_dir(dir.path().join("pasta.pdf")).unwrap();
        fs::write(
            dir.path().join("pasta.pdf").join("dentro.pdf"),
            b"%PDF dentro",
        )
        .unwrap();
        let result = scan_dir(dir.path());
        let names: Vec<&str> = result.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["nota.PDF", "dentro.pdf"]);
        assert_eq!(result.files[0].size, 4);
        assert_eq!(result.sources[0].file_count, 2);
    }
}

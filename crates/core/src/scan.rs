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
///   "nota (1).pdf"); byte-identical files get `duplicate_of` = path of the first one (see
///   `mark_duplicates`).
/// - Unreadable folders or files never abort the scan.
/// - Hashes are remembered for the process (see `KNOWN_HASHES`), so a rescan of unchanged files
///   only lists the folders.
/// - A source that cannot be reached lists its files of `known` again, unavailable, and a file
///   that cannot be read takes its hash of `known` while its size is the same (see
///   [`ScanOptions::known`]).
pub fn scan(options: &ScanOptions) -> ScanResult {
    let mut collector = Collector::new(&options.exclude);
    let mut sources = Vec::with_capacity(options.sources.len());
    let mut unreachable = Vec::new();

    for (index, source) in options.sources.iter().enumerate() {
        let root = PathBuf::from(&source.path);
        let metadata = fs::metadata(&root).ok();
        let exists = metadata.is_some();
        let is_dir = metadata.as_ref().is_some_and(|m| m.is_dir());
        let mut file_count = 0u32;
        let mut unreadable = Vec::new();

        if !exists {
            unreachable.push(index);
        } else if is_dir {
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
                if collector.visit(path, Listed::Now(stamp), index) {
                    file_count += 1;
                }
            }
        } else if is_pdf(&root) {
            let stamp = metadata.as_ref().and_then(Stamp::of);
            if collector.visit(display_path(&root), Listed::Now(stamp), index) {
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

    // After every reachable source, so a file still reachable from one of them is listed as it is
    // now, not as it was.
    for index in unreachable {
        let source = &options.sources[index].path;
        for file in options.known.iter().filter(|f| &f.source == source) {
            let earlier = Listed::Earlier {
                hash: file.hash.clone(),
                size: file.size,
            };
            collector.visit(PathBuf::from(&file.path), earlier, index);
        }
    }

    let (mut found, mut excluded) = (collector.found, collector.excluded);
    found.sort_by_cached_key(|f| sort_key(&f.path));
    excluded.sort_by_cached_key(|path| sort_key(path));

    let known: HashMap<&str, &ScannedFile> = options
        .known
        .iter()
        .filter(|f| !f.hash.is_empty())
        .map(|f| (f.path.as_str(), f))
        .collect();
    let mut files: Vec<ScannedFile> = found
        .into_iter()
        .map(|found| {
            let path = path_to_string(&found.path);
            let (hash, size, available) = match found.listed {
                Listed::Earlier { hash, size } => (hash, size, false),
                Listed::Now(stamp) => {
                    let hash = known_hash(&found.path, stamp);
                    let size = stamp.map_or(0, |s| s.size);
                    if !hash.is_empty() {
                        (hash, size, true)
                    } else {
                        // Not readable now: the content of an earlier scan, if it looks the same.
                        let earlier = known
                            .get(path.as_str())
                            .filter(|k| stamp.is_some() && k.size == size)
                            .map(|k| k.hash.clone());
                        (earlier.unwrap_or_default(), size, false)
                    }
                }
            };
            ScannedFile {
                name: file_name(&found.path),
                dir: found.path.parent().map(path_to_string).unwrap_or_default(),
                size,
                path,
                hash,
                duplicate_of: None,
                source: options.sources[found.source].path.clone(),
                available,
            }
        })
        .collect();
    mark_duplicates(&mut files);

    ScanResult {
        sources,
        files,
        excluded: excluded.iter().map(|p| path_to_string(p)).collect(),
    }
}

/// How a file came into the scan.
enum Listed {
    /// Listed from its folder now (its stamp, when known).
    Now(Option<Stamp>),
    /// A file of an earlier scan whose source cannot be reached: its hash and size back then.
    Earlier { hash: String, size: u64 },
}

/// A PDF of the scan: its display path, how it came in and the source (index) that listed it.
struct Found {
    path: PathBuf,
    listed: Listed,
    source: usize,
}

struct Collector {
    exclude: Vec<String>,
    seen: HashSet<PathBuf>,
    seen_excluded: HashSet<PathBuf>,
    found: Vec<Found>,
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

    /// Records a PDF at its display path, listed by source `source`. Returns true when it passed
    /// the exclude filter (even if another source already listed it).
    fn visit(&mut self, path: PathBuf, listed: Listed, source: usize) -> bool {
        if self.is_excluded(&path) {
            if self.seen_excluded.insert(path.clone()) {
                self.excluded.push(path);
            }
            return false;
        }
        if self.seen.insert(path.clone()) {
            self.found.push(Found {
                path,
                listed,
                source,
            });
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

/// Byte-identical files get `duplicate_of` = path of the first one in scan order, among the files
/// the scan could read first: a copy that can be reached is the original of one that cannot.
fn mark_duplicates(files: &mut [ScannedFile]) {
    let mut first_by_hash: HashMap<String, String> = HashMap::new();
    for available in [true, false] {
        let group = files
            .iter_mut()
            .filter(|f| f.available == available && !f.hash.is_empty());
        for file in group {
            match first_by_hash.entry(file.hash.clone()) {
                Entry::Occupied(first) => file.duplicate_of = Some(first.get().clone()),
                Entry::Vacant(slot) => {
                    slot.insert(file.path.clone());
                }
            }
        }
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

    fn options(dir: &Path, known: Vec<ScannedFile>) -> ScanOptions {
        ScanOptions {
            sources: vec![Source {
                path: dir.to_string_lossy().into_owned(),
                recursive: true,
            }],
            exclude: vec![],
            known,
        }
    }

    fn scan_dir(dir: &Path) -> ScanResult {
        scan(&options(dir, vec![]))
    }

    /// Keeps `path` from being read while the guard lives; None where that cannot be done.
    #[cfg(windows)]
    fn lock(path: &Path) -> Option<File> {
        use std::os::windows::fs::OpenOptionsExt;
        File::options().read(true).share_mode(0).open(path).ok()
    }

    #[cfg(unix)]
    struct Locked(PathBuf);

    #[cfg(unix)]
    impl Drop for Locked {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o644));
        }
    }

    #[cfg(unix)]
    fn lock(path: &Path) -> Option<Locked> {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o000)).ok()?;
        let locked = Locked(path.to_path_buf());
        // Running as root: permissions are not enforced.
        File::open(path).is_err().then_some(locked)
    }

    #[test]
    fn a_source_out_of_reach_lists_its_known_files() {
        let root = tempfile::tempdir().unwrap();
        let notes = root.path().join("notas");
        fs::create_dir(&notes).unwrap();
        fs::write(notes.join("a.pdf"), b"%PDF a").unwrap();
        fs::write(notes.join("b.pdf"), b"%PDF b").unwrap();
        fs::write(notes.join("b-copia.pdf"), b"%PDF b").unwrap();
        let before = scan_dir(&notes);
        let source = notes.to_string_lossy();
        assert!(before
            .files
            .iter()
            .all(|f| f.available && f.source == source));
        assert!(
            before.files[2].duplicate_of.is_some(),
            "b-copia is a copy of b"
        );

        // The folder is gone (moved, or on a drive that was disconnected).
        fs::rename(&notes, root.path().join("movida")).unwrap();
        // A file another source listed is not this source's.
        let mut known = before.files.clone();
        known.push(ScannedFile {
            path: path_to_string(&notes.join("z.pdf")),
            source: "D:/Outra fonte".into(),
            ..before.files[0].clone()
        });
        let after = scan(&options(&notes, known.clone()));
        assert!(!after.sources[0].exists);
        assert_eq!(after.sources[0].file_count, 0);
        let expected: Vec<ScannedFile> = before
            .files
            .iter()
            .map(|f| ScannedFile {
                available: false,
                ..f.clone()
            })
            .collect();
        assert_eq!(after.files, expected, "same paths, hashes and copies");

        // The exclude filter still applies.
        let mut excluding = options(&notes, known);
        excluding.exclude = vec!["CÓPIA".into()];
        let after = scan(&excluding);
        assert_eq!(after.files.len(), 2);
        assert_eq!(after.excluded, vec![before.files[2].path.clone()]);
    }

    #[test]
    fn a_reachable_copy_is_the_original_of_an_unreachable_one() {
        let root = tempfile::tempdir().unwrap();
        // "a-antiga" sorts before "b-nova": the unreachable copy comes first in scan order.
        let old = root.path().join("a-antiga");
        let new = root.path().join("b-nova");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("nota.pdf"), b"%PDF nota").unwrap();
        let before = scan_dir(&old);
        fs::rename(&old, &new).unwrap();

        let mut both = options(&old, before.files);
        both.sources.push(Source {
            path: new.to_string_lossy().into_owned(),
            recursive: false,
        });
        let after = scan(&both);
        assert_eq!(after.files.len(), 2);
        let (moved, here) = (&after.files[0], &after.files[1]);
        assert!(!moved.available && here.available);
        assert_eq!(moved.duplicate_of.as_deref(), Some(here.path.as_str()));
        assert_eq!(here.duplicate_of, None);
        assert_eq!(here.source, new.to_string_lossy());
    }

    #[test]
    fn a_file_that_cannot_be_read_keeps_its_known_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nota.pdf");
        fs::write(&path, b"%PDF bloqueado").unwrap();
        let size = fs::metadata(&path).unwrap().len();
        let lock = lock(&path);
        if cfg!(windows) {
            assert!(lock.is_some(), "Windows can always lock the file");
        }
        let Some(_lock) = lock else {
            return;
        };
        let listed = path_to_string(&display_path(dir.path()).join("nota.pdf"));
        let known = |size| ScannedFile {
            path: listed.clone(),
            name: "nota.pdf".into(),
            dir: String::new(),
            size,
            hash: "abc".into(),
            duplicate_of: None,
            source: String::new(),
            available: true,
        };

        let same = scan(&options(dir.path(), vec![known(size)]));
        assert_eq!(same.files[0].hash, "abc");
        assert!(!same.files[0].available);

        // Another size is another content: its values are not known.
        let changed = scan(&options(dir.path(), vec![known(size + 1)]));
        assert_eq!(changed.files[0].hash, "");
        assert!(!changed.files[0].available);
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

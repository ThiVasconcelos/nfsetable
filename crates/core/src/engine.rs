//! PDFium binding and the public extraction engine.

use crate::error::CoreError;
use crate::extract;
use crate::model::{
    DocResult, FieldDef, FieldKind, Profile, Rect, RegionRead, RenderedPage, Rule, RuleTest,
};
use base64::Engine as _;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// Folder name, under `vendor/pdfium/`, of the PDFium build for the current target.
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub const PDFIUM_PLATFORM: &str = "windows-x64";
/// Folder name, under `vendor/pdfium/`, of the PDFium build for the current target.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub const PDFIUM_PLATFORM: &str = "linux-x64";
/// Folder name, under `vendor/pdfium/`, of the PDFium build for the current target.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub const PDFIUM_PLATFORM: &str = "macos-arm64";
/// Folder name, under `vendor/pdfium/`, of the PDFium build for the current target.
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub const PDFIUM_PLATFORM: &str = "macos-x64";
/// Folder name, under `vendor/pdfium/`, of the PDFium build for the current target.
#[cfg(not(any(
    all(target_os = "windows", target_arch = "x86_64"),
    all(target_os = "linux", target_arch = "x86_64"),
    all(
        target_os = "macos",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ),
)))]
pub const PDFIUM_PLATFORM: &str = "unsupported";

/// Environment variable naming an extra folder where the PDFium library is searched.
pub const PDFIUM_ENV_VAR: &str = "PDFIUM_LIB_DIR";

/// The PDFium folder of a development checkout, `vendor/pdfium/<platform>` at the repository root
/// (filled by `scripts/fetch-pdfium`): for tests, examples and debug builds.
#[doc(hidden)]
pub fn dev_pdfium_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/pdfium")
        .join(PDFIUM_PLATFORM)
}

const MIN_RENDER_WIDTH_PX: u32 = 64;
const MAX_RENDER_WIDTH_PX: u32 = 4096;
const MAX_RENDER_HEIGHT_PX: i32 = 8192;

/// pdfium-render keeps its bindings in a process-wide static that can be initialized only once.
/// This lock serializes binding attempts and remembers which library was bound, so that later
/// `Engine::new` calls in the same process (tests, several windows) reuse it.
static BOUND_LIBRARY: Mutex<Option<String>> = Mutex::new(None);

/// PDFium is not thread safe. pdfium-render serializes each call, but a sequence of calls from two
/// threads (open or build a document, read it, save it) can still interleave inside PDFium's
/// global state, which garbled the text of PDFs built at the same time. Every public operation
/// holds this lock for its whole duration; low-level users of [`Engine::pdfium`] take
/// [`Engine::lock`].
static PDFIUM_LOCK: Mutex<()> = Mutex::new(());

/// The extraction engine. Cheap to share: wrap it in an `Arc` (it is `Send + Sync`; every PDFium
/// call is serialized by pdfium-render's `thread_safe` feature).
#[derive(Debug)]
pub struct Engine {
    pdfium: Pdfium,
    library: String,
}

impl Engine {
    /// Binds PDFium dynamically. The library (`pdfium.dll`, `libpdfium.so` or `libpdfium.dylib`)
    /// is searched, in order, in each of `search_dirs`, in the folder named by the
    /// `PDFIUM_LIB_DIR` environment variable, in the folder of the current executable, and
    /// finally among the system libraries.
    pub fn new(search_dirs: &[PathBuf]) -> Result<Engine, CoreError> {
        let mut bound = BOUND_LIBRARY
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(library) = bound.as_ref() {
            return Ok(Engine::reuse_bindings(library.clone()));
        }

        let library_name = Pdfium::pdfium_platform_library_name();
        let mut tried: Vec<String> = Vec::new();

        for dir in candidate_dirs(search_dirs) {
            let library_path = dir.join(&library_name);
            if !library_path.is_file() {
                tried.push(library_path.display().to_string());
                continue;
            }
            match Engine::from_bind(
                Pdfium::bind_to_library(&library_path),
                library_path.display().to_string(),
                &mut bound,
            ) {
                Ok(engine) => return Ok(engine),
                Err(err) => tried.push(format!(
                    "{} ({})",
                    library_path.display(),
                    describe_bind_error(&err)
                )),
            }
        }

        match Engine::from_bind(
            Pdfium::bind_to_system_library(),
            format!("{} (sistema)", library_name.to_string_lossy()),
            &mut bound,
        ) {
            Ok(engine) => Ok(engine),
            Err(err) => {
                tried.push(format!(
                    "bibliotecas do sistema ({})",
                    describe_bind_error(&err)
                ));
                Err(CoreError::PdfiumUnavailable(format!(
                    "Arquivo {} não encontrado ou inválido. Locais verificados: {}.",
                    library_name.to_string_lossy(),
                    tried.join("; ")
                )))
            }
        }
    }

    /// The engine for the outcome of a bind: the bindings just loaded from `library`, or the ones
    /// this process had already loaded (remembered in `bound`). Other errors are given back.
    fn from_bind(
        result: Result<Box<dyn PdfiumLibraryBindings>, PdfiumError>,
        library: String,
        bound: &mut Option<String>,
    ) -> Result<Engine, PdfiumError> {
        let engine = match result {
            Ok(bindings) => Engine {
                pdfium: Pdfium::new(bindings),
                library,
            },
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => {
                Engine::reuse_bindings("(já carregada neste processo)".to_string())
            }
            Err(err) => return Err(err),
        };
        *bound = Some(engine.library.clone());
        Ok(engine)
    }

    /// A new `Pdfium` handle over bindings already initialized in this process.
    fn reuse_bindings(library: String) -> Engine {
        // With initialized bindings, `Pdfium::default()` skips loading any library and returns a
        // handle that shares the existing bindings.
        Engine {
            pdfium: Pdfium::default(),
            library,
        }
    }

    /// Description of the PDFium library in use (normally its path).
    pub fn library(&self) -> &str {
        &self.library
    }

    /// Low-level PDFium access, for tools such as the synthetic fixture generator. Hold
    /// [`Engine::lock`] while using it.
    pub fn pdfium(&self) -> &Pdfium {
        &self.pdfium
    }

    /// Exclusive access to PDFium for a sequence of low-level calls through [`Engine::pdfium`].
    /// Do not call other `Engine` methods while holding it: they take the same lock.
    pub fn lock(&self) -> MutexGuard<'static, ()> {
        PDFIUM_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Extracts `fields` from the document at `path`.
    ///
    /// `profiles` holds built-in profiles (`builtin: true`) and user profiles, in the order they
    /// should be tried. Per field: built-in profiles whose fingerprint matches, then user profiles
    /// whose fingerprint matches (given order), then the automatic heuristic. Pages
    /// `0..min(page_count, 5)` are searched and the first hit wins.
    ///
    /// Never fails: problems are reported through [`DocResult::status`] and
    /// [`DocResult::message`].
    pub fn extract(&self, path: &Path, profiles: &[Profile], fields: &[FieldDef]) -> DocResult {
        let _pdfium = self.lock();
        extract::extract_document(self, path, profiles, fields)
    }

    /// Reads the text inside `rect` (absolute points, top-left origin) of a 0-based `page` and
    /// parses it as `kind`. Also suggests a region rule that reproduces the read on other files.
    pub fn read_region(
        &self,
        path: &Path,
        page: u32,
        rect: Rect,
        kind: FieldKind,
    ) -> Result<RegionRead, CoreError> {
        let _pdfium = self.lock();
        extract::read_region(self, path, page, rect, kind)
    }

    /// Applies `rule` for `field` to the document at `path`. Never fails: problems are reported
    /// through [`RuleTest::error`].
    pub fn test_rule(&self, path: &Path, field: &FieldDef, rule: &Rule) -> RuleTest {
        let _pdfium = self.lock();
        extract::test_rule(self, path, field, rule)
    }

    /// Renders a 0-based `page` as a PNG data URL about `target_width_px` pixels wide.
    pub fn render_page(
        &self,
        path: &Path,
        page: u32,
        target_width_px: u32,
    ) -> Result<RenderedPage, CoreError> {
        let _pdfium = self.lock();
        let document = self.open(path)?;
        let pages = document.pages();
        let page_count = page_count(pages);
        if page >= page_count {
            return Err(CoreError::PageOutOfRange {
                page,
                count: page_count,
            });
        }
        let pdf_page = pages.get(page as PdfPageIndex)?;
        let width_pt = pdf_page.width().value;
        let height_pt = pdf_page.height().value;

        let width_px = target_width_px.clamp(MIN_RENDER_WIDTH_PX, MAX_RENDER_WIDTH_PX) as Pixels;
        let config = PdfRenderConfig::new()
            .set_target_width(width_px)
            .set_maximum_height(MAX_RENDER_HEIGHT_PX)
            .render_form_data(true)
            .render_annotations(true);
        let bitmap = pdf_page.render_with_config(&config)?;
        let image = bitmap.as_image()?.into_rgb8();

        let mut png = Vec::new();
        PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ExtendedColorType::Rgb8,
            )
            .map_err(|e| CoreError::Render(e.to_string()))?;

        let mut data_url = String::with_capacity(22 + png.len().div_ceil(3) * 4);
        data_url.push_str("data:image/png;base64,");
        base64::engine::general_purpose::STANDARD.encode_string(&png, &mut data_url);

        Ok(RenderedPage {
            data_url,
            width_pt,
            height_pt,
            page_count,
        })
    }

    /// Opens a document fully in memory (NFS-e PDFs are small), mapping failures to pt-BR errors.
    pub(crate) fn open(&self, path: &Path) -> Result<PdfDocument<'_>, CoreError> {
        let bytes = std::fs::read(path)?;
        Ok(self.pdfium.load_pdf_from_byte_vec(bytes, None)?)
    }
}

/// Number of pages of a document, as `u32`.
pub(crate) fn page_count(pages: &PdfPages<'_>) -> u32 {
    u32::try_from(pages.len()).unwrap_or(0)
}

/// `search_dirs`, then `PDFIUM_LIB_DIR`, then the folder of the current executable; duplicates
/// removed, order kept.
fn candidate_dirs(search_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = search_dirs.to_vec();
    if let Some(dir) = std::env::var_os(PDFIUM_ENV_VAR).filter(|d| !d.is_empty()) {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        dirs.push(dir);
    }
    let mut unique: Vec<PathBuf> = Vec::with_capacity(dirs.len());
    for dir in dirs {
        if !unique.contains(&dir) {
            unique.push(dir);
        }
    }
    unique
}

fn describe_bind_error(err: &PdfiumError) -> String {
    match err {
        PdfiumError::LoadLibraryError(e) => e.to_string(),
        other => format!("{other:?}"),
    }
}

#[allow(dead_code)]
fn assert_engine_is_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<Engine>();
}

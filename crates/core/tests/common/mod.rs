//! Helpers shared by the integration tests (each test binary uses part of them).
#![allow(dead_code)]

use nfsetable_core::{dev_pdfium_dir, Engine};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// One PDFium engine per test binary. Needs PDFium: run scripts/fetch-pdfium first.
pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        Engine::new(&[dev_pdfium_dir()]).expect("PDFium not found: run scripts/fetch-pdfium first")
    })
}

/// The folder of the synthetic fixtures.
pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// One synthetic fixture.
pub fn fixture(name: &str) -> PathBuf {
    fixtures().join(name)
}

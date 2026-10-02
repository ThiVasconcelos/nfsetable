//! `nfsetable-core`: reads Brazilian NFS-e PDFs (DANFSe and other layouts) and extracts field values.
//!
//! Everything runs locally: this crate never performs network calls.
//!
//! Conventions shared by the whole API (see [`model`]):
//! - rectangles are in PDF points, origin at the top-left corner of the page, y growing downward
//!   (except the relative `rect` of a region rule, 0..1);
//! - page indices are 0-based;
//! - money is always integer cents (`i64`).
//!
//! Main entry points:
//! - [`Engine`]: binds PDFium and extracts values, reads regions, tests rules and renders pages;
//! - [`scan`]: lists the PDF files of folders/files (no PDFium needed);
//! - [`export_table`]: writes the table as CSV, XLSX, PDF or a PostgreSQL/MySQL script;
//! - [`tax_report`]: MEI, Simples Nacional and Lucro Presumido estimates from the monthly revenue;
//! - [`builtin_fields`] / [`builtin_profiles`]: built-in field definitions and extraction profiles.

pub mod engine;
pub mod error;
pub mod export;
mod extract;
mod layout;
pub mod model;
pub mod parse;
pub mod profile;
mod regex_cache;
pub mod scan;
pub mod tax;
pub mod text;

pub use engine::{dev_pdfium_dir, Engine, PDFIUM_ENV_VAR, PDFIUM_PLATFORM};
pub use error::CoreError;
pub use export::{export_csv, export_table};
pub use model::*;
pub use profile::{
    builtin_fields, builtin_profiles, COMPETENCE_FIELD, NET_VALUE_FIELD, SERVICE_VALUE_FIELD,
};
pub use scan::scan;
pub use tax::{tax_catalog, tax_report};

//! Error type of the core API. Display messages are user facing, in pt-BR.

use pdfium_render::prelude::{PdfiumError, PdfiumInternalError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    /// PDFium could not be found or loaded. The payload lists what was tried.
    #[error("Não foi possível carregar a biblioteca PDFium. {0}")]
    PdfiumUnavailable(String),

    /// The file could not be read from (or written to) disk.
    #[error("Não foi possível acessar o arquivo: {0}")]
    Io(#[from] std::io::Error),

    /// The file is not a valid PDF or is damaged.
    #[error("O arquivo não é um PDF válido ou está corrompido.")]
    InvalidPdf,

    /// The PDF is protected by a password (or by security settings that prevent reading it).
    #[error("O PDF está protegido por senha.")]
    PasswordProtected,

    /// The requested page does not exist. `page` is 0-based; the message shows it 1-based.
    #[error("A página {} não existe (o documento tem {count} página(s)).", .page + 1)]
    PageOutOfRange { page: u32, count: u32 },

    /// A rule or profile is invalid (e.g. a bad regular expression).
    #[error("Regra inválida: {0}")]
    InvalidRule(String),

    /// Any other PDFium failure.
    #[error("Erro ao processar o PDF: {0}")]
    Pdf(String),

    /// The rendered page could not be encoded as an image.
    #[error("Erro ao gerar a imagem da página: {0}")]
    Render(String),

    /// The CSV file could not be written.
    #[error("Erro ao exportar o CSV: {0}")]
    Csv(String),

    /// A table export failed.
    #[error("Erro ao exportar: {0}")]
    Export(String),

    /// The tax estimate could not be computed.
    #[error("Erro no cálculo de impostos: {0}")]
    Tax(String),

    /// Invalid JSON (profiles).
    #[error("JSON inválido: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<PdfiumError> for CoreError {
    fn from(err: PdfiumError) -> Self {
        match err {
            PdfiumError::PdfiumLibraryInternalError(
                PdfiumInternalError::PasswordError | PdfiumInternalError::SecurityError,
            ) => CoreError::PasswordProtected,
            PdfiumError::PdfiumLibraryInternalError(
                PdfiumInternalError::FormatError | PdfiumInternalError::FileError,
            ) => CoreError::InvalidPdf,
            PdfiumError::IoError(io) => CoreError::Io(io),
            PdfiumError::LoadLibraryError(e) => CoreError::PdfiumUnavailable(e.to_string()),
            other => CoreError::Pdf(format!("{other:?}")),
        }
    }
}

impl From<csv::Error> for CoreError {
    fn from(err: csv::Error) -> Self {
        CoreError::Csv(err.to_string())
    }
}

impl From<regex::Error> for CoreError {
    fn from(err: regex::Error) -> Self {
        CoreError::InvalidRule(err.to_string())
    }
}

//! CSV for spreadsheet apps set to pt-BR.

use super::{competence, kind, kind_label, Month, Summary, HEADERS};
use crate::error::CoreError;
use crate::model::ExportRow;
use crate::parse::format_decimal;
use std::borrow::Cow;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

/// The CSV described in [`super::export_csv`].
pub(super) fn to_bytes(rows: &[ExportRow]) -> Result<Vec<u8>, CoreError> {
    let summary = Summary::new(rows);
    let mut buffer = UTF8_BOM.to_vec();
    {
        let mut writer = ::csv::WriterBuilder::new()
            .delimiter(b';')
            .terminator(::csv::Terminator::CRLF)
            .from_writer(&mut buffer);
        writer.write_record(HEADERS)?;
        for row in rows {
            let file = text_field(&row.file);
            let path = text_field(&row.path);
            let doc_type = text_field(&row.doc_type);
            let competence = competence(row).map(Month::display).unwrap_or_default();
            let value = row.cents.map(format_decimal).unwrap_or_default();
            let status = text_field(&row.status);
            let origin = text_field(&row.origin);
            writer.write_record([
                file.as_ref(),
                path.as_ref(),
                doc_type.as_ref(),
                kind_label(kind(row)),
                competence.as_str(),
                value.as_str(),
                status.as_ref(),
                origin.as_ref(),
            ])?;
        }
        for (label, cents) in summary.total_lines() {
            let value = format_decimal(cents);
            writer.write_record([label, "", "", "", "", value.as_str(), "", ""])?;
        }
        writer.flush()?;
    }
    Ok(buffer)
}

/// Spreadsheets run a field starting with `=`, `+`, `-` or `@` as a formula (some also after a
/// leading tab or carriage return); a leading apostrophe keeps such fields as plain text.
fn text_field(value: &str) -> Cow<'_, str> {
    if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        Cow::Owned(format!("'{value}"))
    } else {
        Cow::Borrowed(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutralizes_formula_like_fields() {
        assert_eq!(text_field("=1+1.pdf"), "'=1+1.pdf");
        assert_eq!(text_field("+55 nota.pdf"), "'+55 nota.pdf");
        assert_eq!(text_field("-x.pdf"), "'-x.pdf");
        assert_eq!(text_field("@soma.pdf"), "'@soma.pdf");
        assert_eq!(text_field("nota=1.pdf"), "nota=1.pdf");
        assert_eq!(text_field(""), "");
    }
}

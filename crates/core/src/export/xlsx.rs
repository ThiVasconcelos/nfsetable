//! Excel workbook: the "Notas" sheet (the table, with totals that follow the filter) and the
//! "Resumo" sheet (totals by type and by competence, split by kind when there are expenses).

use super::{
    competence, competence_label, kind, kind_label, truncate_chars, Month, Summary, DISCLAIMER,
    HEADERS, TITLE,
};
use crate::error::CoreError;
use crate::model::{DocKind, ExportRow};
use crate::parse::format_brl;
use rust_xlsxwriter::{
    Color, DocProperties, ExcelDateTime, Format, FormatAlign, FormatBorder, Formula, Workbook,
    Worksheet, XlsxError,
};

/// Brazilian currency, negative amounts in red. `R$` is quoted because `R` is not a literal in
/// number formats; the separators follow the spreadsheet's locale.
const MONEY_FORMAT: &str = "\"R$\" #,##0.00;[Red]-\"R$\" #,##0.00";
const MONTH_FORMAT: &str = "mm/yyyy";
const HEADER_FILL: u32 = 0xDDEBF7;
const TOTAL_FILL: u32 = 0xF2F2F2;
const NOTE_COLOR: u32 = 0x595959;

/// Excel cells hold at most 32 767 characters.
const MAX_CELL_CHARS: usize = 32_767;
/// Rows available for notes: the sheet has 1 048 576 rows, minus the header and up to three
/// total rows.
const MAX_ROWS: usize = 1_048_572;

/// Columns of the "Notas" sheet (see [`HEADERS`]).
const COL_KIND: u16 = 3;
const COL_COMPETENCE: u16 = 4;
const COL_VALUE: u16 = 5;
const LAST_COL: u16 = 7;
/// Hidden helper column, only with expenses: 1 when the row is visible (not filtered out).
const COL_VISIBLE: u16 = 8;

/// The workbook; `company`, when given, goes to the document title and to the top of "Resumo".
pub(super) fn to_bytes(
    rows: &[ExportRow],
    generated_at: &str,
    company: Option<&str>,
) -> Result<Vec<u8>, CoreError> {
    if rows.len() > MAX_ROWS {
        return Err(CoreError::Export(format!(
            "uma planilha comporta no máximo {MAX_ROWS} notas; exporte em CSV ou SQL."
        )));
    }
    build(rows, generated_at, company)
        .map_err(|err| CoreError::Export(format!("não foi possível gerar a planilha ({err}).")))
}

fn build(
    rows: &[ExportRow],
    generated_at: &str,
    company: Option<&str>,
) -> Result<Vec<u8>, XlsxError> {
    let summary = Summary::new(rows);
    let formats = Formats::new();
    let mut workbook = Workbook::new();
    let title = match company {
        Some(company) => format!("{TITLE} — {company}"),
        None => TITLE.to_string(),
    };
    workbook.set_properties(
        &DocProperties::new()
            .set_title(title)
            .set_subject("Notas exportadas pelo nfsetable")
            .set_author("nfsetable")
            .set_comment(DISCLAIMER),
    );
    workbook.push_worksheet(notes_sheet(rows, &summary, &formats)?);
    workbook.push_worksheet(summary_sheet(&summary, generated_at, company, &formats)?);
    workbook.save_to_buffer()
}

struct Formats {
    header: Format,
    month: Format,
    money: Format,
    total_label: Format,
    total_money: Format,
    total_blank: Format,
    title: Format,
    note: Format,
    label: Format,
}

impl Formats {
    fn new() -> Formats {
        let total = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(TOTAL_FILL))
            .set_border_top(FormatBorder::Thin);
        Formats {
            header: Format::new()
                .set_bold()
                .set_background_color(Color::RGB(HEADER_FILL))
                .set_border_bottom(FormatBorder::Thin),
            month: Format::new()
                .set_num_format(MONTH_FORMAT)
                .set_align(FormatAlign::Center),
            money: Format::new().set_num_format(MONEY_FORMAT),
            total_money: total.clone().set_num_format(MONEY_FORMAT),
            total_blank: total.clone(),
            total_label: total,
            title: Format::new().set_bold().set_font_size(14),
            note: Format::new()
                .set_italic()
                .set_font_color(Color::RGB(NOTE_COLOR)),
            label: Format::new().set_bold(),
        }
    }
}

/// The table: header, one row per note, then the total rows, which sum only the rows left
/// visible by the filter. Without expenses: `TOTAL` = `SUBTOTAL(109, …)`. With expenses:
/// `TOTAL RECEITAS` and `TOTAL DESPESAS` = `SUMIFS` of the amounts by natureza over the rows whose
/// hidden helper column (`SUBTOTAL(103, …)` of the row) says they are visible, and `SALDO` = their
/// difference. `SUMIFS` and `SUBTOTAL` work the same in Excel, LibreOffice and Google Sheets.
fn notes_sheet(
    rows: &[ExportRow],
    summary: &Summary,
    formats: &Formats,
) -> Result<Worksheet, XlsxError> {
    let mut sheet = Worksheet::new();
    sheet.set_name("Notas")?;
    for (col, title) in (0..).zip(HEADERS) {
        sheet.write_string_with_format(0, col, title, &formats.header)?;
    }
    let split = summary.has_expenses();
    // `MAX_ROWS` keeps every row index below `u32::MAX`.
    let last_row = rows.len() as u32;
    for (r, row) in (1..).zip(rows) {
        sheet.write_string(r, 0, cell_text(&row.file))?;
        sheet.write_string(r, 1, cell_text(&row.path))?;
        sheet.write_string(r, 2, cell_text(&row.doc_type))?;
        sheet.write_string(r, COL_KIND, kind_label(kind(row)))?;
        if let Some(month) = competence(row) {
            sheet.write_datetime_with_format(r, COL_COMPETENCE, date(month)?, &formats.month)?;
        }
        if let Some(cents) = row.cents {
            sheet.write_number_with_format(r, COL_VALUE, amount(cents), &formats.money)?;
        }
        sheet.write_string(r, 6, cell_text(&row.status))?;
        sheet.write_string(r, 7, cell_text(&row.origin))?;
        if split {
            let visible = Formula::new(format!("=SUBTOTAL(103,D{})", r + 1)).set_result("1");
            sheet.write_formula(r, COL_VISIBLE, visible)?;
        }
    }

    // Excel ranges of the data rows (rows 2 to last_row + 1).
    let range = |col: char| format!("{col}2:{col}{}", last_row + 1);
    let formulas: Vec<String> = if split {
        sheet.write_string(0, COL_VISIBLE, "Linha visível")?;
        sheet.set_column_hidden(COL_VISIBLE)?;
        let sum_of = |kind: DocKind| {
            format!(
                "=SUMIFS({},{},\"{}\",{},1)",
                range('F'),
                range('D'),
                kind_label(kind),
                range('I')
            )
        };
        vec![
            sum_of(DocKind::Revenue),
            sum_of(DocKind::Expense),
            format!("=F{}-F{}", last_row + 2, last_row + 3),
        ]
    } else {
        vec![format!("=SUBTOTAL(109,{})", range('F'))]
    };
    for ((r, (label, cents)), formula) in (last_row + 1..).zip(summary.total_lines()).zip(formulas)
    {
        sheet.write_string_with_format(r, 0, label, &formats.total_label)?;
        for col in [1, 2, COL_KIND, COL_COMPETENCE, 6, 7] {
            sheet.write_blank(r, col, &formats.total_blank)?;
        }
        if rows.is_empty() {
            // `F2:F1` would include the header and the total itself.
            sheet.write_number_with_format(r, COL_VALUE, 0.0, &formats.total_money)?;
        } else {
            let formula =
                Formula::new(formula).set_result(crate::parse::format_decimal_with(cents, '.'));
            sheet.write_formula_with_format(r, COL_VALUE, formula, &formats.total_money)?;
        }
    }

    sheet.set_freeze_panes(1, 0)?;
    // The filter covers the header and the notes, never the total rows.
    sheet.autofilter(0, 0, last_row, LAST_COL)?;

    let widest = |text: fn(&ExportRow) -> &str| {
        rows.iter()
            .map(|row| text(row).chars().count())
            .max()
            .unwrap_or(0)
    };
    let widest_amount = rows
        .iter()
        .filter_map(|row| row.cents)
        .chain(summary.total_lines().into_iter().map(|(_, cents)| cents))
        .map(|cents| format_brl(cents).len())
        .max()
        .unwrap_or(0);
    let widths = [
        column_width(HEADERS[0], widest(|r| &r.file), 60),
        column_width(HEADERS[1], widest(|r| &r.path), 90),
        column_width(HEADERS[2], widest(|r| &r.doc_type), 24),
        column_width(HEADERS[3], 7, 14),
        column_width(HEADERS[4], 7, 16),
        column_width(HEADERS[5], widest_amount, 24),
        column_width(HEADERS[6], widest(|r| &r.status), 32),
        column_width(HEADERS[7], widest(|r| &r.origin), 40),
    ];
    for (col, width) in (0..).zip(widths) {
        sheet.set_column_width(col, width)?;
    }
    Ok(sheet)
}

/// Company (when given), generation date, number of notes, totals, and the totals by type and by
/// competence. With expenses, revenue and expenses are shown apart: totals by natureza and type,
/// and per month the revenue, the expenses and the balance.
fn summary_sheet(
    summary: &Summary,
    generated_at: &str,
    company: Option<&str>,
    formats: &Formats,
) -> Result<Worksheet, XlsxError> {
    let mut sheet = Worksheet::new();
    sheet.set_name("Resumo")?;
    for (col, width) in [(0, 26), (1, 18), (2, 18), (3, 18)] {
        sheet.set_column_width(col, width)?;
    }

    sheet.write_string_with_format(0, 0, "Resumo da exportação", &formats.title)?;
    sheet.write_string_with_format(1, 0, DISCLAIMER, &formats.note)?;

    let mut r = 2;
    if let Some(company) = company {
        r += 1;
        sheet.write_string_with_format(r, 0, "Empresa", &formats.label)?;
        sheet.write_string(r, 1, cell_text(company))?;
    }
    r += 1;
    sheet.write_string_with_format(r, 0, "Gerado em", &formats.label)?;
    sheet.write_string(r, 1, cell_text(generated_at))?;
    r += 1;
    sheet.write_string_with_format(r, 0, "Notas", &formats.label)?;
    sheet.write_number(r, 1, summary.count as f64)?;
    r += 1;
    sheet.write_string_with_format(r, 0, "Notas fora do total", &formats.label)?;
    sheet.write_number(r, 1, summary.without_value as f64)?;
    let totals = &summary.totals;
    let amounts: Vec<(&str, i64)> = if summary.has_expenses() {
        vec![
            ("Receitas", totals.revenue.cents),
            ("Despesas", totals.expense.cents),
            ("Saldo", totals.balance()),
        ]
    } else {
        vec![("Total", totals.revenue.cents)]
    };
    for (label, cents) in amounts {
        r += 1;
        sheet.write_string_with_format(r, 0, label, &formats.label)?;
        sheet.write_number_with_format(r, 1, amount(cents), &formats.money)?;
    }

    r += 2;
    sheet.write_string_with_format(r, 0, "Totais por tipo", &formats.label)?;
    r += 1;
    if summary.has_expenses() {
        header_row(
            &mut sheet,
            r,
            &["Natureza", "Tipo", "Notas", "Total"],
            formats,
        )?;
        for (kind, doc_type, group) in &summary.by_type {
            r += 1;
            sheet.write_string(r, 0, kind_label(*kind))?;
            sheet.write_string(r, 1, cell_text(doc_type))?;
            sheet.write_number(r, 2, group.count as f64)?;
            sheet.write_number_with_format(r, 3, amount(group.cents), &formats.money)?;
        }
    } else {
        header_row(&mut sheet, r, &["Tipo", "Notas", "Total"], formats)?;
        for (_, doc_type, group) in &summary.by_type {
            r += 1;
            sheet.write_string(r, 0, cell_text(doc_type))?;
            sheet.write_number(r, 1, group.count as f64)?;
            sheet.write_number_with_format(r, 2, amount(group.cents), &formats.money)?;
        }
    }

    r += 2;
    sheet.write_string_with_format(r, 0, "Totais por competência", &formats.label)?;
    r += 1;
    if summary.has_expenses() {
        let titles = ["Competência", "Receitas", "Despesas", "Saldo"];
        header_row(&mut sheet, r, &titles, formats)?;
    } else {
        header_row(&mut sheet, r, &["Competência", "Notas", "Total"], formats)?;
    }
    for (month, kinds) in &summary.by_competence {
        r += 1;
        match month {
            Some(month) => {
                sheet.write_datetime_with_format(r, 0, date(*month)?, &formats.month)?;
            }
            None => {
                sheet.write_string(r, 0, competence_label(None))?;
            }
        }
        if summary.has_expenses() {
            let values = [kinds.revenue.cents, kinds.expense.cents, kinds.balance()];
            for (col, cents) in (1..).zip(values) {
                sheet.write_number_with_format(r, col, amount(cents), &formats.money)?;
            }
        } else {
            sheet.write_number(r, 1, kinds.revenue.count as f64)?;
            sheet.write_number_with_format(r, 2, amount(kinds.revenue.cents), &formats.money)?;
        }
    }
    Ok(sheet)
}

fn header_row(
    sheet: &mut Worksheet,
    row: u32,
    titles: &[&str],
    formats: &Formats,
) -> Result<(), XlsxError> {
    for (col, title) in (0..).zip(titles) {
        sheet.write_string_with_format(row, col, *title, &formats.header)?;
    }
    Ok(())
}

/// First day of the competence month.
fn date(month: Month) -> Result<ExcelDateTime, XlsxError> {
    // Competences are limited to the years 1900..=9999 (see `competence`), so they fit.
    let year = u16::try_from(month.year).unwrap_or(1900);
    let number = u8::try_from(month.month).unwrap_or(1);
    ExcelDateTime::from_ymd(year, number, 1)
}

/// Cents as a spreadsheet number (exact for any realistic amount).
fn amount(cents: i64) -> f64 {
    cents as f64 / 100.0
}

/// `text` limited to what a cell can hold.
fn cell_text(text: &str) -> &str {
    truncate_chars(text, MAX_CELL_CHARS)
}

/// Column width in characters: fits the title (plus the filter button) and the longest value,
/// capped at `max`.
fn column_width(title: &str, widest_value: usize, max: usize) -> f64 {
    let title = title.chars().count() + 4;
    (widest_value + 2).max(title).min(max) as f64
}

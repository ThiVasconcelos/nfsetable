//! Table export: CSV, Excel (XLSX), a printable PDF report and SQL scripts for PostgreSQL and
//! MySQL/MariaDB.
//!
//! Every format is built in memory from the same rows and the same totals, then written through a
//! temporary file in the destination folder, so an existing file is only ever replaced by a
//! complete one.
//!
//! Totals are the sum of the rows' `cents`: the app passes `None` for rows left out of the total
//! (duplicates, cancelled notes) and for rows without a value; those rows are listed without one.
//! Revenue and expenses are never summed together: when the table has expense rows, every format
//! shows the revenue total, the expense total and the balance (revenue − expenses) instead of a
//! single total, and splits the totals by type and by competence per kind.

mod csv;
mod helvetica;
mod pdf;
mod sql;
mod xlsx;

use crate::engine::Engine;
use crate::error::CoreError;
use crate::model::{CsvRow, DocKind, ExportFormat, ExportRequest, ExportRow};
use crate::tax::month::Month;
use crate::text::count_label;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Column titles of the tabular formats (CSV and the "Notas" sheet).
const HEADERS: [&str; 8] = [
    "Arquivo",
    "Caminho",
    "Tipo",
    "Natureza",
    "Competência",
    "Valor",
    "Status",
    "Origem",
];

/// Labels of the lines that close the table.
const TOTAL: &str = "TOTAL";
const REVENUE_TOTAL: &str = "TOTAL RECEITAS";
const EXPENSE_TOTAL: &str = "TOTAL DESPESAS";
const BALANCE: &str = "SALDO";

/// Title of the reports.
const TITLE: &str = "Relatório de notas";

/// Reminder shown by the reports.
const DISCLAIMER: &str = "Gerado pelo nfsetable — estimativa, confira com os documentos originais";

/// Label of the rows without a competence in the totals by competence.
const NO_COMPETENCE: &str = "Sem competência";

/// Label of the rows with an empty document type in the totals by type.
const NO_TYPE: &str = "Sem tipo";

/// Writes the table in the requested format to `request.path`.
///
/// `engine` is needed only for PDF (the report is drawn with PDFium); the other formats ignore it.
pub fn export_table(request: &ExportRequest, engine: Option<&Engine>) -> Result<(), CoreError> {
    let path = Path::new(&request.path);
    // Fail before doing any work when the destination is unusable.
    check_destination(path)?;
    let rows = &request.rows;
    let generated_at = request.generated_at.as_str();
    let company = company_name(request);
    let company = company.as_deref();
    let bytes = match request.format {
        // The CSV stays a plain table.
        ExportFormat::Csv => csv::to_bytes(rows)?,
        ExportFormat::Xlsx => xlsx::to_bytes(rows, generated_at, company)?,
        ExportFormat::Pdf => {
            let engine = engine.ok_or_else(|| {
                CoreError::PdfiumUnavailable(
                    "O relatório em PDF é gerado com ela. Os outros formatos (CSV, Excel e SQL) \
                     funcionam sem o PDFium."
                        .to_string(),
                )
            })?;
            let _pdfium = engine.lock();
            pdf::to_bytes(engine, rows, generated_at, company)?
        }
        ExportFormat::Postgresql => {
            sql::to_string(sql::Dialect::Postgresql, rows, generated_at, company).into_bytes()
        }
        ExportFormat::Mysql => {
            sql::to_string(sql::Dialect::Mysql, rows, generated_at, company).into_bytes()
        }
    };
    write_atomically(path, &bytes)
}

/// Company name of the request on a single line (control chars and line breaks become spaces,
/// runs of spaces are collapsed); `None` when missing or blank.
fn company_name(request: &ExportRequest) -> Option<String> {
    let name = one_line(request.company_name.as_deref()?);
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    (!name.is_empty()).then_some(name)
}

/// `text` on a single line: control chars and line or paragraph separators become spaces. Keeps
/// texts from breaking out of a `--` SQL comment.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .collect()
}

/// Writes `rows` to `path` as CSV for spreadsheet apps set to pt-BR: UTF-8 with BOM, `;`
/// delimiter, CRLF line endings, header
/// `Arquivo;Caminho;Tipo;Natureza;Competência;Valor;Status;Origem`, natureza `Receita` or
/// `Despesa`, competence as `MM/AAAA`, amounts with decimal comma and no thousands separator
/// (`1234,56`), fields quoted when needed.
///
/// The table closes with `TOTAL;;;;;<sum of the rows that have cents>;;`, or, when there are
/// expense rows, with the lines `TOTAL RECEITAS`, `TOTAL DESPESAS` and `SALDO` (receitas −
/// despesas) in the same shape.
///
/// Text fields starting with `=`, `+`, `-` or `@` get a leading apostrophe, so spreadsheets do not
/// run them as formulas.
pub fn export_csv(rows: &[CsvRow], path: &Path) -> Result<(), CoreError> {
    write_atomically(path, &csv::to_bytes(rows)?)
}

// ---------------------------------------------------------------- shared data

/// The competence month of `text` (`yyyy-mm` or `yyyy-mm-dd`). Years outside 1900..=9999, the
/// range spreadsheets can show, are left out.
fn competence_month(text: &str) -> Option<Month> {
    Month::parse_month_or_date(text).filter(|month| (1900..=9999).contains(&month.year))
}

/// Parsed competence of a row.
fn competence(row: &ExportRow) -> Option<Month> {
    row.competence.as_deref().and_then(competence_month)
}

/// Revenue or expense of a row; rows without a kind are revenue.
fn kind(row: &ExportRow) -> DocKind {
    row.kind.unwrap_or_default()
}

/// "Receita" / "Despesa".
fn kind_label(kind: DocKind) -> &'static str {
    match kind {
        DocKind::Revenue => "Receita",
        DocKind::Expense => "Despesa",
    }
}

/// Rows and amount of a group of rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Totals {
    /// Rows of the group, with or without a value.
    count: usize,
    cents: i64,
}

impl Totals {
    fn add(&mut self, cents: Option<i64>) {
        self.count += 1;
        self.cents = self.cents.saturating_add(cents.unwrap_or(0));
    }
}

/// Revenue and expense totals of a group, kept apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct KindTotals {
    revenue: Totals,
    expense: Totals,
}

impl KindTotals {
    fn of_mut(&mut self, kind: DocKind) -> &mut Totals {
        match kind {
            DocKind::Revenue => &mut self.revenue,
            DocKind::Expense => &mut self.expense,
        }
    }

    /// Revenue minus expenses.
    fn balance(&self) -> i64 {
        self.revenue.cents.saturating_sub(self.expense.cents)
    }
}

/// Totals shared by every format.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Summary {
    count: usize,
    /// Rows without an amount: left out of the total by the app or without a value.
    without_value: usize,
    totals: KindTotals,
    /// Per kind (revenue first), then per document type in alphabetical order (see
    /// [`type_label`]).
    by_type: Vec<(DocKind, String, Totals)>,
    /// Per competence month, oldest first; rows without a competence last (`None`).
    by_competence: Vec<(Option<Month>, KindTotals)>,
}

impl Summary {
    fn new(rows: &[ExportRow]) -> Summary {
        let mut totals = KindTotals::default();
        let mut without_value = 0;
        // Keyed by (is expense, type): revenue first.
        let mut by_type: BTreeMap<(bool, &str), Totals> = BTreeMap::new();
        let mut by_month: BTreeMap<Month, KindTotals> = BTreeMap::new();
        let mut unknown_month = KindTotals::default();
        for row in rows {
            let kind = kind(row);
            if row.cents.is_none() {
                without_value += 1;
            }
            totals.of_mut(kind).add(row.cents);
            by_type
                .entry((kind == DocKind::Expense, type_label(&row.doc_type)))
                .or_default()
                .add(row.cents);
            match competence(row) {
                Some(month) => by_month.entry(month).or_default(),
                None => &mut unknown_month,
            }
            .of_mut(kind)
            .add(row.cents);
        }
        let mut by_competence: Vec<(Option<Month>, KindTotals)> = by_month
            .into_iter()
            .map(|(month, totals)| (Some(month), totals))
            .collect();
        if unknown_month.revenue.count + unknown_month.expense.count > 0 {
            by_competence.push((None, unknown_month));
        }
        Summary {
            count: rows.len(),
            without_value,
            totals,
            by_type: by_type
                .into_iter()
                .map(|((expense, doc_type), totals)| {
                    let kind = if expense {
                        DocKind::Expense
                    } else {
                        DocKind::Revenue
                    };
                    (kind, doc_type.to_string(), totals)
                })
                .collect(),
            by_competence,
        }
    }

    /// With expense rows, the reports show revenue, expenses and balance instead of one total.
    fn has_expenses(&self) -> bool {
        self.totals.expense.count > 0
    }

    /// Lines that close the table: `TOTAL`, or `TOTAL RECEITAS`, `TOTAL DESPESAS` and `SALDO`
    /// when there are expense rows.
    fn total_lines(&self) -> Vec<(&'static str, i64)> {
        if self.has_expenses() {
            vec![
                (REVENUE_TOTAL, self.totals.revenue.cents),
                (EXPENSE_TOTAL, self.totals.expense.cents),
                (BALANCE, self.totals.balance()),
            ]
        } else {
            vec![(TOTAL, self.totals.revenue.cents)]
        }
    }

    /// "40 notas", or "40 notas: 32 receitas e 8 despesas" when there are expense rows.
    fn count_text(&self) -> String {
        let notes = count_label(self.count, "nota", "notas");
        if self.has_expenses() {
            format!(
                "{notes}: {} e {}",
                count_label(self.totals.revenue.count, "receita", "receitas"),
                count_label(self.totals.expense.count, "despesa", "despesas")
            )
        } else {
            notes
        }
    }
}

/// Document type as shown in the totals.
fn type_label(doc_type: &str) -> &str {
    match doc_type.trim() {
        "" => NO_TYPE,
        label => label,
    }
}

/// Competence as shown in the totals: `MM/AAAA` or "Sem competência".
fn competence_label(month: Option<Month>) -> String {
    month.map_or_else(|| NO_COMPETENCE.to_string(), Month::display)
}

/// The first `max` characters of `text` (SQL column sizes, the spreadsheet cell limit).
fn truncate_chars(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((end, _)) => &text[..end],
        None => text,
    }
}

// ---------------------------------------------------------------- writing

/// Checks that `path` names a file inside an existing folder. Returns the folder.
fn check_destination(path: &Path) -> Result<PathBuf, CoreError> {
    if path.as_os_str().is_empty() {
        return Err(CoreError::Export(
            "nenhum arquivo de destino foi informado.".to_string(),
        ));
    }
    if path.file_name().is_none() || path.is_dir() {
        return Err(CoreError::Export(format!(
            "o destino precisa ser um arquivo, não uma pasta: {}",
            path.display()
        )));
    }
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    };
    if !dir.is_dir() {
        return Err(CoreError::Export(format!(
            "a pasta de destino não existe: {}",
            dir.display()
        )));
    }
    Ok(dir)
}

/// Distinguishes the temporary files of concurrent exports.
static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Writes `bytes` to `path` through a temporary file in the same folder, renamed over `path` only
/// once it is complete and flushed to disk.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let dir = check_destination(path)?;
    let name = crate::scan::file_name(path);
    let (temp_path, mut file) = create_temp_file(&dir, &name)?;
    let written = file.write_all(bytes).and_then(|()| file.sync_all());
    drop(file);
    if let Err(err) = written {
        let _ = fs::remove_file(&temp_path);
        return Err(CoreError::Io(err));
    }
    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(replace_error(path, err));
    }
    Ok(())
}

fn create_temp_file(dir: &Path, name: &str) -> Result<(PathBuf, File), CoreError> {
    // A prefix of the name keeps the temporary name within the file system limits.
    let prefix: String = name.chars().take(64).collect();
    let mut attempts = 0;
    loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_path = dir.join(format!(".{prefix}.{}-{n}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists && attempts < 16 => {
                attempts += 1;
            }
            Err(err) => return Err(CoreError::Io(err)),
        }
    }
}

/// Windows refuses to replace a file that another program keeps open (Excel does).
fn replace_error(path: &Path, err: io::Error) -> CoreError {
    let locked = err.kind() == io::ErrorKind::PermissionDenied
        || (cfg!(windows) && matches!(err.raw_os_error(), Some(32 | 33)));
    if locked {
        CoreError::Export(format!(
            "não foi possível substituir {}. Se o arquivo estiver aberto em outro programa \
             (como o Excel), feche-o e tente de novo. ({err})",
            path.display()
        ))
    } else {
        CoreError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        kind: Option<DocKind>,
        doc_type: &str,
        competence: Option<&str>,
        cents: Option<i64>,
    ) -> ExportRow {
        ExportRow {
            file: "a.pdf".into(),
            path: "C:/Notas/a.pdf".into(),
            doc_type: doc_type.into(),
            kind,
            competence: competence.map(Into::into),
            cents,
            status: "OK".into(),
            origin: String::new(),
        }
    }

    fn totals(count: usize, cents: i64) -> Totals {
        Totals { count, cents }
    }

    #[test]
    fn normalizes_the_company_name() {
        let company = |name: Option<&str>| {
            company_name(&ExportRequest {
                format: ExportFormat::Csv,
                path: String::new(),
                rows: Vec::new(),
                generated_at: String::new(),
                company_name: name.map(Into::into),
            })
        };
        assert_eq!(company(None), None);
        assert_eq!(company(Some(" \t\n ")), None);
        assert_eq!(
            company(Some(" Consultoria  Fictícia\r\nLtda\u{2028}- ME ")).as_deref(),
            Some("Consultoria Fictícia Ltda - ME")
        );
        assert_eq!(
            one_line("29/09/2026\n14:05\r\u{2029}x"),
            "29/09/2026 14:05  x"
        );
    }

    #[test]
    fn parses_competence_months() {
        let march = Month {
            year: 2026,
            month: 3,
        };
        assert_eq!(competence_month("2026-03"), Some(march));
        assert_eq!(competence_month(" 2026-03-15 "), Some(march));
        for bad in [
            "2026-13",
            "2026-00",
            "1899-12",
            "03/2026",
            "2026-3",
            "+202-03",
            "2026-03-1x",
            "",
        ] {
            assert_eq!(competence_month(bad), None, "{bad}");
        }
        assert_eq!(march.display(), "03/2026");
    }

    #[test]
    fn summary_groups_by_type_and_month() {
        let revenue = Some(DocKind::Revenue);
        let rows = [
            row(None, "PDF", Some("2026-04"), Some(1_000)),
            row(revenue, "NFS-e", Some("2026-03"), Some(123_456)),
            row(None, "NFS-e", None, None),
            row(revenue, "NFS-e", Some("2026-04"), Some(-50)),
            row(None, " ", Some("2025-12"), Some(7)),
        ];
        let summary = Summary::new(&rows);
        assert_eq!(summary.count, 5);
        assert_eq!(summary.without_value, 1);
        assert!(!summary.has_expenses());
        assert_eq!(summary.totals.revenue, totals(5, 124_413));
        assert_eq!(summary.total_lines(), [("TOTAL", 124_413)]);
        assert_eq!(summary.count_text(), "5 notas");
        let types: Vec<(DocKind, &str, Totals)> = summary
            .by_type
            .iter()
            .map(|(kind, doc_type, totals)| (*kind, doc_type.as_str(), *totals))
            .collect();
        assert_eq!(
            types,
            [
                (DocKind::Revenue, "NFS-e", totals(3, 123_406)),
                (DocKind::Revenue, "PDF", totals(1, 1_000)),
                (DocKind::Revenue, "Sem tipo", totals(1, 7)),
            ]
        );
        let months: Vec<(String, Totals)> = summary
            .by_competence
            .iter()
            .map(|(month, kinds)| (competence_label(*month), kinds.revenue))
            .collect();
        assert_eq!(
            months,
            [
                ("12/2025".to_string(), totals(1, 7)),
                ("03/2026".to_string(), totals(1, 123_456)),
                ("04/2026".to_string(), totals(2, 950)),
                ("Sem competência".to_string(), totals(1, 0)),
            ]
        );
    }

    #[test]
    fn summary_keeps_revenue_and_expenses_apart() {
        let expense = Some(DocKind::Expense);
        let rows = [
            row(expense, "Boleto", Some("2026-03"), Some(20_000)),
            row(None, "NFS-e", Some("2026-03"), Some(150_000)),
            row(expense, "Boleto", None, None),
            row(expense, "Recibo", Some("2026-04"), Some(5_000)),
            row(
                Some(DocKind::Revenue),
                "NFS-e",
                Some("2026-04"),
                Some(100_000),
            ),
        ];
        let summary = Summary::new(&rows);
        assert!(summary.has_expenses());
        assert_eq!(summary.totals.revenue, totals(2, 250_000));
        assert_eq!(summary.totals.expense, totals(3, 25_000));
        assert_eq!(summary.totals.balance(), 225_000);
        assert_eq!(
            summary.total_lines(),
            [
                ("TOTAL RECEITAS", 250_000),
                ("TOTAL DESPESAS", 25_000),
                ("SALDO", 225_000)
            ]
        );
        assert_eq!(summary.count_text(), "5 notas: 2 receitas e 3 despesas");
        let types: Vec<(DocKind, &str, Totals)> = summary
            .by_type
            .iter()
            .map(|(kind, doc_type, totals)| (*kind, doc_type.as_str(), *totals))
            .collect();
        assert_eq!(
            types,
            [
                (DocKind::Revenue, "NFS-e", totals(2, 250_000)),
                (DocKind::Expense, "Boleto", totals(2, 20_000)),
                (DocKind::Expense, "Recibo", totals(1, 5_000)),
            ]
        );
        let months: Vec<(String, i64, i64, i64)> = summary
            .by_competence
            .iter()
            .map(|(month, kinds)| {
                (
                    competence_label(*month),
                    kinds.revenue.cents,
                    kinds.expense.cents,
                    kinds.balance(),
                )
            })
            .collect();
        assert_eq!(
            months,
            [
                ("03/2026".to_string(), 150_000, 20_000, 130_000),
                ("04/2026".to_string(), 100_000, 5_000, 95_000),
                ("Sem competência".to_string(), 0, 0, 0),
            ]
        );
        assert_eq!(kind_label(kind(&rows[1])), "Receita");
        assert_eq!(kind_label(kind(&rows[0])), "Despesa");
    }

    #[test]
    fn rejects_unusable_destinations() {
        let dir = std::env::temp_dir();
        assert!(matches!(
            check_destination(Path::new("")),
            Err(CoreError::Export(_))
        ));
        assert!(matches!(check_destination(&dir), Err(CoreError::Export(_))));
        let missing = dir.join("nfsetable-pasta-que-nao-existe").join("a.csv");
        assert!(matches!(
            check_destination(&missing),
            Err(CoreError::Export(_))
        ));
        assert_eq!(check_destination(&dir.join("a.csv")).unwrap(), dir);
    }
}

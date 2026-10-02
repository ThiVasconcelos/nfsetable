//! Table export in every format: CSV, XLSX, PDF report and SQL (PostgreSQL, MySQL/MariaDB).
//! The PDF tests need PDFium: run scripts/fetch-pdfium first. Every name and amount is made up.

use base64::Engine as _;
use nfsetable_core::{
    export_csv, export_table, CoreError, DocKind, ExportFormat, ExportRequest, ExportRow,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const GENERATED_AT: &str = "29/09/2026 14:05";

mod common;
use common::engine;

/// A revenue row (no kind: rows without one count as revenue).
fn row(
    file: &str,
    path: &str,
    doc_type: &str,
    competence: Option<&str>,
    cents: Option<i64>,
    status: &str,
    origin: &str,
) -> ExportRow {
    ExportRow {
        file: file.into(),
        path: path.into(),
        doc_type: doc_type.into(),
        kind: None,
        competence: competence.map(Into::into),
        cents,
        status: status.into(),
        origin: origin.into(),
    }
}

fn expense(mut row: ExportRow) -> ExportRow {
    row.kind = Some(DocKind::Expense);
    row
}

/// Three revenue rows; total R$ 1.224,06.
fn sample_rows() -> Vec<ExportRow> {
    vec![
        row(
            "NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
            "C:\\Notas\\2026\\NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
            "NFS-e",
            Some("2026-03"),
            Some(123_456),
            "OK",
            "DANFSe (padrão nacional)",
        ),
        row(
            "Serviço de manutenção — março.pdf",
            "C:\\Notas\\2026\\Serviço de manutenção — março.pdf",
            "PDF",
            Some("2026-04"),
            Some(-1_050),
            "OK",
            "Manual",
        ),
        row(
            "recibo sem valor.pdf",
            "C:\\Notas\\recibo sem valor.pdf",
            "PDF",
            None,
            None,
            "Não encontrado",
            "",
        ),
    ]
}

/// Two revenue rows and three expense rows (one without value). Receitas R$ 23.000,00,
/// despesas R$ 2.956,78, saldo R$ 20.043,22; March: 15.000,00 − 2.500,00 = 12.500,00; April:
/// 8.000,00 − 456,78 = 7.543,22.
fn mixed_rows() -> Vec<ExportRow> {
    let mut explicit_revenue = row(
        "NFS-e 0002 COMÉRCIO FICTÍCIO S.A.pdf",
        "C:\\Notas\\2026\\NFS-e 0002 COMÉRCIO FICTÍCIO S.A.pdf",
        "NFS-e",
        Some("2026-04"),
        Some(800_000),
        "OK",
        "DANFSe (padrão nacional)",
    );
    explicit_revenue.kind = Some(DocKind::Revenue);
    vec![
        row(
            "NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
            "C:\\Notas\\2026\\NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
            "NFS-e",
            Some("2026-03"),
            Some(1_500_000),
            "OK",
            "DANFSe (padrão nacional)",
        ),
        expense(row(
            "Boleto aluguel março.pdf",
            "C:\\Despesas\\Boleto aluguel março.pdf",
            "Boleto",
            Some("2026-03"),
            Some(250_000),
            "OK",
            "Manual",
        )),
        explicit_revenue,
        expense(row(
            "Conta de energia abril.pdf",
            "C:\\Despesas\\Conta de energia abril.pdf",
            "Conta",
            Some("2026-04"),
            Some(45_678),
            "OK",
            "Automático",
        )),
        expense(row(
            "Boleto aluguel março (cópia).pdf",
            "C:\\Despesas\\Boleto aluguel março (cópia).pdf",
            "Boleto",
            Some("2026-03"),
            None,
            "Duplicada",
            "Manual",
        )),
    ]
}

/// `count` varied rows: six months, several types, duplicates and cancelled notes without value
/// and, with `with_expenses`, one bill (expense) every five rows.
fn synthetic_rows(count: usize, with_expenses: bool) -> Vec<ExportRow> {
    const CLIENTS: [&str; 5] = [
        "EMPRESA EXEMPLO LTDA",
        "COMÉRCIO FICTÍCIO S.A.",
        "SERVIÇOS DEMONSTRAÇÃO ME",
        "INDÚSTRIA MODELO EIRELI",
        "ASSOCIAÇÃO AMOSTRA",
    ];
    const SUPPLIERS: [&str; 3] = [
        "ENERGIA EXEMPLO S.A.",
        "IMOBILIÁRIA FICTÍCIA LTDA",
        "CONTABILIDADE MODELO ME",
    ];
    (1..=count)
        .map(|i| {
            let month = i % 6 + 1;
            let is_expense = with_expenses && i % 5 == 0;
            let (file, doc_type, kind, origin) = if is_expense {
                let supplier = SUPPLIERS[i % SUPPLIERS.len()];
                (
                    format!("Boleto {i:04} - {supplier} - 2026-{month:02}.pdf"),
                    "Boleto",
                    Some(DocKind::Expense),
                    "Manual",
                )
            } else {
                let client = CLIENTS[i % CLIENTS.len()];
                let file = format!("NFS-e {i:04} - {client} - 2026-{month:02}.pdf");
                // Revenue rows alternate between an explicit kind and none.
                let kind = (i % 2 == 0).then_some(DocKind::Revenue);
                if i % 4 == 0 {
                    (file, "PDF", kind, "Automático")
                } else {
                    (file, "NFS-e", kind, "DANFSe (padrão nacional)")
                }
            };
            let (status, cents) = match i % 13 {
                0 => ("Duplicada", None),
                7 => ("Cancelada", None),
                _ if is_expense => ("OK", Some((i as i64 * 3_217) % 120_000 + 4_500)),
                _ => ("OK", Some((i as i64 * 7_919) % 900_000 + 15_000)),
            };
            let folder = if is_expense { "Despesas" } else { "Notas" };
            ExportRow {
                path: format!("C:\\{folder}\\2026\\{file}"),
                file,
                doc_type: doc_type.into(),
                kind,
                competence: (i % 17 != 0).then(|| format!("2026-{month:02}")),
                cents,
                status: status.into(),
                origin: origin.into(),
            }
        })
        .collect()
}

fn request(format: ExportFormat, path: &Path, rows: Vec<ExportRow>) -> ExportRequest {
    ExportRequest {
        format,
        path: path.to_string_lossy().into_owned(),
        rows,
        generated_at: GENERATED_AT.into(),
        company_name: None,
    }
}

/// Exports `rows` into `dir/name` and returns the file content.
fn export(format: ExportFormat, dir: &Path, name: &str, rows: Vec<ExportRow>) -> Vec<u8> {
    export_for(None, format, dir, name, rows)
}

/// Made-up company name for the reports that show it.
const COMPANY: &str = "Consultoria Fictícia Ltda";

/// Like [`export`], for the given company.
fn export_for(
    company: Option<&str>,
    format: ExportFormat,
    dir: &Path,
    name: &str,
    rows: Vec<ExportRow>,
) -> Vec<u8> {
    let path = dir.join(name);
    let mut request = request(format, &path, rows);
    request.company_name = company.map(Into::into);
    export_table(&request, Some(engine())).unwrap();
    std::fs::read(&path).unwrap()
}

fn export_text(format: ExportFormat, rows: Vec<ExportRow>) -> String {
    let dir = tempfile::tempdir().unwrap();
    String::from_utf8(export(format, dir.path(), "notas.txt", rows)).unwrap()
}

/// CSV content after the BOM.
fn export_csv_text(rows: Vec<ExportRow>) -> String {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export(ExportFormat::Csv, dir.path(), "notas.csv", rows);
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    String::from_utf8(bytes[3..].to_vec()).unwrap()
}

/// Names of the files in `dir`.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// ---------------------------------------------------------------- CSV

#[test]
fn csv_has_bom_header_natureza_competence_and_total() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export(ExportFormat::Csv, dir.path(), "notas.csv", sample_rows());
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
    assert!(text.ends_with("\r\n"));
    let lines: Vec<&str> = text.split("\r\n").collect();
    assert_eq!(
        lines,
        [
            "Arquivo;Caminho;Tipo;Natureza;Competência;Valor;Status;Origem",
            "NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf;C:\\Notas\\2026\\NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf;NFS-e;Receita;03/2026;1234,56;OK;DANFSe (padrão nacional)",
            "Serviço de manutenção — março.pdf;C:\\Notas\\2026\\Serviço de manutenção — março.pdf;PDF;Receita;04/2026;-10,50;OK;Manual",
            "recibo sem valor.pdf;C:\\Notas\\recibo sem valor.pdf;PDF;Receita;;;Não encontrado;",
            "TOTAL;;;;;1224,06;;",
            "",
        ]
    );
    // Written through a temporary file that is renamed: nothing else is left behind.
    assert_eq!(listing(dir.path()), ["notas.csv"]);
}

#[test]
fn csv_splits_revenue_and_expenses() {
    let text = export_csv_text(mixed_rows());
    let lines: Vec<&str> = text.split("\r\n").collect();
    assert_eq!(
        lines[2],
        "Boleto aluguel março.pdf;C:\\Despesas\\Boleto aluguel março.pdf;Boleto;Despesa;03/2026;2500,00;OK;Manual"
    );
    assert!(
        lines[3].contains(";NFS-e;Receita;04/2026;8000,00;"),
        "{}",
        lines[3]
    );
    assert_eq!(
        lines[6..],
        [
            "TOTAL RECEITAS;;;;;23000,00;;",
            "TOTAL DESPESAS;;;;;2956,78;;",
            "SALDO;;;;;20043,22;;",
            "",
        ]
    );
    assert!(!text.contains("TOTAL;"));
}

#[test]
fn csv_quotes_special_fields_and_neutralizes_formulas() {
    let rows = vec![
        row(
            "nota; com separador.pdf",
            "C:\\Notas\\nota; com separador.pdf",
            "NFS-e",
            Some("2026-01-15"),
            Some(5),
            "OK",
            "linha 1\nlinha 2",
        ),
        row(
            "aspas \"duplas\".pdf",
            "C:\\Notas\\aspas \"duplas\".pdf",
            "PDF",
            Some("inválida"),
            Some(100_000_000),
            "OK",
            "",
        ),
        row(
            "=HIPERLINK(\"x\").pdf",
            "-x.pdf",
            "PDF",
            None,
            None,
            "@status",
            "+origem",
        ),
    ];
    let text = export_csv_text(rows);
    for expected in [
        "\"nota; com separador.pdf\";\"C:\\Notas\\nota; com separador.pdf\";NFS-e;Receita;01/2026;0,05;OK;\"linha 1\nlinha 2\"\r\n",
        "\"aspas \"\"duplas\"\".pdf\";\"C:\\Notas\\aspas \"\"duplas\"\".pdf\";PDF;Receita;;1000000,00;OK;\r\n",
        "\"'=HIPERLINK(\"\"x\"\").pdf\";'-x.pdf;PDF;Receita;;;'@status;'+origem\r\n",
        "TOTAL;;;;;1000000,05;;\r\n",
    ] {
        assert!(text.contains(expected), "{expected:?} not in {text:?}");
    }
}

#[test]
fn csv_ignores_the_company() {
    let dir = tempfile::tempdir().unwrap();
    let with = export_for(
        Some(COMPANY),
        ExportFormat::Csv,
        dir.path(),
        "a.csv",
        mixed_rows(),
    );
    let without = export(ExportFormat::Csv, dir.path(), "b.csv", mixed_rows());
    assert_eq!(with, without);
}

#[test]
fn csv_replaces_an_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notas.csv");
    std::fs::write(&path, "conteúdo antigo").unwrap();
    export_csv(&sample_rows()[..1], &path).unwrap();
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    assert!(text.contains("TOTAL;;;;;1234,56;;"), "{text}");
    assert!(!text.contains("antigo"));
    assert_eq!(listing(dir.path()), ["notas.csv"]);
}

#[test]
fn export_rejects_a_missing_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("não existe").join("notas.csv");
    let err = export_table(&request(ExportFormat::Csv, &path, sample_rows()), None).unwrap_err();
    assert!(matches!(err, CoreError::Export(_)), "{err:?}");
    assert!(err.to_string().contains("pasta de destino"), "{err}");
}

// ---------------------------------------------------------------- XLSX

/// Entries of the exported workbook, with `&quot;` decoded so formulas read as typed.
fn xlsx_files(rows: Vec<ExportRow>) -> BTreeMap<String, String> {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export(ExportFormat::Xlsx, dir.path(), "notas.xlsx", rows);
    assert!(bytes.starts_with(b"PK\x03\x04"), "not a zip file");
    zip::read(&bytes)
        .into_iter()
        .map(|(name, content)| {
            let text = String::from_utf8(content).unwrap();
            (name, text.replace("&quot;", "\""))
        })
        .collect()
}

#[test]
fn xlsx_has_notes_and_summary_sheets() {
    let files = xlsx_files(sample_rows());

    let workbook = &files["xl/workbook.xml"];
    assert!(workbook.contains("name=\"Notas\""), "{workbook}");
    assert!(workbook.contains("name=\"Resumo\""), "{workbook}");

    let notes = &files["xl/worksheets/sheet1.xml"];
    // TOTAL row: SUBTOTAL(109, …) follows the filter; the cached result is set.
    assert!(
        notes.contains("<f>SUBTOTAL(109,F2:F4)</f><v>1224.06</v>"),
        "{notes}"
    );
    // Revenue only: no split totals nor helper column.
    assert!(
        !notes.contains("SUMIFS") && !notes.contains("hidden"),
        "{notes}"
    );
    // The filter covers the header and the notes, not the TOTAL row; the header is frozen.
    assert!(notes.contains("<autoFilter ref=\"A1:H4\""), "{notes}");
    assert!(notes.contains("state=\"frozen\""), "{notes}");
    // Competence 2026-03 as the date 2026-03-01 (Excel serial 46082); amounts as numbers.
    assert!(notes.contains("<v>46082</v>"), "{notes}");
    assert!(notes.contains("<v>1234.56</v>"), "{notes}");
    assert!(notes.contains("<v>-10.5</v>"), "{notes}");

    let strings = &files["xl/sharedStrings.xml"];
    for expected in [
        "Arquivo",
        "Natureza",
        "Receita",
        "Competência",
        "TOTAL",
        "Serviço de manutenção — março.pdf",
        "C:\\Notas\\2026\\NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
        "Totais por tipo",
        "Totais por competência",
        "Sem competência",
        "Gerado pelo nfsetable — estimativa, confira com os documentos originais",
        GENERATED_AT,
    ] {
        assert!(
            strings.contains(&format!(">{expected}<")),
            "{expected} not in {strings}"
        );
    }
    assert!(!strings.contains(">Despesa<") && !strings.contains(">SALDO<"));
    // No company given: no "Empresa" line.
    assert!(!strings.contains(">Empresa<"), "{strings}");

    let styles = &files["xl/styles.xml"];
    assert!(styles.contains("formatCode=\"mm/yyyy\""), "{styles}");
    assert!(
        styles.contains("formatCode=\"\"R$\" #,##0.00;[Red]-\"R$\" #,##0.00\""),
        "{styles}"
    );

    let summary = &files["xl/worksheets/sheet2.xml"];
    // Number of notes, total, and the totals by type (NFS-e 1234.56, PDF -10.50).
    assert!(summary.contains("<v>3</v>"), "{summary}");
    assert!(summary.contains("<v>1224.06</v>"), "{summary}");
    assert!(summary.contains("<v>-10.5</v>"), "{summary}");

    let core = &files["docProps/core.xml"];
    assert!(
        core.contains("<dc:title>Relatório de notas</dc:title>"),
        "{core}"
    );
    assert!(
        core.contains("<dc:creator>nfsetable</dc:creator>"),
        "{core}"
    );
}

#[test]
fn xlsx_splits_revenue_and_expenses() {
    let files = xlsx_files(mixed_rows());
    let notes = &files["xl/worksheets/sheet1.xml"];
    // Rows 2 to 6 are the notes; the hidden column I tells whether each one is visible.
    for r in 2..=6 {
        let helper = format!("<f>SUBTOTAL(103,D{r})</f><v>1</v>");
        assert!(notes.contains(&helper), "{helper} not in {notes}");
    }
    let helper_column = notes
        .split("<col ")
        .find(|col| col.starts_with("min=\"9\" max=\"9\""))
        .expect("column I");
    assert!(helper_column.contains("hidden=\"1\""), "{helper_column}");
    // Totals by natureza over the visible rows, then the balance.
    for expected in [
        "<f>SUMIFS(F2:F6,D2:D6,\"Receita\",I2:I6,1)</f><v>23000.00</v>",
        "<f>SUMIFS(F2:F6,D2:D6,\"Despesa\",I2:I6,1)</f><v>2956.78</v>",
        "<f>F7-F8</f><v>20043.22</v>",
        "<autoFilter ref=\"A1:H6\"",
    ] {
        assert!(notes.contains(expected), "{expected} not in {notes}");
    }
    assert!(!notes.contains("SUBTOTAL(109"), "{notes}");

    let strings = &files["xl/sharedStrings.xml"];
    for expected in [
        "Despesa",
        "Receita",
        "TOTAL RECEITAS",
        "TOTAL DESPESAS",
        "SALDO",
        "Receitas",
        "Despesas",
        "Saldo",
    ] {
        assert!(
            strings.contains(&format!(">{expected}<")),
            "{expected} not in {strings}"
        );
    }
    assert!(!strings.contains(">TOTAL<"));

    // Resumo: receitas, despesas and saldo, and per month the balance (12500 and 7543.22).
    let summary = &files["xl/worksheets/sheet2.xml"];
    for expected in [
        "<v>23000</v>",
        "<v>2956.78</v>",
        "<v>20043.22</v>",
        "<v>12500</v>",
        "<v>7543.22</v>",
    ] {
        assert!(summary.contains(expected), "{expected} not in {summary}");
    }
}

#[test]
fn xlsx_shows_the_company() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export_for(
        Some(COMPANY),
        ExportFormat::Xlsx,
        dir.path(),
        "notas.xlsx",
        sample_rows(),
    );
    let files: BTreeMap<String, String> = zip::read(&bytes)
        .into_iter()
        .map(|(name, content)| (name, String::from_utf8(content).unwrap()))
        .collect();
    let strings = &files["xl/sharedStrings.xml"];
    assert!(strings.contains(">Empresa<"), "{strings}");
    assert!(strings.contains(&format!(">{COMPANY}<")), "{strings}");
    // "Empresa" is the first line of the summary, right below the title and the note.
    let summary = &files["xl/worksheets/sheet2.xml"];
    assert!(summary.contains("<c r=\"A4\" s="), "{summary}");
    assert!(summary.contains("<c r=\"B4\" t=\"s\">"), "{summary}");
    let core = &files["docProps/core.xml"];
    assert!(
        core.contains(&format!(
            "<dc:title>Relatório de notas — {COMPANY}</dc:title>"
        )),
        "{core}"
    );
}

#[test]
fn xlsx_without_rows_has_a_plain_total() {
    let files = xlsx_files(Vec::new());
    let notes = &files["xl/worksheets/sheet1.xml"];
    assert!(!notes.contains("SUBTOTAL"), "{notes}");
    assert!(notes.contains("<autoFilter ref=\"A1:H1\""), "{notes}");
    assert!(files["xl/sharedStrings.xml"].contains(">TOTAL<"));
}

// ---------------------------------------------------------------- PDF

/// Text of each page of the PDF at `path`.
fn pdf_pages(path: &Path) -> Vec<String> {
    let _pdfium = engine().lock();
    let document = engine().pdfium().load_pdf_from_file(path, None).unwrap();
    document
        .pages()
        .iter()
        .map(|page| page.text().unwrap().all())
        .collect()
}

fn export_pdf(rows: Vec<ExportRow>) -> (tempfile::TempDir, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export(ExportFormat::Pdf, dir.path(), "notas.pdf", rows);
    assert!(bytes.starts_with(b"%PDF-"));
    let pages = pdf_pages(&dir.path().join("notas.pdf"));
    (dir, pages)
}

#[test]
fn pdf_report_has_title_names_totals_and_page_numbers() {
    let (_dir, pages) = export_pdf(sample_rows());
    assert_eq!(pages.len(), 1);
    let text = &pages[0];
    for expected in [
        "Relatório de notas",
        "Gerado em 29/09/2026 14:05 pelo nfsetable",
        "3 notas · Total R$ 1.224,06 · 1 fora do total",
        "NFS-e 0001 EMPRESA EXEMPLO LTDA.pdf",
        "Serviço de manutenção — março.pdf",
        "Não encontrado",
        "Competência",
        "03/2026",
        "R$ 1.234,56",
        "-R$ 10,50",
        "TOTAL",
        "R$ 1.224,06",
        "Totais por tipo",
        "Totais por competência",
        "Sem competência",
        "Página 1 de 1",
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
    // Revenue only: the simpler layout, without the natureza.
    assert!(
        !text.contains("Natureza") && !text.contains("SALDO"),
        "{text}"
    );
}

#[test]
fn pdf_report_splits_revenue_and_expenses() {
    let (_dir, pages) = export_pdf(mixed_rows());
    assert_eq!(pages.len(), 1);
    let text = &pages[0];
    for expected in [
        "Receitas R$ 23.000,00 · Despesas R$ 2.956,78 · Saldo R$ 20.043,22",
        "5 notas: 2 receitas e 3 despesas · 1 fora do total",
        "Natureza",
        "Receita",
        "Despesa",
        "Conta de energia abril.pdf",
        "R$ 456,78",
        "TOTAL RECEITAS",
        "TOTAL DESPESAS",
        "SALDO",
        "R$ 23.000,00",
        "R$ 2.956,78",
        "R$ 20.043,22",
        // Totals by competence: the balance of March and of April.
        "R$ 12.500,00",
        "R$ 7.543,22",
        "Página 1 de 1",
    ] {
        assert!(text.contains(expected), "{expected} not in {text}");
    }
}

#[test]
fn pdf_report_paginates_1200_rows() {
    let (_dir, pages) = export_pdf(synthetic_rows(1200, true));
    let count = pages.len();
    assert!(count >= 20, "{count} pages");
    let total_page = pages
        .iter()
        .position(|text| text.contains("TOTAL RECEITAS"))
        .expect("total rows");
    for (index, text) in pages.iter().enumerate() {
        if index <= total_page {
            assert!(
                text.contains("Arquivo") && text.contains("Natureza"),
                "page {index}"
            );
        }
        let footer = format!("Página {} de {count}", index + 1);
        assert!(text.contains(&footer), "{footer} not in page {index}");
    }
    assert!(pages[0].contains("Receitas R$"));
    assert!(pages[0].contains("1200 notas: 960 receitas e 240 despesas"));
    let all = pages.concat();
    assert!(all.contains("Boleto 1200"));
    assert!(all.contains("TOTAL DESPESAS") && all.contains("SALDO"));
    assert!(all.contains("Totais por competência"));
}

#[test]
fn pdf_report_shows_the_company() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = export_for(
        Some(COMPANY),
        ExportFormat::Pdf,
        dir.path(),
        "notas.pdf",
        synthetic_rows(120, true),
    );
    assert!(bytes.starts_with(b"%PDF-"));
    let pages = pdf_pages(&dir.path().join("notas.pdf"));
    assert!(pages.len() > 1);
    assert!(pages[0].contains(COMPANY), "{}", pages[0]);
    assert!(
        pages[0].contains("gerado em 29/09/2026 14:05 pelo nfsetable"),
        "{}",
        pages[0]
    );
    for (index, text) in pages.iter().enumerate() {
        let footer = format!("Relatório de notas · {COMPANY} · gerado em 29/09/2026 14:05");
        assert!(
            text.contains(&footer),
            "{footer} not in page {index}: {text}"
        );
    }
}

#[test]
fn pdf_report_without_rows() {
    let (_dir, pages) = export_pdf(Vec::new());
    assert_eq!(pages.len(), 1);
    for expected in [
        "0 notas · Total R$ 0,00",
        "Nenhuma nota para exportar.",
        "TOTAL",
        "Página 1 de 1",
    ] {
        assert!(
            pages[0].contains(expected),
            "{expected} not in {}",
            pages[0]
        );
    }
}

#[test]
fn pdf_needs_pdfium() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notas.pdf");
    let err = export_table(&request(ExportFormat::Pdf, &path, sample_rows()), None).unwrap_err();
    assert!(matches!(err, CoreError::PdfiumUnavailable(_)), "{err:?}");
    assert!(err.to_string().contains("PDF"), "{err}");
    assert!(!path.exists());
}

// ---------------------------------------------------------------- SQL

/// Rows whose texts need escaping: quote, backslash and `;` in the names, NUL, line breaks and
/// Ctrl-Z in the other texts, and unknown values. The second one is an expense.
fn tricky_rows() -> Vec<ExportRow> {
    vec![
        row(
            "d'água\\nota;x.pdf",
            "C:\\Notas\\d'água\\nota;x.pdf",
            "NFS-e",
            Some("2026-03"),
            Some(123_456),
            "OK\0",
            "linha 1\nlinha 2\r\nfim \u{1a}",
        ),
        expense(row(
            "sem valor.pdf",
            "C:\\Despesas\\sem valor.pdf",
            "Boleto",
            None,
            None,
            "Duplicada",
            "",
        )),
    ]
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Text(String),
    Number(String),
    Null,
}

/// What the script should insert for `row`.
fn expected_values(row: &ExportRow, keep_nul: bool) -> Vec<Value> {
    let text = |s: &str| {
        Value::Text(if keep_nul {
            s.to_string()
        } else {
            s.replace('\0', "")
        })
    };
    let natureza = match row.kind {
        Some(DocKind::Expense) => "despesa",
        Some(DocKind::Revenue) | None => "receita",
    };
    vec![
        text(&row.file),
        text(&row.path),
        text(&row.doc_type),
        text(natureza),
        row.competence
            .as_ref()
            .map_or(Value::Null, |c| Value::Text(format!("{c}-01"))),
        row.cents.map_or(Value::Null, |cents| {
            let sign = if cents < 0 { "-" } else { "" };
            let abs = cents.unsigned_abs();
            Value::Number(format!("{sign}{}.{:02}", abs / 100, abs % 100))
        }),
        text(&row.status),
        text(&row.origin),
    ]
}

/// Decodes the tuples of every `INSERT … VALUES` of `sql`, following the string literal rules
/// of PostgreSQL (`standard_conforming_strings` on) or of MySQL (default `sql_mode`).
fn inserted_tuples(sql: &str, mysql: bool) -> Vec<Vec<Vec<Value>>> {
    let chars: Vec<char> = sql.chars().collect();
    let marker: Vec<char> = "VALUES\n".chars().collect();
    let mut statements = Vec::new();
    let mut pos = 0;
    while let Some(found) = (pos..chars.len()).find(|&i| chars[i..].starts_with(&marker)) {
        pos = found + marker.len();
        let mut tuples = Vec::new();
        loop {
            while chars[pos].is_whitespace() {
                pos += 1;
            }
            assert_eq!(chars[pos], '(', "at {pos}");
            pos += 1;
            let mut values = Vec::new();
            loop {
                while chars[pos] == ' ' {
                    pos += 1;
                }
                if chars[pos] == '\'' {
                    let mut value = String::new();
                    pos += 1;
                    loop {
                        let c = chars[pos];
                        pos += 1;
                        match c {
                            '\'' if chars[pos] == '\'' => {
                                value.push('\'');
                                pos += 1;
                            }
                            '\'' => break,
                            '\\' if mysql => {
                                value.push(match chars[pos] {
                                    '0' => '\0',
                                    'n' => '\n',
                                    'r' => '\r',
                                    'Z' => '\u{1a}',
                                    other => other,
                                });
                                pos += 1;
                            }
                            _ => value.push(c),
                        }
                    }
                    values.push(Value::Text(value));
                } else {
                    let end = (pos..chars.len())
                        .find(|&i| chars[i] == ',' || chars[i] == ')')
                        .unwrap();
                    let token: String = chars[pos..end].iter().collect();
                    values.push(if token == "NULL" {
                        Value::Null
                    } else {
                        Value::Number(token)
                    });
                    pos = end;
                }
                let separator = chars[pos];
                pos += 1;
                if separator == ')' {
                    break;
                }
                assert_eq!(separator, ',', "at {pos}");
            }
            tuples.push(values);
            let separator = chars[pos];
            pos += 1;
            if separator == ';' {
                break;
            }
            assert_eq!(separator, ',', "at {pos}");
        }
        statements.push(tuples);
    }
    statements
}

/// Header comments of the scripts for `tricky_rows`.
const TRICKY_HEADER: &str = "-- Exportado pelo nfsetable em 29/09/2026 14:05\n\
                             -- 2 notas: 1 receita e 1 despesa\n\
                             -- Receitas: R$ 1.234,56\n\
                             -- Despesas: R$ 0,00\n\
                             -- Saldo (receitas - despesas): R$ 1.234,56\n";

#[test]
fn postgresql_script() {
    let rows = tricky_rows();
    let sql = export_text(ExportFormat::Postgresql, rows.clone());
    assert!(sql.starts_with(TRICKY_HEADER), "{sql}");
    for expected in [
        "SET client_encoding = 'UTF8';\n",
        "SET standard_conforming_strings = on;\n",
        "\nBEGIN;\n",
        "CREATE TABLE IF NOT EXISTS notas (\n    id SERIAL PRIMARY KEY,\n    arquivo TEXT NOT NULL,\n    caminho TEXT NOT NULL,\n    tipo TEXT NOT NULL,\n    natureza TEXT NOT NULL,\n    competencia DATE,\n    valor NUMERIC(14,2),\n    status TEXT NOT NULL,\n    origem TEXT NOT NULL\n);\n",
        "INSERT INTO notas (arquivo, caminho, tipo, natureza, competencia, valor, status, origem) VALUES\n",
        // Quotes doubled, backslashes literal, NUL removed.
        "    ('d''água\\nota;x.pdf', 'C:\\Notas\\d''água\\nota;x.pdf', 'NFS-e', 'receita', '2026-03-01', 1234.56, 'OK', 'linha 1\nlinha 2\r\nfim \u{1a}'),\n",
        "    ('sem valor.pdf', 'C:\\Despesas\\sem valor.pdf', 'Boleto', 'despesa', NULL, NULL, 'Duplicada', '');\n",
    ] {
        assert!(sql.contains(expected), "{expected:?} not in {sql}");
    }
    assert!(sql.ends_with("\nCOMMIT;\n"));
    assert!(!sql.contains('\0'));

    let statements = inserted_tuples(&sql, false);
    let expected: Vec<Vec<Value>> = rows.iter().map(|r| expected_values(r, false)).collect();
    assert_eq!(statements, [expected]);
}

#[test]
fn mysql_script() {
    let rows = tricky_rows();
    let sql = export_text(ExportFormat::Mysql, rows.clone());
    assert!(sql.starts_with(TRICKY_HEADER), "{sql}");
    for expected in [
        "sql_mode",
        "NO_BACKSLASH_ESCAPES",
        "SET NAMES utf8mb4;\nSTART TRANSACTION;\n",
        "CREATE TABLE IF NOT EXISTS `notas` (\n    `id` INT AUTO_INCREMENT PRIMARY KEY,\n    `arquivo` VARCHAR(512) NOT NULL,\n    `caminho` TEXT NOT NULL,\n    `tipo` VARCHAR(100) NOT NULL,\n    `natureza` VARCHAR(10) NOT NULL,\n    `competencia` DATE NULL,\n    `valor` DECIMAL(14,2) NULL,\n    `status` VARCHAR(40) NOT NULL,\n    `origem` VARCHAR(200) NOT NULL\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;\n",
        "INSERT INTO `notas` (`arquivo`, `caminho`, `tipo`, `natureza`, `competencia`, `valor`, `status`, `origem`) VALUES\n",
        // Backslashes, NUL, line breaks and Ctrl-Z escaped; quotes doubled.
        "    ('d''água\\\\nota;x.pdf', 'C:\\\\Notas\\\\d''água\\\\nota;x.pdf', 'NFS-e', 'receita', '2026-03-01', 1234.56, 'OK\\0', 'linha 1\\nlinha 2\\r\\nfim \\Z'),\n",
        "    ('sem valor.pdf', 'C:\\\\Despesas\\\\sem valor.pdf', 'Boleto', 'despesa', NULL, NULL, 'Duplicada', '');\n",
    ] {
        assert!(sql.contains(expected), "{expected:?} not in {sql}");
    }
    assert!(sql.ends_with("\nCOMMIT;\n"));
    // Every row stays on one line.
    assert_eq!(sql.lines().filter(|l| l.starts_with("    (")).count(), 2);

    let statements = inserted_tuples(&sql, true);
    let expected: Vec<Vec<Value>> = rows.iter().map(|r| expected_values(r, true)).collect();
    assert_eq!(statements, [expected]);
}

#[test]
fn mysql_cuts_texts_to_the_column_sizes() {
    let mut rows = tricky_rows();
    rows[0].origin = "á".repeat(250);
    rows[0].status = "s".repeat(41);
    let sql = export_text(ExportFormat::Mysql, rows);
    let tuples = &inserted_tuples(&sql, true)[0];
    assert_eq!(tuples[0][7], Value::Text("á".repeat(200)));
    assert_eq!(tuples[0][6], Value::Text("s".repeat(40)));
}

#[test]
fn sql_inserts_in_batches_of_500() {
    let rows = synthetic_rows(1200, true);
    for (format, mysql) in [
        (ExportFormat::Postgresql, false),
        (ExportFormat::Mysql, true),
    ] {
        let sql = export_text(format, rows.clone());
        assert_eq!(sql.matches("INSERT INTO").count(), 3, "{format:?}");
        assert!(sql.contains("-- 1200 notas: 960 receitas e 240 despesas\n"));
        let statements = inserted_tuples(&sql, mysql);
        let sizes: Vec<usize> = statements.iter().map(Vec::len).collect();
        assert_eq!(sizes, [500, 500, 200], "{format:?}");
        let decoded: Vec<Vec<Value>> = statements.into_iter().flatten().collect();
        let expected: Vec<Vec<Value>> = rows.iter().map(|r| expected_values(r, true)).collect();
        assert_eq!(decoded, expected, "{format:?}");
    }
}

#[test]
fn sql_header_names_the_company_on_one_line() {
    let company = "Consultoria Fictícia Ltda\nDROP TABLE notas; --\r\nfim\u{2028}";
    for (format, mysql) in [
        (ExportFormat::Postgresql, false),
        (ExportFormat::Mysql, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let bytes = export_for(
            Some(company),
            format,
            dir.path(),
            "notas.sql",
            tricky_rows(),
        );
        let sql = String::from_utf8(bytes).unwrap();
        assert!(
            sql.starts_with(
                "-- Exportado pelo nfsetable em 29/09/2026 14:05\n\
                 -- Empresa: Consultoria Fictícia Ltda DROP TABLE notas; -- fim\n\
                 -- 2 notas: 1 receita e 1 despesa\n"
            ),
            "{sql}"
        );
        // The name never leaves its comment line.
        assert!(!sql.lines().any(|line| line.starts_with("DROP")), "{sql}");
        let expected: Vec<Vec<Value>> = tricky_rows()
            .iter()
            .map(|row| expected_values(row, mysql))
            .collect();
        assert_eq!(inserted_tuples(&sql, mysql), [expected], "{format:?}");
    }
}

#[test]
fn sql_without_rows_only_creates_the_table() {
    for format in [ExportFormat::Postgresql, ExportFormat::Mysql] {
        let sql = export_text(format, Vec::new());
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS"), "{format:?}");
        assert!(!sql.contains("INSERT"), "{format:?}");
        assert!(
            sql.contains("-- 0 notas\n-- Receitas: R$ 0,00\n-- Despesas: R$ 0,00\n"),
            "{format:?}"
        );
        assert!(sql.ends_with("\nCOMMIT;\n"), "{format:?}");
    }
}

// ---------------------------------------------------------------- samples

/// Writes one file of each format, plus the first and last pages of the PDF as PNG, for a visual
/// check: `NFSETABLE_EXPORT_SAMPLES=<folder> cargo test -p nfsetable-core --test export -- --ignored`
#[test]
#[ignore]
fn write_samples() {
    let Some(dir) = std::env::var_os("NFSETABLE_EXPORT_SAMPLES").map(PathBuf::from) else {
        eprintln!("NFSETABLE_EXPORT_SAMPLES not set: nothing written");
        return;
    };
    std::fs::create_dir_all(&dir).unwrap();
    let mut rows = synthetic_rows(40, true);
    rows[2].file = format!(
        "NFS-e 0003 - {}- relatório de serviços prestados em março de 2026 - nota 000123.pdf",
        "EMPRESA EXEMPLO DE NOME MUITO LONGO LTDA ".repeat(2)
    );
    rows[2].path = format!("C:\\Notas\\2026\\{}", rows[2].file);
    rows[3].cents = Some(-12_345);
    rows[3].status = "Ajuste manual".into();
    for (format, name) in [
        (ExportFormat::Csv, "notas.csv"),
        (ExportFormat::Xlsx, "notas.xlsx"),
        (ExportFormat::Pdf, "notas.pdf"),
        (ExportFormat::Postgresql, "notas-postgresql.sql"),
        (ExportFormat::Mysql, "notas-mysql.sql"),
    ] {
        export_for(Some(COMPANY), format, &dir, name, rows.clone());
    }
    let pdf = dir.join("notas.pdf");
    let last = engine().render_page(&pdf, 0, 64).unwrap().page_count - 1;
    for index in [0, last] {
        let page = engine().render_page(&pdf, index, 1240).unwrap();
        let png = base64::engine::general_purpose::STANDARD
            .decode(page.data_url.trim_start_matches("data:image/png;base64,"))
            .unwrap();
        std::fs::write(dir.join(format!("notas-pagina-{}.png", index + 1)), png).unwrap();
    }
}

// ---------------------------------------------------------------- zip

/// Just enough of ZIP and DEFLATE (RFC 1951) to read the XLSX written by rust_xlsxwriter.
mod zip {
    use std::collections::BTreeMap;

    fn u16_at(bytes: &[u8], at: usize) -> usize {
        usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]))
    }

    fn u32_at(bytes: &[u8], at: usize) -> usize {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize
    }

    /// Entries of the archive, by name, uncompressed (read through the central directory).
    pub fn read(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
        let end = (0..=bytes.len() - 22)
            .rev()
            .find(|&i| bytes[i..i + 4] == *b"PK\x05\x06")
            .expect("end of central directory");
        let mut pos = u32_at(bytes, end + 16);
        let mut files = BTreeMap::new();
        for _ in 0..u16_at(bytes, end + 10) {
            assert_eq!(bytes[pos..pos + 4], *b"PK\x01\x02");
            let method = u16_at(bytes, pos + 10);
            let size = u32_at(bytes, pos + 20);
            let name_len = u16_at(bytes, pos + 28);
            let local = u32_at(bytes, pos + 42);
            let name = String::from_utf8(bytes[pos + 46..pos + 46 + name_len].to_vec()).unwrap();
            pos += 46 + name_len + u16_at(bytes, pos + 30) + u16_at(bytes, pos + 32);
            let start = local + 30 + u16_at(bytes, local + 26) + u16_at(bytes, local + 28);
            let data = &bytes[start..start + size];
            let content = match method {
                0 => data.to_vec(),
                8 => inflate(data),
                other => panic!("compression method {other}"),
            };
            files.insert(name, content);
        }
        files
    }

    struct Bits<'a> {
        data: &'a [u8],
        pos: usize,
        buffer: u32,
        count: u32,
    }

    impl Bits<'_> {
        fn take(&mut self, n: u32) -> usize {
            while self.count < n {
                self.buffer |= u32::from(self.data[self.pos]) << self.count;
                self.pos += 1;
                self.count += 8;
            }
            let value = self.buffer & ((1 << n) - 1);
            self.buffer >>= n;
            self.count -= n;
            value as usize
        }
    }

    /// Canonical Huffman code: number of codes per length and symbols in code order.
    struct Huffman {
        counts: [usize; 16],
        symbols: Vec<usize>,
    }

    fn huffman(lengths: &[usize]) -> Huffman {
        let mut counts = [0; 16];
        for &len in lengths {
            counts[len] += 1;
        }
        counts[0] = 0;
        let mut offsets = [0; 16];
        for len in 1..15 {
            offsets[len + 1] = offsets[len] + counts[len];
        }
        let mut symbols = vec![0; lengths.len()];
        for (symbol, &len) in lengths.iter().enumerate() {
            if len != 0 {
                symbols[offsets[len]] = symbol;
                offsets[len] += 1;
            }
        }
        Huffman { counts, symbols }
    }

    fn decode(bits: &mut Bits, code: &Huffman) -> usize {
        let (mut value, mut first, mut index) = (0, 0, 0);
        for len in 1..16 {
            value |= bits.take(1);
            let count = code.counts[len];
            if value < first + count {
                return code.symbols[index + value - first];
            }
            index += count;
            first = (first + count) << 1;
            value <<= 1;
        }
        panic!("invalid Huffman code");
    }

    const LENGTH_BASE: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [u32; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE_BASE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [u32; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];

    fn inflate(data: &[u8]) -> Vec<u8> {
        let mut bits = Bits {
            data,
            pos: 0,
            buffer: 0,
            count: 0,
        };
        let mut out = Vec::new();
        loop {
            let last = bits.take(1) == 1;
            match bits.take(2) {
                0 => {
                    // Stored block: skip to the byte boundary, then LEN, NLEN and the bytes.
                    bits.buffer = 0;
                    bits.count = 0;
                    let len = u16_at(data, bits.pos);
                    let start = bits.pos + 4;
                    out.extend_from_slice(&data[start..start + len]);
                    bits.pos = start + len;
                }
                1 => {
                    let mut lengths = vec![8; 288];
                    lengths[144..256].fill(9);
                    lengths[256..280].fill(7);
                    block(&mut bits, &mut out, &huffman(&lengths), &huffman(&[5; 30]));
                }
                2 => {
                    let (literals, distances) = dynamic_codes(&mut bits);
                    block(&mut bits, &mut out, &literals, &distances);
                }
                _ => panic!("invalid block type"),
            }
            if last {
                return out;
            }
        }
    }

    fn dynamic_codes(bits: &mut Bits) -> (Huffman, Huffman) {
        const ORDER: [usize; 19] = [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ];
        let literal_count = bits.take(5) + 257;
        let distance_count = bits.take(5) + 1;
        let code_count = bits.take(4) + 4;
        let mut code_lengths = [0; 19];
        for &i in &ORDER[..code_count] {
            code_lengths[i] = bits.take(3);
        }
        let code = huffman(&code_lengths);
        let mut lengths = Vec::new();
        while lengths.len() < literal_count + distance_count {
            let (value, repeat) = match decode(bits, &code) {
                16 => (*lengths.last().unwrap(), 3 + bits.take(2)),
                17 => (0, 3 + bits.take(3)),
                18 => (0, 11 + bits.take(7)),
                symbol => (symbol, 1),
            };
            lengths.resize(lengths.len() + repeat, value);
        }
        (
            huffman(&lengths[..literal_count]),
            huffman(&lengths[literal_count..]),
        )
    }

    fn block(bits: &mut Bits, out: &mut Vec<u8>, literals: &Huffman, distances: &Huffman) {
        loop {
            match decode(bits, literals) {
                byte @ 0..=255 => out.push(byte as u8),
                256 => return,
                symbol => {
                    let i = symbol - 257;
                    let len = LENGTH_BASE[i] + bits.take(LENGTH_EXTRA[i]);
                    let d = decode(bits, distances);
                    let distance = DISTANCE_BASE[d] + bits.take(DISTANCE_EXTRA[d]);
                    let start = out.len() - distance;
                    for k in 0..len {
                        out.push(out[start + k]);
                    }
                }
            }
        }
    }
}

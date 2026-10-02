//! Printable A4 report drawn with PDFium: title, summary line, the table (header repeated on every
//! page, zebra rows, TOTAL row), totals by type and by competence, and "Página N de M".
//!
//! When the table has expense rows, revenue and expenses are kept apart: the table gets a
//! "Natureza" column, the TOTAL row becomes TOTAL RECEITAS, TOTAL DESPESAS and SALDO, the totals
//! by type are listed per natureza and the totals by competence show revenue, expenses and
//! balance per month. Reports with revenue only keep the simpler layout.
//!
//! The layout is computed first, as drawing operations per page, so the number of pages is known
//! when the footers are written. Each page is then drawn with content regeneration set to manual
//! and regenerated once.
//!
//! Text uses the standard Helvetica fonts (WinAnsiEncoding, which covers pt-BR); see
//! [`super::helvetica`] for the metrics used to align and cut it.

use super::helvetica::{fit_end, fit_middle, sanitize, text_width, Font};
use super::{competence, competence_label, kind, kind_label, Summary, BALANCE, DISCLAIMER, TITLE};
use crate::engine::Engine;
use crate::error::CoreError;
use crate::model::ExportRow;
use crate::parse::format_brl;
use pdfium_render::prelude::*;

// Page geometry in points (A4 portrait). Every y below is measured from the TOP of the page.
const PAGE_WIDTH: f32 = 595.2756;
const PAGE_HEIGHT: f32 = 841.8898;
const MARGIN_X: f32 = 40.0;
const MARGIN_TOP: f32 = 40.0;
const CONTENT_WIDTH: f32 = PAGE_WIDTH - 2.0 * MARGIN_X;
/// The content ends here; the footer lives below.
const CONTENT_BOTTOM: f32 = PAGE_HEIGHT - 50.0;
const FOOTER_BASELINE: f32 = PAGE_HEIGHT - 26.0;
const FOOTER_SIZE: f32 = 7.5;

// Tables.
const BODY_SIZE: f32 = 8.0;
const ROW_HEIGHT: f32 = 14.0;
const HEADER_HEIGHT: f32 = 16.0;
const CELL_PADDING: f32 = 4.0;
/// Cap height of Helvetica in em: text is centered on it.
const CAP_HEIGHT: f32 = 0.718;
/// Width of the tables of totals (three and four columns).
const GROUP_WIDTH: f32 = 330.0;
const WIDE_GROUP_WIDTH: f32 = 400.0;
const SECTION_GAP: f32 = 22.0;
const SECTION_TITLE_HEIGHT: f32 = 20.0;

// Colors.
const INK: Rgb = Rgb(17, 24, 39);
const MUTED: Rgb = Rgb(107, 114, 128);
const NEGATIVE: Rgb = Rgb(185, 28, 28);
const HEADER_FILL: Rgb = Rgb(221, 235, 247);
const ZEBRA_FILL: Rgb = Rgb(244, 246, 249);
const TOTAL_FILL: Rgb = Rgb(229, 231, 235);
const RULE: Rgb = Rgb(107, 114, 128);
const LIGHT_RULE: Rgb = Rgb(209, 213, 219);

/// Shown for a missing competence or amount.
const NO_VALUE: &str = "—";

/// A column: title, width (0 = the rest), alignment and how text is cut.
type ColumnSpec = (&'static str, f32, Align, Cut);

/// Columns of the notes table.
const NOTE_COLUMNS: [ColumnSpec; 5] = [
    ("Arquivo", 0.0, Align::Left, Cut::Middle),
    ("Tipo", 48.0, Align::Left, Cut::End),
    ("Competência", 62.0, Align::Left, Cut::End),
    ("Status", 92.0, Align::Left, Cut::End),
    ("Valor", 84.0, Align::Right, Cut::Never),
];

/// Columns of the notes table when there are expenses.
const SPLIT_NOTE_COLUMNS: [ColumnSpec; 6] = [
    ("Arquivo", 0.0, Align::Left, Cut::Middle),
    ("Tipo", 50.0, Align::Left, Cut::End),
    ("Natureza", 46.0, Align::Left, Cut::End),
    ("Competência", 60.0, Align::Left, Cut::End),
    ("Status", 76.0, Align::Left, Cut::End),
    ("Valor", 80.0, Align::Right, Cut::Never),
];

/// The report described in the module documentation; `company`, when given, is shown under the
/// title and in the footer of every page.
pub(super) fn to_bytes(
    engine: &Engine,
    rows: &[ExportRow],
    generated_at: &str,
    company: Option<&str>,
) -> Result<Vec<u8>, CoreError> {
    let pages = layout(rows, &Summary::new(rows), generated_at, company);
    render(engine, &pages)
        .map_err(|err| CoreError::Export(format!("não foi possível gerar o PDF ({err:?}).")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rgb(u8, u8, u8);

impl Rgb {
    fn pdf(self) -> PdfColor {
        PdfColor::new(self.0, self.1, self.2, 255)
    }
}

/// A drawing operation, in points from the top-left corner of the page.
#[derive(Debug, Clone, PartialEq)]
enum Op {
    Text {
        x: f32,
        baseline: f32,
        text: String,
        font: Font,
        size: f32,
        color: Rgb,
    },
    Fill {
        x: f32,
        top: f32,
        width: f32,
        height: f32,
        color: Rgb,
    },
    /// A horizontal line.
    Rule {
        x1: f32,
        x2: f32,
        y: f32,
        width: f32,
        color: Rgb,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
}

/// How a text too wide for its column is cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cut {
    End,
    Middle,
    /// Amounts are never cut.
    Never,
}

#[derive(Debug, Clone, Copy)]
struct Column {
    title: &'static str,
    x: f32,
    width: f32,
    align: Align,
    cut: Cut,
}

/// Lays `specs` out side by side from the left margin; the column of width 0 takes the rest.
fn columns(specs: &[ColumnSpec], total_width: f32) -> Vec<Column> {
    let fixed: f32 = specs.iter().map(|spec| spec.1).sum();
    let mut x = MARGIN_X;
    specs
        .iter()
        .map(|&(title, width, align, cut)| {
            let width = if width > 0.0 {
                width
            } else {
                total_width - fixed
            };
            let column = Column {
                title,
                x,
                width,
                align,
                cut,
            };
            x += width;
            column
        })
        .collect()
}

struct Cell {
    text: String,
    font: Font,
    color: Rgb,
}

impl Cell {
    fn new(text: impl Into<String>, font: Font, color: Rgb) -> Cell {
        Cell {
            text: text.into(),
            font,
            color,
        }
    }

    fn empty() -> Cell {
        Cell::new(String::new(), Font::Regular, INK)
    }
}

fn money_cell(cents: Option<i64>, font: Font) -> Cell {
    match cents {
        Some(cents) if cents < 0 => Cell::new(format_brl(cents), font, NEGATIVE),
        Some(cents) => Cell::new(format_brl(cents), font, INK),
        None => Cell::new(NO_VALUE, font, MUTED),
    }
}

/// Cells of a note, with the natureza after the type when `split`.
fn note_cells(row: &ExportRow, split: bool) -> Vec<Cell> {
    let mut cells = vec![
        Cell::new(sanitize(&row.file), Font::Regular, INK),
        Cell::new(sanitize(row.doc_type.trim()), Font::Regular, INK),
    ];
    if split {
        cells.push(Cell::new(kind_label(kind(row)), Font::Regular, INK));
    }
    cells.extend([
        match competence(row) {
            Some(month) => Cell::new(month.display(), Font::Regular, INK),
            None => Cell::new(NO_VALUE, Font::Regular, MUTED),
        },
        Cell::new(sanitize(&row.status), Font::Regular, INK),
        money_cell(row.cents, Font::Regular),
    ]);
    cells
}

/// Zebra background of the odd rows.
fn zebra(index: usize) -> Option<Rgb> {
    (index % 2 == 1).then_some(ZEBRA_FILL)
}

/// Baseline that centers capital letters of `size` points in a band starting at `top`.
fn baseline_in(top: f32, band: f32, size: f32) -> f32 {
    top + (band + CAP_HEIGHT * size) / 2.0
}

/// Pages under construction and the vertical position of the next block.
struct Layout {
    pages: Vec<Vec<Op>>,
    y: f32,
}

impl Layout {
    fn new() -> Layout {
        Layout {
            pages: vec![Vec::new()],
            y: MARGIN_TOP,
        }
    }

    fn room(&self) -> f32 {
        CONTENT_BOTTOM - self.y
    }

    fn page_break(&mut self) {
        self.pages.push(Vec::new());
        self.y = MARGIN_TOP;
    }

    fn push(&mut self, op: Op) {
        if let Some(page) = self.pages.last_mut() {
            page.push(op);
        }
    }

    fn text(&mut self, x: f32, baseline: f32, text: String, font: Font, size: f32, color: Rgb) {
        if !text.is_empty() {
            self.push(Op::Text {
                x,
                baseline,
                text,
                font,
                size,
                color,
            });
        }
    }

    fn fill(&mut self, x: f32, top: f32, width: f32, height: f32, color: Rgb) {
        self.push(Op::Fill {
            x,
            top,
            width,
            height,
            color,
        });
    }

    fn rule(&mut self, columns: &[Column], y: f32, width: f32, color: Rgb) {
        let (x1, x2) = span(columns);
        self.push(Op::Rule {
            x1,
            x2,
            y,
            width,
            color,
        });
    }

    /// Starts a new page (repeating the table header) when the next row does not fit.
    fn ensure_row(&mut self, columns: &[Column]) {
        if self.room() < ROW_HEIGHT {
            self.page_break();
            self.table_header(columns);
        }
    }

    fn table_header(&mut self, columns: &[Column]) {
        let (left, right) = span(columns);
        self.fill(left, self.y, right - left, HEADER_HEIGHT, HEADER_FILL);
        let baseline = baseline_in(self.y, HEADER_HEIGHT, BODY_SIZE);
        for column in columns {
            self.cell(column, baseline, &Cell::new(column.title, Font::Bold, INK));
        }
        self.rule(columns, self.y + HEADER_HEIGHT, 0.8, RULE);
        self.y += HEADER_HEIGHT;
    }

    fn table_row(&mut self, columns: &[Column], cells: &[Cell], fill: Option<Rgb>) {
        if let Some(color) = fill {
            let (left, right) = span(columns);
            self.fill(left, self.y, right - left, ROW_HEIGHT, color);
        }
        let baseline = baseline_in(self.y, ROW_HEIGHT, BODY_SIZE);
        for (column, cell) in columns.iter().zip(cells) {
            self.cell(column, baseline, cell);
        }
        self.y += ROW_HEIGHT;
    }

    fn cell(&mut self, column: &Column, baseline: f32, cell: &Cell) {
        let room = column.width - 2.0 * CELL_PADDING;
        let text = match column.cut {
            Cut::End => fit_end(&cell.text, cell.font, BODY_SIZE, room),
            Cut::Middle => fit_middle(&cell.text, cell.font, BODY_SIZE, room),
            Cut::Never => cell.text.clone(),
        };
        let x = match column.align {
            Align::Left => column.x + CELL_PADDING,
            Align::Right => {
                column.x + column.width - CELL_PADDING - text_width(&text, cell.font, BODY_SIZE)
            }
        };
        self.text(x, baseline, text, cell.font, BODY_SIZE, cell.color);
    }

    /// The lines that close the table (label in the first column, amount in the last), kept
    /// together on one page, with a rule above the first line and above the balance.
    fn total_rows(&mut self, columns: &[Column], lines: &[(&str, i64)]) {
        if self.room() < ROW_HEIGHT * lines.len() as f32 {
            self.page_break();
            self.table_header(columns);
        }
        let last = columns.len().saturating_sub(1);
        for (i, &(label, cents)) in lines.iter().enumerate() {
            let cells: Vec<Cell> = (0..columns.len())
                .map(|col| match col {
                    0 => Cell::new(label, Font::Bold, INK),
                    col if col == last => money_cell(Some(cents), Font::Bold),
                    _ => Cell::empty(),
                })
                .collect();
            let top = self.y;
            self.table_row(columns, &cells, Some(TOTAL_FILL));
            if i == 0 || label == BALANCE {
                self.rule(columns, top, 0.8, RULE);
            }
        }
    }

    /// A titled table of totals.
    fn section(&mut self, title: &str, specs: &[ColumnSpec], width: f32, rows: &[Vec<Cell>]) {
        if rows.is_empty() {
            return;
        }
        let columns = columns(specs, width);
        // Keeps the title with the table header and the first row.
        if self.room() < SECTION_GAP + SECTION_TITLE_HEIGHT + HEADER_HEIGHT + ROW_HEIGHT {
            self.page_break();
        } else {
            self.y += SECTION_GAP;
        }
        self.text(
            MARGIN_X,
            self.y + 12.0,
            title.to_string(),
            Font::Bold,
            11.0,
            INK,
        );
        self.y += SECTION_TITLE_HEIGHT;
        self.table_header(&columns);
        for (i, cells) in rows.iter().enumerate() {
            self.ensure_row(&columns);
            self.table_row(&columns, cells, zebra(i));
        }
    }
}

/// Totals by type: without expenses, and per natureza.
const TYPE_COLUMNS: [ColumnSpec; 3] = [
    ("Tipo", 0.0, Align::Left, Cut::End),
    ("Notas", 56.0, Align::Right, Cut::Never),
    ("Total", 104.0, Align::Right, Cut::Never),
];
const SPLIT_TYPE_COLUMNS: [ColumnSpec; 4] = [
    ("Natureza", 64.0, Align::Left, Cut::End),
    ("Tipo", 0.0, Align::Left, Cut::End),
    ("Notas", 56.0, Align::Right, Cut::Never),
    ("Total", 104.0, Align::Right, Cut::Never),
];

/// Totals by competence: without expenses, and revenue, expenses and balance per month.
const MONTH_COLUMNS: [ColumnSpec; 3] = [
    ("Competência", 0.0, Align::Left, Cut::End),
    ("Notas", 56.0, Align::Right, Cut::Never),
    ("Total", 104.0, Align::Right, Cut::Never),
];
const SPLIT_MONTH_COLUMNS: [ColumnSpec; 4] = [
    ("Competência", 0.0, Align::Left, Cut::End),
    ("Receitas", 100.0, Align::Right, Cut::Never),
    ("Despesas", 100.0, Align::Right, Cut::Never),
    ("Saldo", 100.0, Align::Right, Cut::Never),
];

/// `text` with its first letter in upper case.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Left and right edges of a table.
fn span(columns: &[Column]) -> (f32, f32) {
    let left = columns.first().map_or(MARGIN_X, |c| c.x);
    let right = columns.last().map_or(MARGIN_X, |c| c.x + c.width);
    (left, right)
}

/// The whole report, page by page, footers included.
fn layout(
    rows: &[ExportRow],
    summary: &Summary,
    generated_at: &str,
    company: Option<&str>,
) -> Vec<Vec<Op>> {
    let generated_at = sanitize(generated_at.trim());
    let company = company.map(sanitize);
    let split = summary.has_expenses();
    let totals = &summary.totals;
    let mut layout = Layout::new();

    layout.text(MARGIN_X, 56.0, TITLE.to_string(), Font::Bold, 18.0, INK);
    let generated = if generated_at.is_empty() {
        "gerado pelo nfsetable".to_string()
    } else {
        format!("gerado em {generated_at} pelo nfsetable")
    };
    match &company {
        // "Empresa Exemplo LTDA · gerado em … pelo nfsetable", the company in bold.
        Some(company) => {
            let company = fit_end(company, Font::Bold, 9.0, CONTENT_WIDTH * 0.6);
            let company_width = text_width(&company, Font::Bold, 9.0);
            layout.text(MARGIN_X, 73.0, company, Font::Bold, 9.0, INK);
            let rest = format!(" · {generated}");
            let rest = fit_end(&rest, Font::Regular, 9.0, CONTENT_WIDTH - company_width);
            let x = MARGIN_X + company_width;
            layout.text(x, 73.0, rest, Font::Regular, 9.0, MUTED);
        }
        None => {
            let subtitle = capitalize(&generated);
            let subtitle = fit_end(&subtitle, Font::Regular, 9.0, CONTENT_WIDTH);
            layout.text(MARGIN_X, 73.0, subtitle, Font::Regular, 9.0, MUTED);
        }
    }
    let outside = if summary.without_value > 0 {
        format!(" · {} fora do total", summary.without_value)
    } else {
        String::new()
    };
    if split {
        let amounts = format!(
            "Receitas {} · Despesas {} · Saldo {}",
            format_brl(totals.revenue.cents),
            format_brl(totals.expense.cents),
            format_brl(totals.balance())
        );
        let amounts = fit_end(&amounts, Font::Bold, 11.0, CONTENT_WIDTH);
        layout.text(MARGIN_X, 93.0, amounts, Font::Bold, 11.0, INK);
        let counts = format!("{}{outside}", summary.count_text());
        let counts = fit_end(&counts, Font::Regular, 9.0, CONTENT_WIDTH);
        layout.text(MARGIN_X, 108.0, counts, Font::Regular, 9.0, MUTED);
        layout.y = 120.0;
    } else {
        let line = format!(
            "{} · Total {}{outside}",
            summary.count_text(),
            format_brl(totals.revenue.cents)
        );
        layout.text(MARGIN_X, 93.0, line, Font::Bold, 11.0, INK);
        layout.y = 106.0;
    }

    let columns = if split {
        columns(&SPLIT_NOTE_COLUMNS, CONTENT_WIDTH)
    } else {
        columns(&NOTE_COLUMNS, CONTENT_WIDTH)
    };
    layout.table_header(&columns);
    if rows.is_empty() {
        let cells = [Cell::new(
            "Nenhuma nota para exportar.",
            Font::Regular,
            MUTED,
        )];
        layout.table_row(&columns, &cells, None);
    }
    for (i, row) in rows.iter().enumerate() {
        layout.ensure_row(&columns);
        layout.table_row(&columns, &note_cells(row, split), zebra(i));
    }
    layout.total_rows(&columns, &summary.total_lines());

    let text = |text: String| Cell::new(text, Font::Regular, INK);
    let money = |cents: i64| money_cell(Some(cents), Font::Regular);
    if split {
        let by_type: Vec<Vec<Cell>> = summary
            .by_type
            .iter()
            .map(|(kind, doc_type, group)| {
                vec![
                    text(kind_label(*kind).to_string()),
                    text(sanitize(doc_type)),
                    text(group.count.to_string()),
                    money(group.cents),
                ]
            })
            .collect();
        let title = "Totais por tipo";
        layout.section(title, &SPLIT_TYPE_COLUMNS, WIDE_GROUP_WIDTH, &by_type);
        let by_month: Vec<Vec<Cell>> = summary
            .by_competence
            .iter()
            .map(|(month, kinds)| {
                vec![
                    text(competence_label(*month)),
                    money(kinds.revenue.cents),
                    money(kinds.expense.cents),
                    money(kinds.balance()),
                ]
            })
            .collect();
        let title = "Totais por competência";
        layout.section(title, &SPLIT_MONTH_COLUMNS, WIDE_GROUP_WIDTH, &by_month);
    } else {
        let by_type: Vec<Vec<Cell>> = summary
            .by_type
            .iter()
            .map(|(_, doc_type, group)| {
                vec![
                    text(sanitize(doc_type)),
                    text(group.count.to_string()),
                    money(group.cents),
                ]
            })
            .collect();
        layout.section("Totais por tipo", &TYPE_COLUMNS, GROUP_WIDTH, &by_type);
        let by_month: Vec<Vec<Cell>> = summary
            .by_competence
            .iter()
            .map(|(month, kinds)| {
                vec![
                    text(competence_label(*month)),
                    text(kinds.revenue.count.to_string()),
                    money(kinds.revenue.cents),
                ]
            })
            .collect();
        let title = "Totais por competência";
        layout.section(title, &MONTH_COLUMNS, GROUP_WIDTH, &by_month);
    }

    if layout.room() < 24.0 {
        layout.page_break();
    } else {
        layout.y += 18.0;
    }
    layout.text(
        MARGIN_X,
        layout.y + 8.0,
        format!("{DISCLAIMER}."),
        Font::Regular,
        FOOTER_SIZE,
        MUTED,
    );

    let mut pages = layout.pages;
    add_footers(&mut pages, &generated, company.as_deref());
    pages
}

/// A rule, the report name and "Página N de M" at the bottom of every page.
/// `generated` is "gerado em … pelo nfsetable"; the company, when given, goes after the title.
fn add_footers(pages: &mut [Vec<Op>], generated: &str, company: Option<&str>) {
    let count = pages.len();
    let name = match company {
        Some(company) => format!("{TITLE} · {company} · {generated}"),
        None => format!("{TITLE} · {generated}"),
    };
    for (index, ops) in pages.iter_mut().enumerate() {
        let number = format!("Página {} de {count}", index + 1);
        let number_width = text_width(&number, Font::Regular, FOOTER_SIZE);
        ops.push(Op::Rule {
            x1: MARGIN_X,
            x2: PAGE_WIDTH - MARGIN_X,
            y: FOOTER_BASELINE - 10.0,
            width: 0.5,
            color: LIGHT_RULE,
        });
        ops.push(Op::Text {
            x: MARGIN_X,
            baseline: FOOTER_BASELINE,
            text: fit_end(
                &name,
                Font::Regular,
                FOOTER_SIZE,
                CONTENT_WIDTH - number_width - 24.0,
            ),
            font: Font::Regular,
            size: FOOTER_SIZE,
            color: MUTED,
        });
        ops.push(Op::Text {
            x: PAGE_WIDTH - MARGIN_X - number_width,
            baseline: FOOTER_BASELINE,
            text: number,
            font: Font::Regular,
            size: FOOTER_SIZE,
            color: MUTED,
        });
    }
}

/// Draws the pages with PDFium and returns the PDF file.
fn render(engine: &Engine, pages: &[Vec<Op>]) -> Result<Vec<u8>, PdfiumError> {
    let mut document = engine.pdfium().create_new_pdf()?;
    let regular = document.fonts_mut().helvetica();
    let bold = document.fonts_mut().helvetica_bold();
    for ops in pages {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())?;
        // Regenerating the content stream after every object would make large reports slow.
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let page_height = page.height().value;
        for op in ops {
            draw(&mut page, op, regular, bold, page_height)?;
        }
        page.regenerate_content()?;
    }
    document.save_to_bytes()
}

/// Draws one operation; PDFium places objects from the BOTTOM-left corner of the page.
fn draw(
    page: &mut PdfPage,
    op: &Op,
    regular: PdfFontToken,
    bold: PdfFontToken,
    page_height: f32,
) -> Result<(), PdfiumError> {
    let objects = page.objects_mut();
    match op {
        Op::Text {
            x,
            baseline,
            text,
            font,
            size,
            color,
        } => {
            let font = match font {
                Font::Regular => regular,
                Font::Bold => bold,
            };
            let mut object = objects.create_text_object(
                PdfPoints::new(*x),
                PdfPoints::new(page_height - baseline),
                text,
                font,
                PdfPoints::new(*size),
            )?;
            object.set_fill_color(color.pdf())?;
        }
        Op::Fill {
            x,
            top,
            width,
            height,
            color,
        } => {
            let rect = PdfRect::new_from_values(
                page_height - top - height,
                *x,
                page_height - top,
                x + width,
            );
            objects.create_path_object_rect(rect, None, None, Some(color.pdf()))?;
        }
        Op::Rule {
            x1,
            x2,
            y,
            width,
            color,
        } => {
            objects.create_path_object_line(
                PdfPoints::new(*x1),
                PdfPoints::new(page_height - y),
                PdfPoints::new(*x2),
                PdfPoints::new(page_height - y),
                color.pdf(),
                PdfPoints::new(*width),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::DocKind;

    fn rows(count: usize) -> Vec<ExportRow> {
        (1..=count)
            .map(|i| ExportRow {
                file: format!("NFS-e {i:04} EMPRESA EXEMPLO LTDA.pdf"),
                path: format!("C:/Notas/NFS-e {i:04} EMPRESA EXEMPLO LTDA.pdf"),
                doc_type: "NFS-e".into(),
                kind: None,
                competence: Some(format!("2026-{:02}", i % 12 + 1)),
                cents: (i % 10 != 0).then_some(i as i64 * 1_000 + 56),
                status: "OK".into(),
                origin: "DANFSe (padrão nacional)".into(),
            })
            .collect()
    }

    fn texts(page: &[Op]) -> Vec<&str> {
        page.iter()
            .filter_map(|op| match op {
                Op::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn report(rows: &[ExportRow]) -> Vec<Vec<Op>> {
        layout(rows, &Summary::new(rows), "29/09/2026 14:05", None)
    }

    fn company_report(rows: &[ExportRow], company: &str) -> Vec<Vec<Op>> {
        layout(rows, &Summary::new(rows), "29/09/2026 14:05", Some(company))
    }

    #[test]
    fn shows_the_company_under_the_title_and_in_the_footer() {
        let rows = rows(120);
        let pages = company_report(&rows, "Consultoria Fictícia Ltda");
        check_pages(&pages);
        let first = texts(&pages[0]);
        assert!(first.contains(&"Consultoria Fictícia Ltda"), "{first:?}");
        assert!(
            first.contains(&" · gerado em 29/09/2026 14:05 pelo nfsetable"),
            "{first:?}"
        );
        for page in &pages {
            let footer = "Relatório de notas · Consultoria Fictícia Ltda · gerado em 29/09/2026 \
                          14:05 pelo nfsetable";
            assert!(texts(page).contains(&footer), "{:?}", texts(page));
        }

        // Without a company, the subtitle and the footer stay as before.
        let pages = report(&rows);
        let first = texts(&pages[0]);
        assert!(first.contains(&"Gerado em 29/09/2026 14:05 pelo nfsetable"));
        assert!(first.contains(&"Relatório de notas · gerado em 29/09/2026 14:05 pelo nfsetable"));
        assert!(!first.iter().any(|text| text.contains("Fictícia")));
    }

    #[test]
    fn cuts_a_long_company_name() {
        let company = "CONSULTORIA E ASSESSORIA EMPRESARIAL FICTÍCIA ".repeat(4);
        let pages = company_report(&rows(3), company.trim());
        // `check_pages` keeps the subtitle and the footer inside the margins.
        check_pages(&pages);
        let first = texts(&pages[0]);
        assert!(
            first
                .iter()
                .any(|text| text.starts_with("CONSULTORIA") && text.ends_with('…')),
            "{first:?}"
        );
    }

    /// Every page of the table repeats its header, every page has its footer, and nothing leaves
    /// the page margins.
    fn check_pages(pages: &[Vec<Op>]) {
        let count = pages.len();
        let total_page = pages
            .iter()
            .position(|page| {
                let texts = texts(page);
                texts.contains(&"TOTAL") || texts.contains(&"TOTAL RECEITAS")
            })
            .expect("total row");
        for (index, page) in pages.iter().enumerate() {
            let texts = texts(page);
            if index <= total_page {
                assert!(texts.contains(&"Arquivo"), "page {index} has no header");
            }
            let footer = format!("Página {} de {count}", index + 1);
            assert!(texts.contains(&footer.as_str()), "page {index}: {texts:?}");
            for op in page {
                match op {
                    Op::Text {
                        x,
                        baseline,
                        text,
                        font,
                        size,
                        ..
                    } => {
                        assert!(*x >= MARGIN_X - 0.01, "{text} at x = {x}");
                        let right = x + text_width(text, *font, *size);
                        assert!(
                            right <= PAGE_WIDTH - MARGIN_X + 0.01,
                            "{text} ends at {right}"
                        );
                        assert!(*baseline > 0.0 && *baseline < PAGE_HEIGHT, "{text}");
                    }
                    Op::Fill { top, height, .. } => {
                        assert!(top + height <= CONTENT_BOTTOM + 0.01, "fill ends at {top}");
                    }
                    Op::Rule { y, .. } => assert!(*y < PAGE_HEIGHT),
                }
            }
        }
    }

    #[test]
    fn paginates_large_tables() {
        let pages = report(&rows(1200));
        assert!((20..=30).contains(&pages.len()), "{} pages", pages.len());
        check_pages(&pages);
        let last = texts(pages.last().unwrap());
        assert!(last.contains(&"TOTAL"));
        assert!(last.contains(&"Totais por tipo"));
        assert!(last.contains(&"Totais por competência"));
        let all: Vec<&str> = pages.iter().flat_map(|page| texts(page)).collect();
        assert!(all.contains(&"NFS-e 1200 EMPRESA EXEMPLO LTDA.pdf"));
        assert!(all.contains(&"1200 notas · Total R$ 6.480.604,80 · 120 fora do total"));
        // Revenue only: no natureza column nor split totals.
        assert!(!all.contains(&"Natureza") && !all.contains(&"SALDO"));
    }

    #[test]
    fn splits_revenue_and_expenses() {
        let mut rows = rows(300);
        for row in rows.iter_mut().skip(3).step_by(4) {
            row.kind = Some(DocKind::Expense);
            row.doc_type = "Boleto".into();
        }
        let summary = Summary::new(&rows);
        let totals = summary.totals;
        let pages = report(&rows);
        check_pages(&pages);
        let first = texts(&pages[0]);
        let amounts = format!(
            "Receitas {} · Despesas {} · Saldo {}",
            format_brl(totals.revenue.cents),
            format_brl(totals.expense.cents),
            format_brl(totals.balance())
        );
        assert!(first.contains(&amounts.as_str()), "{first:?}");
        let counts = format!(
            "300 notas: {} receitas e {} despesas · 30 fora do total",
            totals.revenue.count, totals.expense.count
        );
        assert!(first.contains(&counts.as_str()), "{first:?}");
        for expected in ["Natureza", "Receita", "Despesa"] {
            assert!(first.contains(&expected), "{expected}");
        }
        let all: Vec<&str> = pages.iter().flat_map(|page| texts(page)).collect();
        for expected in [
            "TOTAL RECEITAS",
            "TOTAL DESPESAS",
            "SALDO",
            "Receitas",
            "Despesas",
            "Saldo",
        ] {
            assert!(all.contains(&expected), "{expected}");
        }
        assert!(!all.contains(&"TOTAL"));
        let balance = format_brl(totals.balance());
        assert!(all.contains(&balance.as_str()));
    }

    #[test]
    fn empty_report_has_one_page() {
        let pages = report(&[]);
        assert_eq!(pages.len(), 1);
        let texts = texts(&pages[0]);
        for expected in [
            "Relatório de notas",
            "0 notas · Total R$ 0,00",
            "Nenhuma nota para exportar.",
            "TOTAL",
            "R$ 0,00",
            "Página 1 de 1",
        ] {
            assert!(texts.contains(&expected), "{expected}: {texts:?}");
        }
        assert!(!texts.contains(&"Totais por tipo"));
    }

    #[test]
    fn cuts_long_names_and_keeps_amounts() {
        let mut rows = rows(1);
        rows[0].file = format!("{}.pdf", "NOTA FISCAL DE SERVIÇOS ELETRÔNICA ".repeat(6));
        rows[0].cents = Some(-123_456);
        let pages = report(&rows);
        let texts = texts(&pages[0]);
        let name = texts
            .iter()
            .find(|text| text.starts_with("NOTA FISCAL"))
            .unwrap();
        assert!(name.contains('…') && name.ends_with(".pdf"), "{name}");
        assert!(texts.contains(&"-R$ 1.234,56"));
        assert!(texts.contains(&"1 nota · Total -R$ 1.234,56"));
    }
}

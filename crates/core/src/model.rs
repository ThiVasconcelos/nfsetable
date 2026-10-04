//! Public data model shared by the core, the Tauri backend and the frontend.
//!
//! This module is THE contract of the application: every type here is serialized with serde and
//! mirrored field by field in `app/src/lib/types.ts`. Any change here must be reflected there.
//!
//! Conventions:
//! - JSON keys are camelCase (`rename_all = "camelCase"`).
//! - Data-carrying enums are internally tagged with a `"type"` key whose value is the camelCase
//!   variant name; the fields of their struct variants are camelCase too.
//! - Unit enums serialize as camelCase strings.
//! - `Option::None` serializes as `null`; a missing optional key deserializes as `None`.
//! - Rectangles are in PDF points (1/72 inch), origin at the TOP-LEFT corner of the page, y growing
//!   downward. The only exception is the `rect` of [`Rule::Region`], which is relative to the page
//!   size (0..1).
//! - Page indices are 0-based everywhere (the UI shows `page + 1`).
//! - Money is always integer cents (`i64`); floats are never used for money.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A rectangle in PDF points, origin at the top-left corner of the page, y growing downward.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// How a field value is parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldKind {
    /// Brazilian currency ("R$ 1.234,56"), stored as integer cents.
    Money,
    /// A date ("dd/mm/aaaa"), stored as ISO `yyyy-mm-dd`.
    Date,
    /// Free text.
    Text,
}

/// A field that can be extracted from documents. Built-in fields: `net_value` ("Valor líquido",
/// money, required), `service_value` ("Valor do serviço", money) and `competence`
/// ("Competência", date), the last two used by the tax calculations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub id: String,
    pub label: String,
    pub kind: FieldKind,
    /// Required fields decide the document status: a document is `ok` when all of them were
    /// found. Optional fields are extracted when possible and never make a document fail.
    #[serde(default)]
    pub required: bool,
}

/// Where the value is searched relative to an anchor label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Below,
    Right,
}

/// An extraction rule. Profiles hold, per field, an ordered list of rules (first hit wins).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Rule {
    /// Finds a label and reads the value next to it.
    Anchor {
        /// Regex over NORMALIZED text (lowercase, accents removed, whitespace collapsed to single
        /// spaces). Patterns put `\s*` between words so glued text also matches.
        pattern: String,
        /// Regex tested against the normalized text immediately following the match on the same
        /// line; when it matches, that label occurrence is rejected (Rust regex has no lookahead).
        exclude_suffix: Option<String>,
        direction: Direction,
        /// Maximum distance in points between the label and the value.
        max_distance: f32,
    },
    /// Reads a fixed region of a page, optionally positioned relative to an anchor text.
    Region {
        /// RELATIVE to the page size (0..1), top-left origin.
        rect: Rect,
        /// 0-based page index.
        page: u32,
        anchor: Option<RegionAnchor>,
    },
}

/// Positions a region relative to a text found on the page, so the region follows blocks that
/// move between documents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionAnchor {
    /// Normalized text of the anchor segment.
    pub text: String,
    /// Offset in points from the anchor text box top-left corner to the region top-left corner.
    pub dx: f32,
    pub dy: f32,
    /// Region size in points.
    pub w: f32,
    pub h: f32,
}

/// Whether a document is income (a note the user issued) or an expense (a bill the user paid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DocKind {
    #[default]
    Revenue,
    Expense,
}

/// An extraction profile: how to recognize a kind of document (text fingerprint and/or file name
/// patterns), how to classify it (type and kind) and the rules to read its fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub builtin: bool,
    /// Normalized substrings that must ALL appear in the document's normalized text with ALL
    /// whitespace removed. Empty = matches any document.
    pub fingerprint: Vec<String>,
    /// File name patterns, case- and accent-insensitive; `*` matches any run of characters and
    /// `?` one character, and a pattern without wildcards matches anywhere in the name. When
    /// present, the profile only applies to files whose name matches at least one of them.
    #[serde(default)]
    pub name_patterns: Vec<String>,
    /// Type given to the documents this profile matches (e.g. "Internet", "Aluguel").
    #[serde(default)]
    pub doc_type: Option<String>,
    /// Revenue or expense, for the documents this profile matches.
    #[serde(default)]
    pub kind: Option<DocKind>,
    /// Rules per field id (e.g. `"net_value"`), tried in order.
    pub fields: BTreeMap<String, Vec<Rule>>,
}

/// A folder or a single file chosen by the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Folder or single file path.
    pub path: String,
    /// Recurse into subfolders (folders only).
    pub recursive: bool,
}

/// Input of [`crate::scan`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    pub sources: Vec<Source>,
    /// Case- and accent-insensitive substrings of the FILE NAME; matching files are excluded.
    pub exclude: Vec<String>,
}

/// What was found for one [`Source`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub path: String,
    pub is_dir: bool,
    pub exists: bool,
    /// Number of PDF files found under this source (after exclusion).
    pub file_count: u32,
}

/// A PDF file found by the scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFile {
    /// Full path.
    pub path: String,
    /// File name with extension.
    pub name: String,
    /// Parent folder.
    pub dir: String,
    /// Size in bytes.
    pub size: u64,
    /// SHA-256 of the content, lowercase hex; empty when the file could not be read (locked by
    /// another program, no permission).
    pub hash: String,
    /// Path of the first file (in scan order) with the same hash, if this one is a duplicate.
    pub duplicate_of: Option<String>,
}

/// Output of [`crate::scan`].
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub sources: Vec<SourceInfo>,
    /// Sorted by folder, then file name; canonical paths deduplicated.
    pub files: Vec<ScannedFile>,
    /// Paths of the PDF files skipped by the exclude filter.
    pub excluded: Vec<String>,
}

/// Outcome of the extraction of one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Every requested field has a value.
    Ok,
    /// The document has text, but at least one field has no value.
    NotFound,
    /// No page has extractable text (e.g. a scanned image).
    NoText,
    /// The file could not be opened (corrupt, password protected, unreadable...).
    Error,
}

/// Where a value came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Origin {
    /// A profile rule (built-in or saved by the user).
    Profile { id: String, name: String },
    /// The automatic label heuristic.
    Auto,
    /// A region drawn by the user.
    Region,
    /// Typed by the user.
    Manual,
}

/// A value read from a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldValue {
    /// Text as found in the document (e.g. "R$ 1.234,56").
    pub raw: String,
    /// Parsed amount in cents (money fields).
    pub cents: Option<i64>,
    /// Parsed ISO date `yyyy-mm-dd` (date fields).
    pub date: Option<String>,
    /// Parsed text (text fields).
    pub text: Option<String>,
    pub origin: Origin,
    /// 0-based page index where the value was found.
    pub page: u32,
    /// Box of the value on that page, in points, top-left origin.
    pub bbox: Rect,
}

/// Extraction result for one document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocResult {
    pub path: String,
    pub status: Status,
    /// Human readable detail in pt-BR (mainly for `error`).
    pub message: Option<String>,
    /// Type from the first matching profile that sets one (user profiles first, then the
    /// built-in ones, e.g. "NFS-e"), otherwise "PDF".
    pub doc_type: String,
    /// Revenue or expense, from the first matching profile that sets it (default: revenue).
    #[serde(default)]
    pub kind: DocKind,
    /// Values per field id.
    pub fields: BTreeMap<String, FieldValue>,
    pub page_count: u32,
}

/// Result of reading a region drawn by the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionRead {
    /// Text inside the region (lines joined by a space).
    pub text: String,
    /// Parsed value, if any.
    pub value: Option<FieldValue>,
    /// A region rule (relative rect + anchor when one was found) reproducing this read.
    pub suggested_rule: Rule,
}

/// Result of testing a rule on one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleTest {
    pub path: String,
    pub value: Option<FieldValue>,
    /// pt-BR message when the file could not be processed.
    pub error: Option<String>,
}

/// A rendered page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedPage {
    /// `data:image/png;base64,...`
    pub data_url: String,
    /// Page size in points (as displayed, i.e. after the page rotation).
    pub width_pt: f32,
    pub height_pt: f32,
    pub page_count: u32,
}

/// File format of a table export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    /// UTF-8 with BOM, `;` separated, decimal comma (spreadsheets set to pt-BR).
    Csv,
    /// Excel workbook.
    Xlsx,
    /// Printable report.
    Pdf,
    /// SQL script for PostgreSQL.
    Postgresql,
    /// SQL script for MySQL / MariaDB.
    Mysql,
}

/// One row of a table export. `status` and `origin` are display texts (pt-BR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRow {
    pub file: String,
    pub path: String,
    pub doc_type: String,
    /// Revenue or expense; `None` counts as revenue. Totals are kept apart per kind.
    #[serde(default)]
    pub kind: Option<DocKind>,
    /// Competence month `yyyy-mm`, when known.
    #[serde(default)]
    pub competence: Option<String>,
    /// Amount in cents; `None` for rows without value or left out of the total.
    pub cents: Option<i64>,
    pub status: String,
    pub origin: String,
}

/// Former name of [`ExportRow`], kept for the CSV helpers.
pub type CsvRow = ExportRow;

/// Everything needed to export the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub format: ExportFormat,
    /// Destination file.
    pub path: String,
    pub rows: Vec<ExportRow>,
    /// Local date and time of the export, already formatted for display (e.g. "29/09/2026 14:05").
    pub generated_at: String,
    /// Name of the company the notes belong to, shown in the report headers when given.
    #[serde(default)]
    pub company_name: Option<String>,
}

/// Application information shown by the frontend (and used to detect a missing PDFium).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub pdfium_ok: bool,
    /// pt-BR message when PDFium could not be loaded.
    pub pdfium_error: Option<String>,
    pub profiles_dir: String,
    /// Folder with the persistent data (profiles and stored documents) in use.
    #[serde(default)]
    pub data_dir: String,
    /// The app's own data folder, used when no other folder was chosen.
    #[serde(default)]
    pub default_data_dir: String,
    /// pt-BR warning when the chosen data folder is unavailable (the default one is used).
    #[serde(default)]
    pub data_dir_error: Option<String>,
}

/// Payload of the `extract-progress` and `test-progress` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub done: u32,
    pub total: u32,
    /// Rule test the progress belongs to (`test-progress`); 0 for the extraction.
    #[serde(default)]
    pub run: u32,
}

// ---------------------------------------------------------------- taxes

/// An amount for one month (`yyyy-mm`), in cents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthAmount {
    pub month: String,
    pub cents: i64,
}

/// Tax regime of the provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaxRegime {
    Mei,
    Simples,
    Presumido,
}

/// How the Simples Nacional annex is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SimplesActivity {
    /// Activities subject to the Fator R (software, IT consulting...): Annex III when the payroll
    /// reaches 28% of the revenue, otherwise Annex V.
    FatorR,
    /// Activities always taxed in Annex III.
    AnnexIii,
}

/// Input of the tax report. Amounts in cents; rates as decimals (0.05 = 5%).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxInput {
    /// Month whose tables (minimum wage, INSS, IRRF...) are used, `yyyy-mm`. Normally the most
    /// recent month considered.
    pub reference_month: String,
    /// Gross revenue of the months the user chose to consider (from the notes, after the user's
    /// edits; duplicates and cancelled notes left out). Their average is the "typical month" used
    /// by every estimate: e.g. the first 3 months of a new company give a projection for the year.
    pub revenue_by_month: Vec<MonthAmount>,
    /// Current regime (the one used for "sobra do mês" and pricing).
    pub regime: TaxRegime,
    pub activity: SimplesActivity,
    /// Month the company started (`yyyy-mm`); drives the MEI proportional limit and the Simples
    /// rules for the first months.
    #[serde(default)]
    pub opening_month: Option<String>,
    /// Monthly pró-labore; `None` = automatic: the smallest amount whose Fator R reaches 28% in the
    /// typical month and in every month considered (each with the RBT12 of its 12 previous months),
    /// minus the other payroll and never below the minimum wage.
    #[serde(default)]
    pub pro_labore_cents: Option<i64>,
    /// Revenue of every known month (notes and projections), not only the months considered: the
    /// RBT12 of each month of the cash flow (and the automatic pró-labore) looks at the 12 months
    /// before it. The months considered are added to it.
    #[serde(default)]
    pub revenue_history: Vec<MonthAmount>,
    /// Other monthly payroll that counts for the Fator R (salaries, 13th, FGTS...).
    pub payroll_cents: i64,
    /// Dependents of the partner, for the IRRF on the pró-labore.
    pub dependents: u32,
    /// Fixed monthly costs that apply to every month (yearly items divided by 12), for "sobra do
    /// mês", the comparison, "quanto cobrar" and the cash flow.
    pub monthly_costs_cents: i64,
    /// Fixed costs per month, for items active only from a start month (and until an end month):
    /// added to `monthly_costs_cents`. The typical month uses the reference month's (an item that
    /// ended counts nothing, one that started counts in full); the cash flow uses each month's.
    #[serde(default)]
    pub fixed_costs_by_month: Vec<MonthAmount>,
    /// Variable costs per month (typed by the user or read from expense documents, by competence
    /// month). The typical month uses the reference month's; the cash flow uses each month's.
    #[serde(default)]
    pub costs_by_month: Vec<MonthAmount>,
    /// Monthly revenue used by "sobra do mês" instead of the average: e.g. the average without the
    /// notes typed as bonus, or a fixed amount (only the contracts). `None` = the average.
    #[serde(default)]
    pub leftover_revenue_cents: Option<i64>,
    /// Share of the revenue set aside every month as a reserve (0.10 = 10%): contingencies, 13th
    /// salary and vacation the PJ does not have. Used by "sobra do mês", the comparison and
    /// "quanto cobrar".
    #[serde(default)]
    pub reserve_rate: f64,
    /// Municipal ISS rate for Lucro Presumido (Recife: 0.05).
    pub iss_rate: f64,
    /// Net amount the user wants to keep per month, for "quanto cobrar".
    #[serde(default)]
    pub desired_net_cents: Option<i64>,
}

/// Revenue of the months considered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevenueSummary {
    /// Months considered, oldest first.
    pub months: Vec<MonthAmount>,
    pub months_count: u32,
    pub total_cents: i64,
    /// total / months_count: the "typical month" used by the estimates.
    pub average_monthly_cents: i64,
    /// Revenue of the reference month.
    pub month_cents: i64,
    /// Months considered from January to the reference month of the same year.
    pub year_to_date_cents: i64,
    /// 12-month revenue that picks the Simples bracket: the sum of the 12 most recent months when
    /// 12 or more are considered, otherwise the average × 12 (the rule for new companies).
    pub rbt12_cents: i64,
    /// True when `rbt12_cents` is the average × 12.
    pub rbt12_annualized: bool,
}

/// Where the year's revenue stands against the MEI limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MeiStatus {
    /// Within the limit.
    Within,
    /// Above the limit by up to 20%: leaves the MEI next January and pays the difference.
    UpToTolerance,
    /// More than 20% above: leaves the MEI retroactively to January (or to the opening).
    AboveTolerance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeiReport {
    /// Annual limit (proportional in the opening year).
    pub limit_cents: i64,
    pub year_to_date_cents: i64,
    /// year_to_date / limit.
    pub used_ratio: f64,
    pub status: MeiStatus,
    /// Year to date plus the average for each remaining month of the year ("no ritmo atual").
    pub year_projection_cents: i64,
    /// year_projection / limit.
    pub projection_ratio: f64,
    pub projected_status: MeiStatus,
    /// Monthly DAS-MEI for services.
    pub das_cents: i64,
    /// Software development and IT services are not allowed to the MEI.
    pub it_services_allowed: bool,
}

/// One tax inside the DAS (or inside a regime's total).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxShare {
    /// "irpj", "csll", "cofins", "pis", "cpp", "iss", "inss", "irrf"...
    pub tax: String,
    pub cents: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimplesReport {
    /// Payroll of the window / RBT12, truncated to two decimals like the PGDAS-D.
    pub fator_r: f64,
    /// Payroll of the window (FS12) used for the Fator R.
    pub fs12_cents: i64,
    /// "III" or "V".
    pub annex: String,
    /// 1 to 6.
    pub bracket: u32,
    pub nominal_rate: f64,
    pub deduction_cents: i64,
    pub effective_rate: f64,
    /// DAS of a typical month (average revenue).
    pub das_cents: i64,
    /// DAS split by tax.
    pub split: Vec<TaxShare>,
    /// Monthly pró-labore that brings the Fator R to 28% in the typical month and in every month
    /// considered (the automatic amount), never below the minimum wage.
    pub pro_labore_for_annex_iii_cents: i64,
    /// Room left under the ME limit (R$ 360 mil) in the reference year.
    pub me_limit_remaining_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProLaboreReport {
    pub gross_cents: i64,
    /// True when the amount was chosen automatically for the Fator R.
    pub automatic: bool,
    /// Monthly revenue the automatic amount was based on (RBT12 / 12), and how many months it covers.
    pub basis_average_cents: i64,
    pub basis_months: u32,
    /// The month whose 12 previous months required the automatic amount (the one with the largest
    /// RBT12); `None` when the typical month or the minimum wage decided it.
    pub basis_month: Option<String>,
    /// 11% up to the INSS ceiling, withheld from the partner.
    pub inss_cents: i64,
    pub irrf_cents: i64,
    /// Reduction of Lei 15.270/2025 already applied to `irrf_cents`.
    pub irrf_reduction_cents: i64,
    pub net_cents: i64,
}

/// Cost of one regime for a typical month (average revenue).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeCost {
    /// "mei", "simplesIii", "simplesV" or "presumido".
    pub regime: String,
    /// Display name (pt-BR).
    pub label: String,
    /// False when the regime does not fit (e.g. IT services in the MEI, revenue above a limit).
    pub available: bool,
    /// pt-BR explanation (why not available, assumptions).
    pub note: Option<String>,
    /// Taxes paid by the company (DAS, or IRPJ/CSLL/PIS/Cofins/ISS/CPP).
    pub company_taxes_cents: i64,
    pub company_taxes: Vec<TaxShare>,
    /// Pró-labore assumed in this scenario.
    pub pro_labore_cents: i64,
    /// INSS and IRRF withheld from the pró-labore.
    pub owner_taxes_cents: i64,
    pub total_taxes_cents: i64,
    /// total_taxes / revenue.
    pub total_rate: f64,
    /// Reserve set aside (reserve_rate × revenue).
    #[serde(default)]
    pub reserve_cents: i64,
    /// What is left for the partner: revenue − all taxes − fixed and average variable costs −
    /// reserve.
    pub owner_net_cents: i64,
}

/// "Sobra do mês" in the current regime, for a typical month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverReport {
    /// Monthly revenue used: the average, or `leftover_revenue_cents` when given.
    pub revenue_cents: i64,
    /// True when `leftover_revenue_cents` replaced the average.
    #[serde(default)]
    pub custom_revenue: bool,
    /// Company taxes on that revenue (DAS at the effective rate of the real RBT12).
    pub company_taxes_cents: i64,
    pub pro_labore_inss_cents: i64,
    pub pro_labore_irrf_cents: i64,
    /// Fixed costs of the typical month: `monthly_costs_cents` plus the reference month's
    /// `fixed_costs_by_month`.
    pub costs_cents: i64,
    /// Variable costs of the reference month.
    #[serde(default)]
    pub variable_costs_cents: i64,
    /// reserve_rate × revenue.
    #[serde(default)]
    pub reserve_cents: i64,
    /// revenue − company taxes − INSS − IRRF − fixed and variable costs − reserve: pró-labore net
    /// plus distributable profit.
    pub leftover_cents: i64,
    /// Profit that can be distributed tax free without bookkeeping.
    pub tax_free_distribution_limit_cents: i64,
}

/// One month of the cash flow in the current regime: the month's own revenue, costs and pró-labore.
/// In the Simples Nacional each month also has its own RBT12 (the 12 previous months known to the
/// app), Fator R and annex.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CashFlowMonth {
    /// `yyyy-mm`.
    pub month: String,
    pub revenue_cents: i64,
    /// DAS (or DAS-MEI, or IRPJ/CSLL/PIS/Cofins/ISS/CPP) on the month's revenue.
    pub company_taxes_cents: i64,
    /// INSS and IRRF withheld from the pró-labore (zero for the MEI).
    pub owner_taxes_cents: i64,
    pub fixed_costs_cents: i64,
    pub variable_costs_cents: i64,
    pub reserve_cents: i64,
    /// revenue − taxes − costs − reserve.
    pub leftover_cents: i64,
    /// Gross pró-labore of the month (zero in the MEI), and the INSS and IRRF withheld from it.
    pub pro_labore_cents: i64,
    pub inss_cents: i64,
    pub irrf_cents: i64,
    /// What the company earned: revenue − company taxes − costs − gross pró-labore.
    pub profit_cents: i64,
    /// Simples Nacional: RBT12 of the month (the 12 previous months known to the app, proportional
    /// when there are fewer, the month itself × 12 when there is none).
    pub rbt12_cents: Option<i64>,
    /// Simples Nacional: 12 × (pró-labore + other payroll) / RBT12, truncated like the PGDAS-D.
    pub fator_r: Option<f64>,
    /// Simples Nacional: "III" or "V".
    pub annex: Option<String>,
    /// Simples Nacional with the Fator R: the smallest monthly pró-labore that keeps this month in
    /// Anexo III.
    pub pro_labore_for_annex_iii_cents: Option<i64>,
}

/// "Quanto cobrar": revenue needed to keep the desired net in the current regime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PricingReport {
    pub desired_net_cents: i64,
    pub required_revenue_cents: i64,
    /// pt-BR explanation of the assumptions.
    pub note: String,
}

/// Output of the tax report. Everything is an estimate; amounts in cents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxReport {
    pub reference_month: String,
    pub revenue: RevenueSummary,
    pub mei: MeiReport,
    pub simples: SimplesReport,
    pub pro_labore: ProLaboreReport,
    pub comparison: Vec<RegimeCost>,
    pub leftover: LeftoverReport,
    pub pricing: Option<PricingReport>,
    /// Month by month in the current regime, for the months considered (oldest first).
    #[serde(default)]
    pub cash_flow: Vec<CashFlowMonth>,
    /// pt-BR notes about assumptions and limits of the estimate.
    pub warnings: Vec<String>,
    /// Legal basis of the tables used (pt-BR).
    pub sources: Vec<String>,
}

/// A CNAE activity code and how the Simples Nacional taxes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CnaeInfo {
    /// e.g. "6201-5/01".
    pub code: String,
    /// Description (pt-BR).
    pub description: String,
    /// Annex rule in the Simples Nacional.
    pub activity: SimplesActivity,
    /// Whether the activity is allowed to the MEI, and under which occupation.
    pub mei_allowed: bool,
    pub mei_occupation: Option<String>,
    /// pt-BR caveat, e.g. "⚠ confirmar com contador".
    pub note: Option<String>,
}

/// CNAE codes known to the tax calculations (IT services first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxCatalog {
    pub cnaes: Vec<CnaeInfo>,
    /// Suggested default, e.g. "6201-5/01".
    pub default_cnae: String,
}

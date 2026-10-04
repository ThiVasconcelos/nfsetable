// TypeScript mirror of crates/core/src/model.rs (THE contract between core, backend and UI).
//
// Conventions (see model.rs):
// - JSON keys are camelCase.
// - Data-carrying enums (`Rule`, `Origin`) are internally tagged with a "type" key whose value is
//   the camelCase variant name; they are discriminated unions here.
// - Unit enums are camelCase string unions.
// - `Option<T>` is `T | null` (a missing optional key is also accepted by the backend).
// - Rectangles are in PDF points, origin at the TOP-LEFT of the page, y growing downward. The only
//   exception is `RegionRule.rect`, which is relative to the page size (0..1).
// - Page indices are 0-based everywhere (the UI shows `page + 1`).
// - Money is always integer cents (`i64` in Rust, a safe integer here). Never floats.

/** A rectangle in PDF points, top-left origin, y growing downward. */
export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

/** How a field value is parsed: money as integer cents, dates as ISO `yyyy-mm-dd`, or free text. */
export type FieldKind = 'money' | 'date' | 'text'

/**
 * A field that can be extracted from documents. Built-in fields: `net_value` ("Valor líquido",
 * money, required), `service_value` ("Valor do serviço", money) and `competence` ("Competência",
 * date), the last two used by the tax calculations.
 */
export interface FieldDef {
  id: string
  label: string
  kind: FieldKind
  /**
   * Required fields decide the document status: a document is `ok` when all of them were found.
   * Optional fields are extracted when possible and never make a document fail.
   */
  required: boolean
}

/** Where the value is searched relative to an anchor label. */
export type Direction = 'below' | 'right'

/** Finds a label (regex over normalized text) and reads the value next to it. */
export interface AnchorRule {
  type: 'anchor'
  pattern: string
  excludeSuffix: string | null
  direction: Direction
  /** Maximum distance in points between the label and the value. */
  maxDistance: number
}

/** Reads a fixed region of a page, optionally positioned relative to an anchor text. */
export interface RegionRule {
  type: 'region'
  /** RELATIVE to the page size (0..1), top-left origin. */
  rect: Rect
  /** 0-based page index. */
  page: number
  anchor: RegionAnchor | null
}

/** An extraction rule. Profiles hold, per field, an ordered list of rules (first hit wins). */
export type Rule = AnchorRule | RegionRule

/** Positions a region relative to a text found on the page. */
export interface RegionAnchor {
  /** Normalized text of the anchor segment. */
  text: string
  /** Offset in points from the anchor text box top-left to the region top-left. */
  dx: number
  dy: number
  /** Region size in points. */
  w: number
  h: number
}

/** Whether a document is income (a note the user issued) or an expense (a bill the user paid). */
export type DocKind = 'revenue' | 'expense'

/**
 * An extraction profile: how to recognize a kind of document (text fingerprint and/or file name
 * patterns), how to classify it (type and kind) and the rules to read its fields.
 *
 * Classification uses the user's profiles first, then the built-in ones (so a user rule can say
 * "the DANFSe issued by my accountant is an expense"); values follow the built-in rules, then the
 * user rules, then the heuristic.
 */
export interface Profile {
  id: string
  name: string
  builtin: boolean
  /**
   * Normalized substrings that must ALL appear in the document's normalized text with ALL
   * whitespace removed. Empty = matches any document.
   */
  fingerprint: string[]
  /**
   * File name patterns, case- and accent-insensitive; `*` matches any run of characters and `?`
   * one character, and a pattern without wildcards matches anywhere in the name. When present, the
   * profile only applies to files whose name matches at least one of them.
   */
  namePatterns: string[]
  /** Type given to the documents this profile matches (e.g. "Internet", "Aluguel"). */
  docType: string | null
  /** Revenue or expense, for the documents this profile matches. */
  kind: DocKind | null
  /** Rules per field id (e.g. "net_value"), tried in order. */
  fields: Record<string, Rule[]>
}

/** A folder or a single file chosen by the user. */
export interface Source {
  path: string
  /** Recurse into subfolders (folders only). */
  recursive: boolean
}

/** Input of `scan_sources`. */
export interface ScanOptions {
  sources: Source[]
  /** Case- and accent-insensitive substrings of the FILE NAME; matching files are excluded. */
  exclude: string[]
}

/** What was found for one Source. */
export interface SourceInfo {
  path: string
  isDir: boolean
  exists: boolean
  /** Number of PDF files found under this source (after exclusion). */
  fileCount: number
  /** Folders (or files) below this source that could not be read, e.g. without permission (at most a few). */
  unreadable: string[]
}

/** A PDF file found by the scan. */
export interface ScannedFile {
  path: string
  name: string
  dir: string
  size: number
  /** SHA-256 of the content, lowercase hex; empty when the file could not be read (locked by another program, no permission). */
  hash: string
  /** Path of the first file (in scan order) with the same hash, if this one is a duplicate. */
  duplicateOf: string | null
}

/** Output of `scan_sources`. */
export interface ScanResult {
  sources: SourceInfo[]
  /** Sorted by folder, then file name; canonical paths deduplicated. */
  files: ScannedFile[]
  /** Paths of the PDF files skipped by the exclude filter. */
  excluded: string[]
}

/** Outcome of the extraction of one document. */
export type Status = 'ok' | 'notFound' | 'noText' | 'error'

export interface ProfileOrigin {
  type: 'profile'
  id: string
  name: string
}
export interface AutoOrigin {
  type: 'auto'
}
export interface RegionOrigin {
  type: 'region'
}
export interface ManualOrigin {
  type: 'manual'
}

/** Where a value came from. */
export type Origin = ProfileOrigin | AutoOrigin | RegionOrigin | ManualOrigin

/** A value read from a document. */
export interface FieldValue {
  /** Text as found in the document (e.g. "R$ 1.234,56"). */
  raw: string
  /** Parsed amount in cents (money fields). */
  cents: number | null
  /** Parsed ISO date yyyy-mm-dd (date fields). */
  date: string | null
  /** Parsed text (text fields). */
  text: string | null
  origin: Origin
  /** 0-based page index where the value was found. */
  page: number
  /** Box of the value on that page, in points, top-left origin. */
  bbox: Rect
}

/** Extraction result for one document. */
export interface DocResult {
  path: string
  status: Status
  /** Human readable detail in pt-BR (mainly for `error`). */
  message: string | null
  /**
   * Type from the first matching profile that sets one (user profiles first, then the built-in
   * ones, e.g. "NFS-e"), otherwise "PDF".
   */
  docType: string
  /** Revenue or expense, from the first matching profile that sets it (default: revenue). */
  kind: DocKind
  /** Values per field id. */
  fields: Record<string, FieldValue>
  pageCount: number
}

/** Result of reading a region drawn by the user. */
export interface RegionRead {
  /** Text inside the region (lines joined by a space). */
  text: string
  value: FieldValue | null
  /** A region rule (relative rect + anchor when one was found) reproducing this read. */
  suggestedRule: Rule
}

/** Result of testing a rule on one file. */
export interface RuleTest {
  path: string
  value: FieldValue | null
  /** pt-BR message when the file could not be processed. */
  error: string | null
}

/** A rendered page. */
export interface RenderedPage {
  /** "data:image/png;base64,..." (the browser mock returns an SVG data URL). */
  dataUrl: string
  /** Page size in points (as displayed, i.e. after the page rotation). */
  widthPt: number
  heightPt: number
  pageCount: number
}

/**
 * File format of a table export: `csv` (UTF-8 with BOM, `;` separated, decimal comma), `xlsx`
 * (Excel workbook), `pdf` (printable report), `postgresql` / `mysql` (SQL scripts).
 */
export type ExportFormat = 'csv' | 'xlsx' | 'pdf' | 'postgresql' | 'mysql'

/** One row of a table export. `status` and `origin` are display texts (pt-BR). */
export interface ExportRow {
  file: string
  path: string
  docType: string
  /** Revenue or expense; null counts as revenue. Totals are kept apart per kind. */
  kind: DocKind | null
  /** Competence month `yyyy-mm`, when known. */
  competence: string | null
  /** Amount in cents; null for rows without value or left out of the total. */
  cents: number | null
  status: string
  origin: string
}

/** Former name of ExportRow, kept for the CSV helpers. */
export type CsvRow = ExportRow

/** Everything needed to export the table (`export_table`). */
export interface ExportRequest {
  format: ExportFormat
  /** Destination file. */
  path: string
  rows: ExportRow[]
  /** Local date and time of the export, already formatted for display (e.g. "29/09/2026 14:05"). */
  generatedAt: string
  /** Name of the company the notes belong to, shown in the report headers when given. */
  companyName: string | null
}

/** Application information (also used to detect a missing PDFium). */
export interface AppInfo {
  version: string
  pdfiumOk: boolean
  /** pt-BR message when PDFium could not be loaded. */
  pdfiumError: string | null
  profilesDir: string
  /** Folder with the persistent data (profiles and stored documents) in use. */
  dataDir: string
  /** The app's own data folder, used when no other folder was chosen. */
  defaultDataDir: string
  /** pt-BR warning when the chosen data folder is unavailable (the default one is used). */
  dataDirError: string | null
  /** pt-BR problems of the profile files that could not be loaded (they are left out). */
  profileErrors: string[]
}

/** Payload of the "extract-progress" and "test-progress" events. */
export interface Progress {
  done: number
  total: number
  /** Rule test the progress belongs to (`test-progress`); 0 for the extraction. */
  run: number
}

// ---------------------------------------------------------------- taxes

/** An amount for one month (`yyyy-mm`), in cents. */
export interface MonthAmount {
  month: string
  cents: number
}

/** Tax regime of the provider. */
export type TaxRegime = 'mei' | 'simples' | 'presumido'

/**
 * How the Simples Nacional annex is chosen. `fatorR`: activities subject to the Fator R (software,
 * IT consulting...): Annex III when the payroll reaches 28% of the revenue, otherwise Annex V.
 * `annexIii`: activities always taxed in Annex III.
 */
export type SimplesActivity = 'fatorR' | 'annexIii'

/** Input of the tax report (`tax_report`). Amounts in cents; rates as decimals (0.05 = 5%). */
export interface TaxInput {
  /**
   * Month whose tables (minimum wage, INSS, IRRF...) are used, `yyyy-mm`. Normally the most recent
   * month considered.
   */
  referenceMonth: string
  /**
   * Gross revenue of the months the user chose to consider (from the notes, after the user's
   * edits; duplicates and cancelled notes left out). Their average is the "typical month" used by
   * every estimate: e.g. the first 3 months of a new company give a projection for the year.
   */
  revenueByMonth: MonthAmount[]
  /** Current regime (the one used for "sobra do mês" and pricing). */
  regime: TaxRegime
  activity: SimplesActivity
  /**
   * Month the company started (`yyyy-mm`); drives the MEI proportional limit and the Simples rules
   * for the first months.
   */
  openingMonth: string | null
  /**
   * Monthly pró-labore; null = automatic: the smallest amount whose Fator R reaches 28% in the
   * typical month and in every month considered (each with the RBT12 of its 12 previous months),
   * minus the other payroll and never below the minimum wage.
   */
  proLaboreCents: number | null
  /**
   * Revenue of every known month (notes and projections), not only the months considered: the RBT12
   * of each month of the cash flow (and the automatic pró-labore) looks at the 12 months before it.
   * The months considered are added to it.
   */
  revenueHistory: MonthAmount[]
  /** Other monthly payroll that counts for the Fator R (salaries, 13th, FGTS...). */
  payrollCents: number
  /** Dependents of the partner, for the IRRF on the pró-labore. */
  dependents: number
  /**
   * Fixed monthly costs (the sum of the user's fixed cost items, yearly ones divided by 12), for
   * "sobra do mês", the comparison, "quanto cobrar" and the cash flow.
   */
  monthlyCostsCents: number
  /**
   * Fixed costs per month, for items active only from a start month (and until an end month):
   * added to `monthlyCostsCents`. The typical month uses the reference month's (an item that ended
   * counts nothing, one that started counts in full); the cash flow uses each month's.
   */
  fixedCostsByMonth: MonthAmount[]
  /**
   * Variable costs per month (typed by the user or read from expense documents, by competence
   * month). The typical month uses the reference month's; the cash flow uses each month's.
   */
  costsByMonth: MonthAmount[]
  /**
   * Monthly revenue used by "sobra do mês" instead of the average: e.g. the average without the
   * notes typed as bonus, or a fixed amount (only the contracts). null = the average.
   */
  leftoverRevenueCents: number | null
  /**
   * Share of the revenue set aside every month as a reserve (0.10 = 10%): contingencies, 13th
   * salary and vacation the PJ does not have. Used by "sobra do mês", the comparison and "quanto
   * cobrar".
   */
  reserveRate: number
  /** Municipal ISS rate for Lucro Presumido (Recife: 0.05). */
  issRate: number
  /** Net amount the user wants to keep per month, for "quanto cobrar". */
  desiredNetCents: number | null
}

/** Revenue of the months considered. */
export interface RevenueSummary {
  /** Months considered, oldest first. */
  months: MonthAmount[]
  monthsCount: number
  totalCents: number
  /** total / monthsCount: the "typical month" used by the estimates. */
  averageMonthlyCents: number
  /** Revenue of the reference month. */
  monthCents: number
  /** Months considered from January to the reference month of the same year. */
  yearToDateCents: number
  /** The year to date plus the average for each remaining month of the year ("no ritmo atual"). */
  yearProjectionCents: number
  /**
   * 12-month revenue that picks the Simples bracket: the sum of the 12 most recent months when 12
   * or more are considered, otherwise the average × 12 (the rule for new companies).
   */
  rbt12Cents: number
  /** True when `rbt12Cents` is the average × 12. */
  rbt12Annualized: boolean
}

/**
 * Where the year's revenue stands against the MEI limit. `within`: within the limit.
 * `upToTolerance`: above the limit by up to 20% (leaves the MEI next January and pays the
 * difference). `aboveTolerance`: more than 20% above (leaves the MEI retroactively to January, or to
 * the opening).
 */
export type MeiStatus = 'within' | 'upToTolerance' | 'aboveTolerance'

export interface MeiReport {
  /** Annual limit (proportional in the opening year). */
  limitCents: number
  yearToDateCents: number
  /** yearToDate / limit. */
  usedRatio: number
  status: MeiStatus
  /** Year to date plus the average for each remaining month of the year ("no ritmo atual"). */
  yearProjectionCents: number
  /** yearProjection / limit. */
  projectionRatio: number
  projectedStatus: MeiStatus
  /** Monthly DAS-MEI for services. */
  dasCents: number
  /** Software development and IT services are not allowed to the MEI. */
  itServicesAllowed: boolean
}

/** One tax inside the DAS (or inside a regime's total). */
export interface TaxShare {
  /** "irpj", "csll", "cofins", "pis", "cpp", "iss", "inss", "irrf"... */
  tax: string
  cents: number
}

export interface SimplesReport {
  /** Payroll of the window / RBT12, truncated to two decimals like the PGDAS-D. */
  fatorR: number
  /** Payroll of the window (FS12) used for the Fator R. */
  fs12Cents: number
  /** "III" or "V". */
  annex: string
  /** 1 to 6. */
  bracket: number
  nominalRate: number
  deductionCents: number
  effectiveRate: number
  /** DAS of a typical month (average revenue). */
  dasCents: number
  /** DAS split by tax. */
  split: TaxShare[]
  /**
   * Monthly pró-labore that brings the Fator R to 28% in the typical month and in every month
   * considered (the automatic amount), never below the minimum wage.
   */
  proLaboreForAnnexIiiCents: number
  /** Room left under the ME limit (R$ 360 mil) in the reference year. */
  meLimitRemainingCents: number
  /** Annual limit of the ME (R$ 360 mil) and of the EPP, the ceiling of the Simples Nacional (R$ 4,8 milhões). */
  meLimitCents: number
  eppLimitCents: number
}

export interface ProLaboreReport {
  grossCents: number
  /** True when the amount was chosen automatically for the Fator R. */
  automatic: boolean
  /** Monthly revenue the automatic amount was based on (RBT12 / 12), and how many months it covers. */
  basisAverageCents: number
  basisMonths: number
  /**
   * The month whose 12 previous months required the automatic amount (the one with the largest
   * RBT12); null when the typical month or the minimum wage decided it.
   */
  basisMonth: string | null
  /** 11% up to the INSS ceiling, withheld from the partner. */
  inssCents: number
  irrfCents: number
  /** Reduction of Lei 15.270/2025 already applied to `irrfCents`. */
  irrfReductionCents: number
  netCents: number
}

/** Cost of one regime for a typical month (average revenue). */
export interface RegimeCost {
  /** "mei", "simplesIii", "simplesV" or "presumido". */
  regime: string
  /** Display name (pt-BR). */
  label: string
  /** False when the regime does not fit (e.g. IT services in the MEI, revenue above a limit). */
  available: boolean
  /** pt-BR explanation (why not available, assumptions). */
  note: string | null
  /** Taxes paid by the company (DAS, or IRPJ/CSLL/PIS/Cofins/ISS/CPP). */
  companyTaxesCents: number
  companyTaxes: TaxShare[]
  /** Pró-labore assumed in this scenario. */
  proLaboreCents: number
  /** INSS and IRRF withheld from the pró-labore. */
  ownerTaxesCents: number
  totalTaxesCents: number
  /** totalTaxes / revenue. */
  totalRate: number
  /** Reserve set aside (reserveRate × revenue). */
  reserveCents: number
  /** What is left for the partner: revenue − all taxes − fixed and average variable costs − reserve. */
  ownerNetCents: number
}

/** "Sobra do mês" in the current regime, for a typical month. */
export interface LeftoverReport {
  /** Monthly revenue used: the average, or `leftoverRevenueCents` when given. */
  revenueCents: number
  /** True when `leftoverRevenueCents` replaced the average. */
  customRevenue: boolean
  /** Company taxes on that revenue (DAS at the effective rate of the real RBT12). */
  companyTaxesCents: number
  proLaboreInssCents: number
  proLaboreIrrfCents: number
  /**
   * Fixed costs of the typical month: `monthlyCostsCents` plus the average of `fixedCostsByMonth`
   * over the months considered.
   */
  costsCents: number
  /** Average variable costs of the months considered. */
  variableCostsCents: number
  /** reserveRate × revenue. */
  reserveCents: number
  /**
   * revenue − company taxes − INSS − IRRF − fixed and variable costs − reserve: pró-labore net plus
   * distributable profit.
   */
  leftoverCents: number
  /** Profit that can be distributed tax free without bookkeeping. */
  taxFreeDistributionLimitCents: number
}

/**
 * One month of the cash flow in the current regime: the month's own revenue, costs and pró-labore.
 * In the Simples Nacional each month also has its own RBT12 (the 12 previous months known to the
 * app), Fator R and annex.
 */
export interface CashFlowMonth {
  /** `yyyy-mm`. */
  month: string
  revenueCents: number
  /** DAS (or DAS-MEI, or IRPJ/CSLL/PIS/Cofins/ISS/CPP) on the month's revenue. */
  companyTaxesCents: number
  /** INSS and IRRF withheld from the pró-labore (zero for the MEI). */
  ownerTaxesCents: number
  fixedCostsCents: number
  variableCostsCents: number
  reserveCents: number
  /** revenue − taxes − costs − reserve. */
  leftoverCents: number
  /** Gross pró-labore of the month (zero in the MEI), and the INSS and IRRF withheld from it. */
  proLaboreCents: number
  inssCents: number
  irrfCents: number
  /** What the company earned: revenue − company taxes − costs − gross pró-labore. */
  profitCents: number
  /**
   * Simples Nacional: RBT12 of the month (the 12 previous months known to the app, proportional when
   * there are fewer, the month itself × 12 when there is none). null otherwise.
   */
  rbt12Cents: number | null
  /** Simples Nacional: 12 × (pró-labore + other payroll) / RBT12, truncated like the PGDAS-D. */
  fatorR: number | null
  /** Simples Nacional: "III" or "V". */
  annex: string | null
  /**
   * Simples Nacional with the Fator R: the smallest monthly pró-labore that keeps this month in
   * Anexo III.
   */
  proLaboreForAnnexIiiCents: number | null
}

/** "Quanto cobrar": revenue needed to keep the desired net in the current regime. */
export interface PricingReport {
  desiredNetCents: number
  requiredRevenueCents: number
  /** pt-BR explanation of the assumptions. */
  note: string
}

/** Output of the tax report (`tax_report`). Everything is an estimate; amounts in cents. */
export interface TaxReport {
  referenceMonth: string
  revenue: RevenueSummary
  mei: MeiReport
  simples: SimplesReport
  proLabore: ProLaboreReport
  comparison: RegimeCost[]
  leftover: LeftoverReport
  pricing: PricingReport | null
  /** Month by month in the current regime, for the months considered (oldest first). */
  cashFlow: CashFlowMonth[]
  /** pt-BR notes about assumptions and limits of the estimate. */
  warnings: string[]
  /** Legal basis of the tables used (pt-BR). */
  sources: string[]
}

/** A CNAE activity code and how the Simples Nacional taxes it. */
export interface CnaeInfo {
  /** e.g. "6201-5/01". */
  code: string
  /** Description (pt-BR). */
  description: string
  /** Annex rule in the Simples Nacional. */
  activity: SimplesActivity
  /** Whether the activity is allowed to the MEI, and under which occupation. */
  meiAllowed: boolean
  meiOccupation: string | null
  /** pt-BR caveat, e.g. "⚠ confirmar com contador". */
  note: string | null
}

/** CNAE codes known to the tax calculations (IT services first), from `tax_catalog`. */
export interface TaxCatalog {
  cnaes: CnaeInfo[]
  /** Suggested default, e.g. "6201-5/01". */
  defaultCnae: string
}

// ---------------------------------------------------------------- built-in field ids

/** Net value ("Valor líquido", required): the "Valor" column and the totals. */
export const NET_VALUE_FIELD = 'net_value'
/** Gross value ("Valor do serviço"): the revenue used by the taxes. */
export const SERVICE_VALUE_FIELD = 'service_value'
/** Competence ("Competência", date): the month used by the taxes. */
export const COMPETENCE_FIELD = 'competence'

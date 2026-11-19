// Simplified tax estimates for the browser mock (npm run dev, screenshots). This is NOT the real
// engine (crates/core/src/tax): it only produces plausible numbers with the tables of each year in
// round figures, so the tax page can be exercised without Tauri. The CNAE catalog below is
// synthetic too (real codes, simplified rules).

import { addMonths, formatBRLPlain, formatPercent } from './format'
import type {
  CashFlowMonth,
  MeiStatus,
  RegimeCost,
  SimplesActivity,
  TaxCatalog,
  TaxInput,
  TaxRegime,
  TaxReport,
  TaxRate,
  TaxShare,
} from './types'

// ---------------------------------------------------------------- tables (cents)

const INSS_RATE = 0.11
/** Monthly IRRF table: [base up to, rate, deduction]. */
const IRRF_TABLE: [number, number, number][] = [
  [242_880, 0, 0],
  [282_665, 0.075, 18_216],
  [375_105, 0.15, 39_416],
  [466_468, 0.225, 67_549],
  [Infinity, 0.275, 90_873],
]
const IRRF_DEPENDENT = 18_959
const IRRF_SIMPLIFIED = 60_720
/** Withheld IRRF up to this amount is not withheld (Lei 9.430/1996, art. 67). */
const IRRF_MIN_WITHHOLDING = 1_000
/** Share of the minimum wage in the DAS-MEI (the owner's INSS). */
const MEI_INSS_RATE = 0.05
/** Maximum effective ISS inside the DAS. */
const ISS_CAP = 0.05
/** Canonical order of the taxes of a split (the columns of the tables of the law). */
const TAX_ORDER = ['irpj', 'csll', 'cofins', 'pis', 'cbs', 'cpp', 'iss', 'ibs']
const ME_LIMIT = 36_000_000
const EPP_LIMIT = 480_000_000
const PRESUMPTION = 0.32

interface Bracket {
  upTo: number
  nominal: number
  deduction: number
  split: Record<string, number>
}

const ANNEX_2026: Record<'III' | 'V', Bracket[]> = {
  III: [
    { upTo: 18_000_000, nominal: 0.06, deduction: 0, split: { irpj: 0.04, csll: 0.035, cofins: 0.1282, pis: 0.0278, cpp: 0.434, iss: 0.335 } },
    { upTo: 36_000_000, nominal: 0.112, deduction: 936_000, split: { irpj: 0.04, csll: 0.035, cofins: 0.1405, pis: 0.0305, cpp: 0.434, iss: 0.32 } },
    { upTo: 72_000_000, nominal: 0.135, deduction: 1_764_000, split: { irpj: 0.04, csll: 0.035, cofins: 0.1364, pis: 0.0296, cpp: 0.434, iss: 0.325 } },
    { upTo: 180_000_000, nominal: 0.16, deduction: 3_564_000, split: { irpj: 0.04, csll: 0.035, cofins: 0.1364, pis: 0.0296, cpp: 0.434, iss: 0.325 } },
    { upTo: 360_000_000, nominal: 0.21, deduction: 12_564_000, split: { irpj: 0.04, csll: 0.035, cofins: 0.1282, pis: 0.0278, cpp: 0.434, iss: 0.335 } },
    { upTo: 480_000_000, nominal: 0.33, deduction: 64_800_000, split: { irpj: 0.35, csll: 0.15, cofins: 0.1603, pis: 0.0347, cpp: 0.305 } },
  ],
  V: [
    { upTo: 18_000_000, nominal: 0.155, deduction: 0, split: { irpj: 0.25, csll: 0.15, cofins: 0.141, pis: 0.0305, cpp: 0.2885, iss: 0.14 } },
    { upTo: 36_000_000, nominal: 0.18, deduction: 450_000, split: { irpj: 0.23, csll: 0.15, cofins: 0.141, pis: 0.0305, cpp: 0.2785, iss: 0.17 } },
    { upTo: 72_000_000, nominal: 0.195, deduction: 990_000, split: { irpj: 0.24, csll: 0.15, cofins: 0.1492, pis: 0.0323, cpp: 0.2385, iss: 0.19 } },
    { upTo: 180_000_000, nominal: 0.205, deduction: 1_710_000, split: { irpj: 0.21, csll: 0.15, cofins: 0.1574, pis: 0.0341, cpp: 0.2385, iss: 0.21 } },
    { upTo: 360_000_000, nominal: 0.23, deduction: 6_210_000, split: { irpj: 0.23, csll: 0.125, cofins: 0.141, pis: 0.0305, cpp: 0.2385, iss: 0.235 } },
    { upTo: 480_000_000, nominal: 0.305, deduction: 54_000_000, split: { irpj: 0.35, csll: 0.155, cofins: 0.1644, pis: 0.0356, cpp: 0.295 } },
  ],
}

/** 2027–2028 (LC 214/2025, Anexos XX and XXII): CBS and IBS take the place of Cofins and PIS. */
const ANNEX_2027: Record<'III' | 'V', Bracket[]> = {
  III: [
    { upTo: 18_000_000, nominal: 0.06, deduction: 0, split: { irpj: 0.04, csll: 0.035, cbs: 0.1543, cpp: 0.434, iss: 0.335, ibs: 0.0017 } },
    { upTo: 36_000_000, nominal: 0.112, deduction: 936_000, split: { irpj: 0.04, csll: 0.035, cbs: 0.1691, cpp: 0.434, iss: 0.32, ibs: 0.0019 } },
    { upTo: 72_000_000, nominal: 0.135, deduction: 1_764_000, split: { irpj: 0.04, csll: 0.035, cbs: 0.1641, cpp: 0.434, iss: 0.325, ibs: 0.0019 } },
    { upTo: 180_000_000, nominal: 0.16, deduction: 3_564_000, split: { irpj: 0.04, csll: 0.035, cbs: 0.1641, cpp: 0.434, iss: 0.325, ibs: 0.0019 } },
    { upTo: 360_000_000, nominal: 0.21, deduction: 12_564_000, split: { irpj: 0.04, csll: 0.035, cbs: 0.1543, cpp: 0.434, iss: 0.335, ibs: 0.0017 } },
    { upTo: 480_000_000, nominal: 0.329, deduction: 64_800_000, split: { irpj: 0.3509, csll: 0.1504, cbs: 0.1929, cpp: 0.3058 } },
  ],
  V: [
    { upTo: 18_000_000, nominal: 0.155, deduction: 0, split: { irpj: 0.25, csll: 0.15, cbs: 0.1696, cpp: 0.2885, iss: 0.14, ibs: 0.0019 } },
    { upTo: 36_000_000, nominal: 0.18, deduction: 450_000, split: { irpj: 0.23, csll: 0.15, cbs: 0.1696, cpp: 0.2785, iss: 0.17, ibs: 0.0019 } },
    { upTo: 72_000_000, nominal: 0.195, deduction: 990_000, split: { irpj: 0.24, csll: 0.15, cbs: 0.1795, cpp: 0.2385, iss: 0.19, ibs: 0.002 } },
    { upTo: 180_000_000, nominal: 0.205, deduction: 1_710_000, split: { irpj: 0.21, csll: 0.15, cbs: 0.1894, cpp: 0.2385, iss: 0.21, ibs: 0.0021 } },
    { upTo: 360_000_000, nominal: 0.23, deduction: 6_210_000, split: { irpj: 0.23, csll: 0.125, cbs: 0.1696, cpp: 0.2385, iss: 0.235, ibs: 0.0019 } },
    { upTo: 480_000_000, nominal: 0.304, deduction: 54_000_000, split: { irpj: 0.351, csll: 0.1554, cbs: 0.1978, cpp: 0.2958 } },
  ],
}

/** The tables in force from one month on, like the versioned blocks of the engine's br.json. */
interface Tables {
  /** First month (yyyy-mm). */
  from: string
  minWage: number
  inssCeiling: number
  meiAnnualLimit: number
  meiOpeningMonthlyLimit: number
  /** Fixed amounts of the DAS-MEI of a service provider, on top of the INSS. */
  meiDasFixed: Record<string, number>
  annex: Record<'III' | 'V', Bracket[]>
  /** How the effective rate above the ISS cap is split; absent: in proportion to the other taxes. */
  issExcess: Partial<Record<'III' | 'V', Record<string, number>>>
  /** The IRRF reduction of Lei 15.270/2025 (from 2026). */
  irrfReduction: boolean
  /** The RBT12 window ends this many months before the month (2 from 2027 on). */
  windowOffset: number
  /** From 2027: months of activity taxed at the 1st bracket with a Fator R of 0,28. */
  openingMonths: number
  /** pt-BR warnings on what these tables still lack. */
  pending: string[]
  /** pt-BR caveat on the MEI limit, warned in the MEI. */
  meiNote?: string
}

const TABLES_2026: Tables = {
  from: '2026-01',
  minWage: 162_100,
  inssCeiling: 847_555,
  meiAnnualLimit: 8_100_000,
  meiOpeningMonthlyLimit: 675_000,
  meiDasFixed: { iss: 500 },
  annex: ANNEX_2026,
  issExcess: {
    III: { irpj: 0.0602, csll: 0.0526, cofins: 0.1928, pis: 0.0418, cpp: 0.6526 },
    V: { irpj: 0.3007, csll: 0.1634, cofins: 0.1843, pis: 0.0399, cpp: 0.3117 },
  },
  irrfReduction: true,
  windowOffset: 1,
  openingMonths: 0,
  pending: [],
}
/** LC 238/2026: the MEI limit goes to R$ 110 mil in 2027 and R$ 140 mil from 2028 on. */
const TABLES_2027: Tables = {
  ...TABLES_2026,
  from: '2027-01',
  meiAnnualLimit: 11_000_000,
  meiOpeningMonthlyLimit: 916_667,
  meiNote:
    'Os limites do MEI de 2027 (R$ 110.000,00 por ano, ou R$ 9.166,67 por mês de atividade no ano de abertura) dependem de a lei orçamentária considerar a renúncia de receita (LC 238/2026, art. 3º); sem isso, ficam em R$ 81.000,00 (ou R$ 6.750,00 por mês de atividade no ano de abertura).',
  // LC 214/2025, Anexo XXIII: CBS 0,994 and IBS 0,006, each rounded to the cent.
  meiDasFixed: { iss: 500, cbs: 99, ibs: 1 },
  annex: ANNEX_2027,
  // Anexo V has no excess table from 2027 (the cap is never reached there).
  issExcess: { III: { irpj: 0.0602, csll: 0.0526, cbs: 0.232, cpp: 0.6526, ibs: 0.0026 } },
  windowOffset: 2,
  openingMonths: 2,
  pending: [
    'O salário mínimo e o teto do INSS de 2027 ainda não foram publicados: usados os de 2026.',
    'Em 2027 o PIS e a Cofins acabam e entram a CBS e o IBS; a alíquota de referência da CBS de 2027 (11,3%, calculada pelo TCU) aguarda o Senado, então o Lucro Presumido ainda usa o PIS e a Cofins de 2026 e sai provavelmente subestimado.',
  ],
}
const TABLES_2028: Tables = {
  ...TABLES_2027,
  from: '2028-01',
  meiAnnualLimit: 14_000_000,
  meiOpeningMonthlyLimit: 1_166_667,
  meiNote:
    'Os limites do MEI de 2028 e 2029 (R$ 140.000,00 por ano, ou R$ 11.666,67 por mês de atividade no ano de abertura) dependem de a lei orçamentária de cada ano considerar a renúncia de receita (LC 238/2026, art. 3º); sem isso, ficam em R$ 81.000,00 (ou R$ 6.750,00 por mês de atividade no ano de abertura).',
  pending: [
    'As regras de 2028 em diante estão incompletas: faltam salário mínimo, INSS, o Lucro Presumido com CBS e IBS e, a partir de 2029, as tabelas do Simples Nacional da troca do ISS pelo IBS.',
  ],
}

const MEI_NOTE_2030 =
  'O limite de R$ 140.000,00 supõe que as leis orçamentárias de 2027 a 2029 tenham considerado a renúncia de receita (LC 238/2026, art. 3º).'

/** DAS-MEI from 2029: the ISS falls and the IBS takes its place (no ISS from 2033). */
const TABLES_2029_ON: Tables[] = [
  { ...TABLES_2028, from: '2029-01', meiDasFixed: { iss: 450, cbs: 100, ibs: 20 } },
  // From 2030 the limit assumes the budget laws of 2027 to 2029 took the waiver into account.
  { ...TABLES_2028, from: '2030-01', meiDasFixed: { iss: 400, cbs: 100, ibs: 40 }, meiNote: MEI_NOTE_2030 },
  { ...TABLES_2028, from: '2031-01', meiDasFixed: { iss: 350, cbs: 100, ibs: 60 }, meiNote: MEI_NOTE_2030 },
  { ...TABLES_2028, from: '2032-01', meiDasFixed: { iss: 300, cbs: 100, ibs: 80 }, meiNote: MEI_NOTE_2030 },
  { ...TABLES_2028, from: '2033-01', meiDasFixed: { cbs: 100, ibs: 200 }, meiNote: MEI_NOTE_2030 },
]

/** 2025: the minimum wage and the INSS ceiling of 2025, no IRRF reduction yet. */
const TABLES_2025: Tables = { ...TABLES_2026, from: '2025-01', minWage: 151_800, inssCeiling: 815_741, irrfReduction: false }

/** Oldest first. */
const TABLES: Tables[] = [TABLES_2025, TABLES_2026, TABLES_2027, TABLES_2028, ...TABLES_2029_ON]

/** The tables of `month` (yyyy-mm): the latest that started by then, the first ones before them. */
function tablesFor(month: string): Tables {
  let found = TABLES[0]
  for (const t of TABLES) if (t.from <= month) found = t
  return found
}

/** DAS-MEI of a service provider by tax: 5% of the minimum wage (INSS) plus the fixed amounts. */
function meiDas(t: Tables): TaxShare[] {
  const fixed = Object.entries(t.meiDasFixed).map(([tax, cents]) => ({ tax, cents }))
  return [{ tax: 'inss', cents: Math.round(t.minWage * MEI_INSS_RATE) }, ...fixed]
}

const sum = (values: number[]) => values.reduce((a, b) => a + b, 0)

// ---------------------------------------------------------------- building blocks

function ownerTaxes(gross: number, dependents: number, t: Tables) {
  const inss = Math.round(Math.min(gross, t.inssCeiling) * INSS_RATE)
  const deductions = Math.max(inss + dependents * IRRF_DEPENDENT, IRRF_SIMPLIFIED)
  const base = Math.max(0, gross - deductions)
  const [, rate, deduction] = IRRF_TABLE.find(([upTo]) => base <= upTo)!
  const tax = Math.max(0, Math.round(base * rate - deduction))
  // Lei 15.270/2025 (from 2026): full reduction up to R$ 5.000, linear phase-out up to R$ 7.350.
  let reduction = 0
  if (!t.irrfReduction) reduction = 0
  else if (gross <= 500_000) reduction = Math.min(tax, 31_289)
  else if (gross <= 735_000) reduction = Math.min(tax, Math.max(0, Math.round(97_862 - 0.133145 * gross)))
  const withheld = tax - reduction
  const irrf = withheld <= IRRF_MIN_WITHHOLDING ? 0 : withheld
  return { inss, irrf, reduction, net: gross - inss - irrf }
}

/** Splits `total` by shares, fixing the rounding on the largest share. */
function splitBy(total: number, shares: Record<string, number>): TaxShare[] {
  const out = Object.entries(shares).map(([tax, share]) => ({ tax, cents: Math.round(total * share) }))
  const diff = total - sum(out.map((s) => s.cents))
  if (diff && out.length) out.reduce((a, b) => (b.cents > a.cents ? b : a)).cents += diff
  return out
}

function simples(annex: 'III' | 'V', rbt12: number, revenue: number, t: Tables) {
  const table = t.annex[annex]
  let index = table.findIndex((b) => rbt12 <= b.upTo)
  if (index < 0) index = table.length - 1
  const b = table[index]
  const rbt = Math.max(rbt12, 1)
  const effective = Math.max(0, (rbt * b.nominal - b.deduction) / rbt)
  const das = Math.round(revenue * effective)
  const iss = b.split.iss
  let split: TaxShare[]
  let rates: TaxRate[] = Object.entries(b.split).map(([tax, share]) => ({ tax, rate: effective * share }))
  if (iss != null && effective * iss > ISS_CAP) {
    // The ISS stays at the cap; the rest goes by the excess table or in proportion to the others.
    const capped = Math.round(revenue * ISS_CAP)
    const others = Object.fromEntries(Object.entries(b.split).filter(([tax]) => tax !== 'iss'))
    const otherTotal = sum(Object.values(others))
    const excess = t.issExcess[annex] ?? Object.fromEntries(Object.entries(others).map(([tax, s]) => [tax, s / otherTotal]))
    split = [...splitBy(das - capped, excess), { tax: 'iss', cents: capped }]
    split.sort((a, b) => TAX_ORDER.indexOf(a.tax) - TAX_ORDER.indexOf(b.tax))
    rates = [
      ...Object.entries(excess).map(([tax, share]) => ({ tax, rate: (effective - ISS_CAP) * share })),
      { tax: 'iss', rate: ISS_CAP },
    ].sort((a, b) => TAX_ORDER.indexOf(a.tax) - TAX_ORDER.indexOf(b.tax))
  } else split = splitBy(das, b.split)
  // The 6th bracket has no ISS inside the DAS: it is paid to the municipality.
  return { bracket: index + 1, nominal: b.nominal, deduction: b.deduction, effective, das, split, rates, issOutside: iss == null }
}

function presumido(revenue: number, proLabore: number, issRate: number) {
  const base = revenue * PRESUMPTION
  const shares: TaxShare[] = [
    { tax: 'irpj', cents: Math.round(base * 0.15 + Math.max(0, base - 2_000_000) * 0.1) },
    { tax: 'csll', cents: Math.round(base * 0.09) },
    { tax: 'pis', cents: Math.round(revenue * 0.0065) },
    { tax: 'cofins', cents: Math.round(revenue * 0.03) },
    { tax: 'iss', cents: Math.round(revenue * issRate) },
    { tax: 'cpp', cents: Math.round(proLabore * 0.2) },
  ]
  return { shares, total: sum(shares.map((s) => s.cents)), base: Math.round(base) }
}

function meiStatus(ratio: number): MeiStatus {
  return ratio <= 1 ? 'within' : ratio <= 1.2 ? 'upToTolerance' : 'aboveTolerance'
}

function share(shares: TaxShare[], tax: string): number {
  return shares.find((s) => s.tax === tax)?.cents ?? 0
}

/** `ceil(a / b)` for integers (b > 0). */
const ceilDiv = (a: number, b: number) => -Math.floor(-a / b)

/**
 * Fator R = FS12 / RBT12, truncated to two decimals like the PGDAS-D (integer math): 0.01 without
 * payroll, 0.28 with payroll and no revenue.
 */
function fatorROf(fs12: number, rbt12: number): number {
  if (fs12 <= 0) return 0.01
  if (rbt12 <= 0) return 0.28
  return Math.floor((fs12 * 100) / rbt12) / 100
}

/** Anexo III for activities always taxed there or with the Fator R at 28%, else Anexo V. */
const annexFor = (activity: SimplesActivity, fatorR: number): 'III' | 'V' =>
  activity === 'annexIii' || fatorR >= 0.28 ? 'III' : 'V'

/** 28% of a monthly revenue (RBT12 / 12) minus the other payroll, rounded up (integer math). */
const annexIiiNeed = (rbt12: number, payroll: number) => ceilDiv(28 * rbt12, 1200) - payroll

// ---------------------------------------------------------------- one scenario for a typical month

interface Scenario {
  companyTaxes: TaxShare[]
  companyTotal: number
  proLabore: number
  inss: number
  irrf: number
  taxFreeLimit: number
  annex: 'III' | 'V' | null
}

interface Context {
  /** Tables of the month computed. */
  tables: Tables
  activity: SimplesActivity
  payroll: number
  dependents: number
  issRate: number
  /** Fixed pró-labore, or null for automatic. */
  fixedProLabore: number | null
}

function autoProLabore(average: number, payroll: number, t: Tables): number {
  // Integer math (no float noise), rounded up so the Fator R is not truncated below 28%.
  return Math.max(t.minWage, Math.ceil((average * 28) / 100) - payroll)
}

function scenario(regime: TaxRegime, average: number, rbt12: number, ctx: Context): Scenario {
  if (regime === 'mei') {
    const das = meiDas(ctx.tables)
    return {
      companyTaxes: das,
      companyTotal: sum(das.map((s) => s.cents)),
      proLabore: 0,
      inss: 0,
      irrf: 0,
      taxFreeLimit: Math.round(average * PRESUMPTION),
      annex: null,
    }
  }
  if (regime === 'presumido') {
    const proLabore = ctx.fixedProLabore ?? ctx.tables.minWage
    const p = presumido(average, proLabore, ctx.issRate)
    const o = ownerTaxes(proLabore, ctx.dependents, ctx.tables)
    const federal = share(p.shares, 'irpj') + share(p.shares, 'csll') + share(p.shares, 'pis') + share(p.shares, 'cofins')
    return { companyTaxes: p.shares, companyTotal: p.total, proLabore, inss: o.inss, irrf: o.irrf, taxFreeLimit: Math.max(0, p.base - federal), annex: null }
  }
  return simplesScenario(average, rbt12, ctx, ctx.fixedProLabore ?? autoProLabore(average, ctx.payroll, ctx.tables))
}

/** Simples Nacional with a given pró-labore; the annex follows the Fator R unless forced. */
function simplesScenario(
  average: number,
  rbt12: number,
  ctx: Context,
  proLabore: number,
  forceAnnex?: 'III' | 'V',
): Scenario {
  const annex = forceAnnex ?? annexFor(ctx.activity, fatorROf((proLabore + ctx.payroll) * 12, rbt12))
  const s = simples(annex, rbt12, average, ctx.tables)
  const o = ownerTaxes(proLabore, ctx.dependents, ctx.tables)
  const outside = s.issOutside ? [{ tax: 'iss', cents: Math.round(average * ctx.issRate) }] : []
  return {
    companyTaxes: [...s.split, ...outside],
    companyTotal: s.das + sum(outside.map((x) => x.cents)),
    proLabore,
    inss: o.inss,
    irrf: o.irrf,
    taxFreeLimit: Math.max(0, Math.round(average * PRESUMPTION) - share(s.split, 'irpj')),
    annex,
  }
}

/** Revenue − company taxes − INSS − IRRF − costs − reserve. */
function leftoverOf(revenue: number, costs: number, s: Scenario, reserveRate: number): number {
  return revenue - s.companyTotal - s.inss - s.irrf - costs - Math.round(revenue * reserveRate)
}

// ---------------------------------------------------------------- report

export function mockTaxReport(input: TaxInput): TaxReport {
  const months = [...input.revenueByMonth].sort((a, b) => (a.month < b.month ? -1 : a.month > b.month ? 1 : 0))
  if (!months.length) throw 'Escolha pelo menos um mês com faturamento.'
  const ref = input.referenceMonth
  const refTables = tablesFor(ref)
  const refYear = ref.slice(0, 4)
  const refMonth = Number(ref.slice(5, 7))
  const count = months.length
  const total = sum(months.map((m) => m.cents))
  const average = Math.round(total / count)
  const yearToDate = sum(months.filter((m) => m.month.startsWith(refYear) && m.month <= ref).map((m) => m.cents))
  const annualized = count < 12
  // Annualized from the exact mean, like core (not the rounded average).
  const rbt12 = annualized ? Math.round((total * 12) / count) : sum(months.slice(-12).map((m) => m.cents))
  const reserveRate = Math.min(0.9, Math.max(0, input.reserveRate ?? 0))

  // Fixed costs: every month (undated items) plus the dated items month by month; variable costs
  // per month (duplicates summed, negatives as zero). The typical month uses the reference month's
  // (never an average), the cash flow each month's.
  const byMonth = (list: { month: string; cents: number }[] | undefined) => {
    const map = new Map<string, number>()
    for (const c of list ?? []) map.set(c.month, (map.get(c.month) ?? 0) + c.cents)
    for (const [month, cents] of map) if (cents < 0) map.set(month, 0)
    return map
  }
  const inReference = (map: Map<string, number>) => map.get(ref) ?? 0
  const everyMonth = Math.max(0, input.monthlyCostsCents)
  const fixedByMonth = byMonth(input.fixedCostsByMonth)
  const variableByMonth = byMonth(input.costsByMonth)
  const fixed = everyMonth + inReference(fixedByMonth)
  const variableTypical = inReference(variableByMonth)
  const costs = fixed + variableTypical

  const ctx: Context = {
    tables: refTables,
    activity: input.activity,
    payroll: input.payrollCents,
    dependents: input.dependents,
    issRate: input.issRate,
    fixedProLabore: input.proLaboreCents,
  }

  // RBT12 of each month: the known months (notes and projections, plus the months considered) among
  // the 12 of its window (from 2027, the 12 before the previous month), from the opening;
  // proportional when fewer, the month itself × 12 when none.
  const opening = input.openingMonth
  const known = new Map<string, number>()
  for (const m of input.revenueHistory ?? []) known.set(m.month, (known.get(m.month) ?? 0) + m.cents)
  for (const m of months) known.set(m.month, m.cents)
  const monthIndex = (month: string) => Number(month.slice(0, 4)) * 12 + Number(month.slice(5, 7))
  /** One of the first months of activity taxed at the 1st bracket (from 2027). */
  const openingRule = (month: string) =>
    !!opening && monthIndex(month) - monthIndex(opening) >= 0 && monthIndex(month) - monthIndex(opening) < tablesFor(month).openingMonths
  const rbt12Of = (month: string, revenue: number) => {
    if (openingRule(month)) return { rbt12: 0, n: 0 }
    let windowSum = 0
    let n = 0
    const offset = tablesFor(month).windowOffset
    for (let i = offset; i < offset + 12; i++) {
      const m = addMonths(month, -i)
      const cents = known.get(m)
      if (cents == null || (opening && m < opening)) continue
      windowSum += cents
      n++
    }
    return { rbt12: n === 0 ? revenue * 12 : n === 12 ? windowSum : Math.round((windowSum * 12) / n), n }
  }

  // Pró-labore of the user's choice. Automatic: the least that keeps the typical month and every
  // month considered in Anexo III (28% of its RBT12 / 12, minus the other payroll), at least the
  // minimum wage. The binding month (ties: the latest) explains it when it decides.
  const automatic = input.proLaboreCents == null
  const typicalNeed = annexIiiNeed(rbt12, input.payrollCents)
  let monthNeed = -Infinity
  let binding: { month: string; rbt12: number; n: number } | null = null
  for (const m of months) {
    const w = rbt12Of(m.month, m.cents)
    const need = annexIiiNeed(w.rbt12, input.payrollCents)
    if (need >= monthNeed) {
      monthNeed = need
      binding = { month: m.month, ...w }
    }
  }
  const forAnnexIii = Math.max(refTables.minWage, typicalNeed, monthNeed)
  const basis = binding && monthNeed > Math.max(refTables.minWage, typicalNeed) ? binding : null
  const gross = input.proLaboreCents ?? forAnnexIii
  // The pró-labore of the current regime, like the engine's report: in the Lucro Presumido without a
  // typed one, one minimum wage (no Fator R); the Simples estimates keep `gross`.
  const ownerGross = input.regime === 'presumido' ? (input.proLaboreCents ?? refTables.minWage) : gross
  const owner = ownerTaxes(ownerGross, input.dependents, refTables)

  // Simples with the user's pró-labore.
  const fs12 = (gross + input.payrollCents) * 12
  const fatorR = fatorROf(fs12, rbt12)
  const annex = annexFor(input.activity, fatorR)
  const s = simples(annex, rbt12, average, refTables)

  // MEI.
  const limit =
    opening && opening.slice(0, 4) === refYear
      ? Math.min(refTables.meiOpeningMonthlyLimit * (13 - Number(opening.slice(5, 7))), refTables.meiAnnualLimit)
      : refTables.meiAnnualLimit
  const projection = yearToDate + average * (12 - refMonth)
  const status = meiStatus(yearToDate / limit)
  const projectedStatus = meiStatus(projection / limit)

  // Comparison for a typical month.
  const tier = rbt12 <= ME_LIMIT ? 'ME' : 'EPP'
  const cost = (
    regime: string,
    label: string,
    available: boolean,
    note: string | null,
    sc: Scenario,
  ): RegimeCost => {
    const ownerTotal = sc.inss + sc.irrf
    const totalTaxes = sc.companyTotal + ownerTotal
    const reserve = Math.round(average * reserveRate)
    return {
      regime,
      label,
      available,
      note,
      companyTaxesCents: sc.companyTotal,
      companyTaxes: sc.companyTaxes,
      proLaboreCents: sc.proLabore,
      ownerTaxesCents: ownerTotal,
      totalTaxesCents: totalTaxes,
      totalRate: average > 0 ? totalTaxes / average : 0,
      reserveCents: reserve,
      ownerNetCents: average - totalTaxes - costs - reserve,
    }
  }

  const meiSc = scenario('mei', average, rbt12, ctx)
  const meiNote =
    (input.activity === 'fatorR'
      ? 'Desenvolvimento de software e serviços de TI não podem ser MEI.'
      : 'DAS fixo; o INSS do titular já está no DAS.') +
    (projectedStatus !== 'within'
      ? ` No ritmo atual, a receita do ano chega a ${formatBRLPlain(projection)}, acima do limite do MEI (${formatBRLPlain(limit)}).`
      : '') +
    (refTables.meiNote ? ` ${refTables.meiNote}` : '')
  // Annex III needs the Fator R at 28% (for Fator R activities); Annex V is the cheapest with the
  // minimum wage (or the fixed pró-labore).
  const iiiSc = simplesScenario(average, rbt12, ctx, input.activity === 'fatorR' ? Math.max(gross, forAnnexIii) : gross, 'III')
  // Anexo V: the typed pró-labore when it is below the automatic one (or the activity is always
  // in Anexo III), otherwise one minimum wage.
  const typed = input.proLaboreCents
  const vTyped = typed != null && (typed < forAnnexIii || input.activity !== 'fatorR')
  const vPl = vTyped ? typed : refTables.minWage
  const vSc = simplesScenario(average, rbt12, ctx, vPl, 'V')
  const presSc = scenario('presumido', average, rbt12, ctx)
  const overLimit = rbt12 > EPP_LIMIT
  const comparison: RegimeCost[] = [
    cost('mei', 'MEI', input.activity === 'annexIii' && projectedStatus === 'within', meiNote, meiSc),
    cost(
      'simplesIii',
      `Simples Nacional (${tier}) – Anexo III`,
      !overLimit,
      input.activity === 'fatorR' ? `Com pró-labore de ${formatBRLPlain(iiiSc.proLabore)} (Fator R de 28%).` : null,
      iiiSc,
    ),
    cost(
      'simplesV',
      `Simples Nacional (${tier}) – Anexo V`,
      !overLimit && input.activity === 'fatorR',
      input.activity !== 'fatorR'
        ? 'Esta atividade é sempre tributada no Anexo III.'
        : vTyped
          ? `Com o pró-labore informado (${formatBRLPlain(vSc.proLabore)}) o Fator R fica abaixo de 28% e a receita vai para o Anexo V.`
          : `Com pró-labore de um salário mínimo (${formatBRLPlain(vSc.proLabore)}), Fator R abaixo de 28%.`,
      vSc,
    ),
    cost('presumido', 'Lucro Presumido', true, `ISS de ${formatPercent(input.issRate)} e CPP de 20% sobre o pró-labore.`, presSc),
  ]

  // "Sobra do mês" in the current regime, on the chosen revenue (the average, the average without
  // bonus or a fixed amount). The pró-labore and the rates stay the ones of the real revenue.
  const real = scenario(input.regime, average, rbt12, { ...ctx, fixedProLabore: input.regime === 'simples' ? gross : ctx.fixedProLabore })
  const customRevenue = input.leftoverRevenueCents != null
  const leftoverRevenue = customRevenue ? Math.max(0, input.leftoverRevenueCents!) : average
  const current = customRevenue
    ? scenario(input.regime, leftoverRevenue, rbt12, { ...ctx, fixedProLabore: real.proLabore })
    : real
  const leftover = {
    revenueCents: leftoverRevenue,
    customRevenue,
    companyTaxesCents: current.companyTotal,
    proLaboreInssCents: current.inss,
    proLaboreIrrfCents: current.irrf,
    costsCents: fixed,
    variableCostsCents: variableTypical,
    reserveCents: Math.round(leftoverRevenue * reserveRate),
    leftoverCents: leftoverOf(leftoverRevenue, costs, current, reserveRate),
    taxFreeDistributionLimitCents: current.taxFreeLimit,
  }

  // Month by month in the current regime, at the tables of each month: its revenue, costs and
  // pró-labore. In the Simples each month has its own RBT12, Fator R and annex (and the pró-labore
  // that keeps it in Anexo III).
  const cashFlow: CashFlowMonth[] = months.map((m) => {
    const revenue = m.cents
    const t = tablesFor(m.month)
    const plInUse = input.regime === 'mei' ? 0 : input.regime === 'presumido' ? (input.proLaboreCents ?? t.minWage) : gross
    const plTaxes = input.regime === 'mei' ? { inss: 0, irrf: 0 } : ownerTaxes(plInUse, input.dependents, t)
    let company: number
    let simplesMonth: Pick<CashFlowMonth, 'rbt12Cents' | 'fatorR' | 'annex' | 'proLaboreForAnnexIiiCents'> = {
      rbt12Cents: null,
      fatorR: null,
      annex: null,
      proLaboreForAnnexIiiCents: null,
    }
    if (input.regime === 'mei') company = sum(meiDas(t).map((s) => s.cents))
    else if (input.regime === 'presumido') company = presumido(revenue, plInUse, input.issRate).total
    else {
      const { rbt12: monthRbt12 } = rbt12Of(m.month, revenue)
      const monthFatorR = openingRule(m.month) ? 0.28 : fatorROf(12 * (plInUse + input.payrollCents), monthRbt12)
      const monthAnnex = annexFor(input.activity, monthFatorR)
      const das = simples(monthAnnex, monthRbt12, revenue, t)
      // 6th bracket: the ISS is paid to the municipality, outside the DAS.
      company = das.das + (das.issOutside ? Math.round(revenue * input.issRate) : 0)
      simplesMonth = {
        rbt12Cents: monthRbt12,
        fatorR: monthFatorR,
        annex: monthAnnex,
        proLaboreForAnnexIiiCents:
          input.activity === 'fatorR' ? Math.max(0, annexIiiNeed(monthRbt12, input.payrollCents)) : null,
      }
    }
    const ownerTotal = plTaxes.inss + plTaxes.irrf
    const variable = variableByMonth.get(m.month) ?? 0
    const monthFixed = everyMonth + (fixedByMonth.get(m.month) ?? 0)
    const reserve = Math.round(revenue * reserveRate)
    return {
      month: m.month,
      revenueCents: revenue,
      companyTaxesCents: company,
      ownerTaxesCents: ownerTotal,
      fixedCostsCents: monthFixed,
      variableCostsCents: variable,
      reserveCents: reserve,
      leftoverCents: revenue - company - ownerTotal - monthFixed - variable - reserve,
      proLaboreCents: plInUse,
      inssCents: plTaxes.inss,
      irrfCents: plTaxes.irrf,
      profitCents: revenue - company - monthFixed - variable - plInUse,
      ...simplesMonth,
    }
  })

  // "Quanto cobrar": bisection on the monthly revenue of the current regime.
  let pricing: TaxReport['pricing'] = null
  if (input.desiredNetCents != null) {
    const desired = input.desiredNetCents
    // In the Simples the price always assumes the automatic pró-labore, like the engine.
    const pricingCtx = input.regime === 'simples' ? { ...ctx, fixedProLabore: null } : ctx
    const netFor = (revenue: number) =>
      leftoverOf(revenue, costs, scenario(input.regime, revenue, revenue * 12, pricingCtx), reserveRate)
    let lo = 0
    let hi = Math.max(100_000, (desired + costs) * 4)
    for (let i = 0; i < 60 && netFor(hi) < desired; i++) hi *= 2
    for (let i = 0; i < 60; i++) {
      const mid = Math.floor((lo + hi) / 2)
      if (netFor(mid) >= desired) hi = mid
      else lo = mid + 1
    }
    pricing = {
      desiredNetCents: desired,
      requiredRevenueCents: hi,
      note:
        `Faturamento mensal para sobrarem ${formatBRLPlain(desired)} depois dos impostos, dos custos` +
        (reserveRate > 0 ? ` e da reserva de ${formatPercent(reserveRate)}` : '') +
        ', no regime atual' +
        (input.regime === 'mei'
          ? '.'
          : input.regime === 'simples' || automatic
            ? ', com o pró-labore automático.'
            : ', com o pró-labore informado.') +
        (reserveRate > 0 ? '' : ' Sem reserva para 13º, férias e imprevistos.'),
    }
  }

  const warnings: string[] = []
  if (input.regime === 'mei' && refTables.meiNote) warnings.push(refTables.meiNote)
  if (annualized)
    warnings.push(
      `Com ${count === 1 ? '1 mês' : `${count} meses`}, o RBT12 foi estimado pela média × 12 (regra das empresas novas).`,
    )
  if (input.activity === 'fatorR' && annex === 'V')
    warnings.push(
      `Fator R de ${formatPercent(fatorR)} (abaixo de 28%): Anexo V. Com pró-labore de ${formatBRLPlain(forAnnexIii)} por mês, a empresa vai para o Anexo III.`,
    )
  if (projectedStatus !== 'within')
    warnings.push(`No ritmo atual, o faturamento do ano passa do limite do MEI (${formatBRLPlain(limit)}).`)
  warnings.push('Não considera retenções na fonte (IRRF, CSRF, ISS retido), 13º, férias nem outras receitas.')
  if (refTables.pending.length) warnings.push(...refTables.pending)
  else
    warnings.push(
      `Tabelas de ${refTables.from.slice(0, 4)}: salário mínimo de ${formatBRLPlain(refTables.minWage)} e teto do INSS de ${formatBRLPlain(refTables.inssCeiling)}.`,
    )

  return {
    referenceMonth: ref,
    revenue: {
      months,
      monthsCount: count,
      totalCents: total,
      averageMonthlyCents: average,
      monthCents: months.find((m) => m.month === ref)?.cents ?? 0,
      yearToDateCents: yearToDate,
      yearProjectionCents: projection,
      rbt12Cents: rbt12,
      rbt12Annualized: annualized,
    },
    mei: {
      limitCents: limit,
      yearToDateCents: yearToDate,
      usedRatio: yearToDate / limit,
      status,
      yearProjectionCents: projection,
      projectionRatio: projection / limit,
      projectedStatus,
      dasCents: meiSc.companyTotal,
      dasSplit: meiSc.companyTaxes,
      itServicesAllowed: false,
    },
    simples: {
      fatorR,
      fs12Cents: fs12,
      annex,
      bracket: s.bracket,
      nominalRate: s.nominal,
      deductionCents: s.deduction,
      effectiveRate: s.effective,
      dasCents: s.das,
      split: s.split,
      rates: s.rates,
      proLaboreForAnnexIiiCents: forAnnexIii,
      meLimitRemainingCents: Math.max(0, ME_LIMIT - yearToDate),
      meLimitCents: ME_LIMIT,
      eppLimitCents: EPP_LIMIT,
    },
    proLabore: {
      grossCents: ownerGross,
      automatic,
      basisAverageCents: Math.round((basis ? basis.rbt12 : rbt12) / 12),
      basisMonths: basis ? Math.max(1, basis.n) : Math.min(count, 12),
      basisMonth: input.regime === 'presumido' ? null : (basis?.month ?? null),
      inssCents: owner.inss,
      irrfCents: owner.irrf,
      irrfReductionCents: owner.reduction,
      netCents: owner.net,
    },
    comparison,
    leftover,
    pricing,
    cashFlow,
    warnings,
    sources: [
      'LC 123/2006, art. 18 e Anexos III e V; Res. CGSN 140/2018 (Simples Nacional)',
      'LC 123/2006, art. 18-A; Res. CGSN 140/2018, Anexo XI (MEI)',
      'Lei 8.212/1991, art. 21 e Portaria MPS/MF 13/2026 (INSS)',
      'Lei 11.482/2007 e Lei 15.270/2025 (IRRF e redução)',
      'Decreto 12.797/2025 (salário mínimo de 2026)',
      'Lei 9.249/1995 e IN RFB 1.700/2017 (Lucro Presumido)',
      'Lei Municipal 15.563/1991 do Recife (ISS)',
    ],
  }
}

// ---------------------------------------------------------------- CNAE catalog

const it = (code: string, description: string, note: string | null = null) => ({
  code,
  description,
  activity: 'fatorR' as const,
  meiAllowed: false,
  meiOccupation: null,
  note,
})

const mei = (code: string, description: string, occupation: string) => ({
  code,
  description,
  activity: 'annexIii' as const,
  meiAllowed: true,
  meiOccupation: occupation,
  note: null,
})

export const MOCK_TAX_CATALOG: TaxCatalog = {
  defaultCnae: '6201-5/01',
  cnaes: [
    it('6201-5/01', 'Desenvolvimento de programas de computador sob encomenda'),
    it('6201-5/02', 'Web design'),
    it('6202-3/00', 'Desenvolvimento e licenciamento de programas de computador customizáveis'),
    it('6203-1/00', 'Desenvolvimento e licenciamento de programas de computador não customizáveis'),
    it('6204-0/00', 'Consultoria em tecnologia da informação'),
    it('6209-1/00', 'Suporte técnico, manutenção e outros serviços em tecnologia da informação'),
    it('6311-9/00', 'Tratamento de dados, provedores de serviços de aplicação e serviços de hospedagem na internet'),
    it('6319-4/00', 'Portais, provedores de conteúdo e outros serviços de informação na internet'),
    it(
      '7020-4/00',
      'Atividades de consultoria em gestão empresarial, exceto consultoria técnica específica',
      'Confirme o anexo com o contador: depende do serviço descrito nas notas.',
    ),
    it(
      '7490-1/99',
      'Outras atividades profissionais, científicas e técnicas não especificadas anteriormente',
      '⚠ Atividade genérica: confirme o anexo com o contador.',
    ),
    mei('8599-6/03', 'Treinamento em informática', 'Instrutor(a) de informática independente'),
    mei(
      '9511-8/00',
      'Reparação e manutenção de computadores e de equipamentos periféricos',
      'Técnico(a) de manutenção de computador independente',
    ),
    mei(
      '8219-9/99',
      'Preparação de documentos e serviços especializados de apoio administrativo não especificados anteriormente',
      'Digitador(a) independente',
    ),
  ],
}

// Simplified tax estimates for the browser mock (npm run dev, screenshots). This is NOT the real
// engine (crates/core/src/tax): it only produces plausible numbers with the 2026 tables in round
// figures, so the tax page can be exercised without Tauri. The CNAE catalog below is synthetic too
// (real codes, simplified rules).

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
  TaxShare,
} from './types'

// ---------------------------------------------------------------- 2026 tables (cents)

const MIN_WAGE = 162_100
const INSS_CEILING = 847_555
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
const MEI_ANNUAL_LIMIT = 8_100_000
const MEI_OPENING_MONTHLY_LIMIT = 675_000
const MEI_DAS_SERVICES = 8_605
const ME_LIMIT = 36_000_000
const EPP_LIMIT = 480_000_000
const PRESUMPTION = 0.32

interface Bracket {
  upTo: number
  nominal: number
  deduction: number
  split: Record<string, number>
}

const ANNEX: Record<'III' | 'V', Bracket[]> = {
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

const sum = (values: number[]) => values.reduce((a, b) => a + b, 0)

// ---------------------------------------------------------------- building blocks

function ownerTaxes(gross: number, dependents: number) {
  const inss = Math.round(Math.min(gross, INSS_CEILING) * INSS_RATE)
  const deductions = Math.max(inss + dependents * IRRF_DEPENDENT, IRRF_SIMPLIFIED)
  const base = Math.max(0, gross - deductions)
  const [, rate, deduction] = IRRF_TABLE.find(([upTo]) => base <= upTo)!
  const tax = Math.max(0, Math.round(base * rate - deduction))
  // Lei 15.270/2025: full reduction up to R$ 5.000, linear phase-out up to R$ 7.350.
  let reduction = 0
  if (gross <= 500_000) reduction = Math.min(tax, 31_289)
  else if (gross <= 735_000) reduction = Math.min(tax, Math.max(0, Math.round(97_862 - 0.133145 * gross)))
  const irrf = tax - reduction
  return { inss, irrf, reduction, net: gross - inss - irrf }
}

/** Splits `total` by shares, fixing the rounding on the largest share. */
function splitBy(total: number, shares: Record<string, number>): TaxShare[] {
  const out = Object.entries(shares).map(([tax, share]) => ({ tax, cents: Math.round(total * share) }))
  const diff = total - sum(out.map((s) => s.cents))
  if (diff && out.length) out.reduce((a, b) => (b.cents > a.cents ? b : a)).cents += diff
  return out
}

function simples(annex: 'III' | 'V', rbt12: number, revenue: number) {
  const table = ANNEX[annex]
  let index = table.findIndex((b) => rbt12 <= b.upTo)
  if (index < 0) index = table.length - 1
  const b = table[index]
  const rbt = Math.max(rbt12, 1)
  const effective = Math.max(0, (rbt * b.nominal - b.deduction) / rbt)
  const das = Math.round(revenue * effective)
  return { bracket: index + 1, nominal: b.nominal, deduction: b.deduction, effective, das, split: splitBy(das, b.split) }
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
  activity: SimplesActivity
  payroll: number
  dependents: number
  issRate: number
  /** Fixed pró-labore, or null for automatic. */
  fixedProLabore: number | null
}

function autoProLabore(average: number, payroll: number): number {
  // Integer math (no float noise), rounded up so the Fator R is not truncated below 28%.
  return Math.max(MIN_WAGE, Math.ceil((average * 28) / 100) - payroll)
}

function scenario(regime: TaxRegime, average: number, rbt12: number, ctx: Context): Scenario {
  if (regime === 'mei') {
    return {
      companyTaxes: [{ tax: 'das', cents: MEI_DAS_SERVICES }],
      companyTotal: MEI_DAS_SERVICES,
      proLabore: 0,
      inss: 0,
      irrf: 0,
      taxFreeLimit: Math.round(average * PRESUMPTION),
      annex: null,
    }
  }
  if (regime === 'presumido') {
    const proLabore = ctx.fixedProLabore ?? MIN_WAGE
    const p = presumido(average, proLabore, ctx.issRate)
    const o = ownerTaxes(proLabore, ctx.dependents)
    const federal = share(p.shares, 'irpj') + share(p.shares, 'csll') + share(p.shares, 'pis') + share(p.shares, 'cofins')
    return { companyTaxes: p.shares, companyTotal: p.total, proLabore, inss: o.inss, irrf: o.irrf, taxFreeLimit: Math.max(0, p.base - federal), annex: null }
  }
  return simplesScenario(average, rbt12, ctx, ctx.fixedProLabore ?? autoProLabore(average, ctx.payroll))
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
  const s = simples(annex, rbt12, average)
  const o = ownerTaxes(proLabore, ctx.dependents)
  return {
    companyTaxes: s.split,
    companyTotal: s.das,
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
    activity: input.activity,
    payroll: input.payrollCents,
    dependents: input.dependents,
    issRate: input.issRate,
    fixedProLabore: input.proLaboreCents,
  }

  // RBT12 of each month: the known months (notes and projections, plus the months considered) among
  // the 12 before it, from the opening; proportional when fewer, the month itself × 12 when none.
  const opening = input.openingMonth
  const known = new Map<string, number>()
  for (const m of input.revenueHistory ?? []) known.set(m.month, (known.get(m.month) ?? 0) + m.cents)
  for (const m of months) known.set(m.month, m.cents)
  const rbt12Of = (month: string, revenue: number) => {
    let windowSum = 0
    let n = 0
    for (let i = 1; i <= 12; i++) {
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
  const forAnnexIii = Math.max(MIN_WAGE, typicalNeed, monthNeed)
  const basis = binding && monthNeed > Math.max(MIN_WAGE, typicalNeed) ? binding : null
  const gross = input.proLaboreCents ?? forAnnexIii
  const owner = ownerTaxes(gross, input.dependents)

  // Simples with the user's pró-labore.
  const fs12 = (gross + input.payrollCents) * 12
  const fatorR = fatorROf(fs12, rbt12)
  const annex = annexFor(input.activity, fatorR)
  const s = simples(annex, rbt12, average)

  // MEI.
  const limit =
    opening && opening.slice(0, 4) === refYear
      ? MEI_OPENING_MONTHLY_LIMIT * (13 - Number(opening.slice(5, 7)))
      : MEI_ANNUAL_LIMIT
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
      : '')
  // Annex III needs the Fator R at 28% (for Fator R activities); Annex V is the cheapest with the
  // minimum wage (or the fixed pró-labore).
  const iiiSc = simplesScenario(average, rbt12, ctx, input.activity === 'fatorR' ? Math.max(gross, forAnnexIii) : gross, 'III')
  const vSc = simplesScenario(average, rbt12, ctx, MIN_WAGE, 'V')
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
      input.activity === 'fatorR'
        ? `Com pró-labore de um salário mínimo (${formatBRLPlain(vSc.proLabore)}), Fator R abaixo de 28%.`
        : 'Esta atividade é sempre tributada no Anexo III.',
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
    ? scenario(input.regime, leftoverRevenue, rbt12, { ...ctx, fixedProLabore: real.proLabore || null })
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

  // Month by month in the current regime: each month's revenue, costs and pró-labore. In the Simples
  // each month has its own RBT12, Fator R and annex (and the pró-labore that keeps it in Anexo III).
  const plInUse = input.regime === 'mei' ? 0 : input.regime === 'presumido' ? real.proLabore : gross
  const plTaxes = input.regime === 'mei' ? { inss: 0, irrf: 0 } : ownerTaxes(plInUse, input.dependents)
  const cashFlow: CashFlowMonth[] = months.map((m) => {
    const revenue = m.cents
    let company: number
    let simplesMonth: Pick<CashFlowMonth, 'rbt12Cents' | 'fatorR' | 'annex' | 'proLaboreForAnnexIiiCents'> = {
      rbt12Cents: null,
      fatorR: null,
      annex: null,
      proLaboreForAnnexIiiCents: null,
    }
    if (input.regime === 'mei') company = MEI_DAS_SERVICES
    else if (input.regime === 'presumido') company = presumido(revenue, plInUse, input.issRate).total
    else {
      const { rbt12: monthRbt12 } = rbt12Of(m.month, revenue)
      const monthFatorR = fatorROf(12 * (plInUse + input.payrollCents), monthRbt12)
      const monthAnnex = annexFor(input.activity, monthFatorR)
      const das = simples(monthAnnex, monthRbt12, revenue)
      // 6th bracket: the ISS is paid to the municipality, outside the DAS.
      company = das.das + (das.bracket === 6 ? Math.round(revenue * input.issRate) : 0)
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
    const netFor = (revenue: number) =>
      leftoverOf(revenue, costs, scenario(input.regime, revenue, revenue * 12, ctx), reserveRate)
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
        (input.regime === 'mei' ? '.' : automatic ? ', com o pró-labore automático.' : ', com o pró-labore informado.') +
        (reserveRate > 0 ? '' : ' Sem reserva para 13º, férias e imprevistos.'),
    }
  }

  const warnings: string[] = []
  if (annualized)
    warnings.push(
      `Com ${count === 1 ? '1 mês' : `${count} meses`}, o RBT12 foi estimado pela média × 12 (regra das empresas novas).`,
    )
  if (input.activity === 'fatorR' && annex === 'V')
    warnings.push(
      `Fator R de ${formatPercent(fatorR)} (abaixo de 28%): Anexo V. Com pró-labore de ${formatBRLPlain(forAnnexIii)} por mês, a empresa vai para o Anexo III.`,
    )
  if (projectedStatus !== 'within')
    warnings.push('No ritmo atual, o faturamento do ano passa do limite do MEI (R$ 81 mil).')
  warnings.push('Não considera retenções na fonte (IRRF, CSRF, ISS retido), 13º, férias nem outras receitas.')
  warnings.push('Tabelas de 2026: salário mínimo de R$ 1.621,00 e teto do INSS de R$ 8.475,55.')

  return {
    referenceMonth: ref,
    revenue: {
      months,
      monthsCount: count,
      totalCents: total,
      averageMonthlyCents: average,
      monthCents: months.find((m) => m.month === ref)?.cents ?? 0,
      yearToDateCents: yearToDate,
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
      dasCents: MEI_DAS_SERVICES,
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
      proLaboreForAnnexIiiCents: forAnnexIii,
      meLimitRemainingCents: Math.max(0, ME_LIMIT - yearToDate),
    },
    proLabore: {
      grossCents: gross,
      automatic,
      basisAverageCents: basis ? Math.round(basis.rbt12 / 12) : average,
      basisMonths: basis ? basis.n : count,
      basisMonth: basis?.month ?? null,
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

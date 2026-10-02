// State of the tax page (Svelte 5 runes).
//
// - The planning of the active company: everything the user types on the page (company settings,
//   projections and the rows being added, fixed and variable costs, reserve, the base of "sobra do
//   mês", the period / included months, open tab). Saved with the backend store
//   (`write_store('planning-<company>')`, debounced) and read when the company is opened, see
//   ./persist.svelte.ts and ./companies.svelte.ts.
// - The revenue per competence month computed from the table (revenue rows only), joined with the
//   projections: the months of "Receitas e projeção". Each month uses its notes or its projection
//   ("Previsto"), and the included months with a value are the report input.
// - The costs: fixed items (monthly or yearly) and variable costs per month. Variable costs are the
//   expense rows of the table (by competence) plus the ones typed by the user. A fixed item can be
//   linked to a type of note: expense rows of that type confirm the item (planned × real) instead
//   of adding up as variable costs.
// - The revenue of every month of "Receitas e projeção" (`revenueHistory`): the RBT12 of each month
//   of the cash flow (and the automatic pró-labore) looks at the 12 months before it.
// - The CNAE catalog and the report computed by the backend (`tax_report`, debounced).
//
// Revenue of the notes: only revenue rows that count towards the total (no duplicates, no cancelled
// notes, not removed). Amount = the value typed by the user, else the gross value ("Valor do
// serviço"), else the net value. Expense rows use their value (net or typed). Month = the competence
// typed by the user, else the one read from the note. Rows without a month are left out and
// reported, so the user can fill them in.

import * as api from './api'
import { errorMessage } from './api'
import { addMonths, formatMonth, isMonthKey, monthKeyOf, normalizeText } from './format'
import { StoreDoc } from './persist.svelte'
import { inRange, parsePeriod, periodRange, type Period, type PeriodPreset } from './period'
import { isRecord, loadJSON, removeKey } from './storage'
import { isBonusType, store, type Row } from './store.svelte'
import type {
  CnaeInfo,
  MonthAmount,
  SimplesActivity,
  TaxCatalog,
  TaxInput,
  TaxRegime,
  TaxReport,
} from './types'

export type { SaveState } from './persist.svelte'

export type TaxTab = 'summary' | 'revenue' | 'costs' | 'prolabore' | 'cashflow'
/** The months considered: a period preset, "Personalizado" (range) or "Seleção manual" (custom). */
export type MonthPreset = PeriodPreset
export type ProLaboreMode = 'auto' | 'fixed'
export type MonthSource = 'notes' | 'forecast'
export type CostFrequency = 'monthly' | 'yearly'
export type LeftoverBase = 'average' | 'noBonus' | 'fixed'

/** Company settings (the sidebar of the tax page). */
export interface TaxSettings {
  regime: TaxRegime
  /** Chosen CNAE code; null = the catalog default. */
  cnae: string | null
  /** Annex rule used only when the CNAE catalog could not be loaded. */
  activity: SimplesActivity
  proLaboreMode: ProLaboreMode
  /** Fixed monthly pró-labore; null until the user switches to "Valor fixo" once. */
  proLaboreCents: number | null
  payrollCents: number
  dependents: number
  issRate: number
  openingMonth: string | null
  desiredNetCents: number | null
}

/** Revenue expected for a month ("Previsto"). */
export interface Projection {
  month: string
  /** null = not filled in yet (the month then has no value). */
  cents: number | null
  /** Short note, e.g. "bônus", "contrato X". */
  note: string
}

/**
 * A row added with "Adicionar linha" whose month is not usable yet (empty, invalid or already in
 * the list). Ignored by the report; it becomes a projection as soon as its month is valid and new.
 */
export interface DraftRow {
  id: string
  month: string | null
  cents: number | null
  note: string
}

/** A fixed cost (the same amount every month, or every year). */
export interface CostItem {
  id: string
  name: string
  /** Amount per period (month or year); null = not filled in. */
  cents: number | null
  frequency: CostFrequency
  /** Type of note whose expense rows are the real amount of this cost ("Ligar a um tipo de nota"). */
  linkedType: string | null
  /** First month the cost applies (`yyyy-mm`); null = always. */
  start: string | null
  /** Last month (`yyyy-mm`); null = no end. */
  end: string | null
}

/** A variable cost typed by the user (valid when it has a month and a value). */
export interface VariableCost {
  id: string
  month: string | null
  description: string
  cents: number | null
}

/** Everything the user types on the tax page (the "planning" store). */
export interface Planning {
  version: 1
  settings: TaxSettings
  preset: MonthPreset
  /** Months of "Personalizado" (preset "range"); null = open. */
  rangeFrom: string | null
  rangeTo: string | null
  /** Months included one by one (preset "custom", "Seleção manual"). */
  included: string[]
  projections: Projection[]
  /** Rows being added in "Receitas e projeção" (not valid yet). */
  drafts: DraftRow[]
  /** Source chosen by the user per month; months not listed use the default. */
  sources: Record<string, MonthSource>
  costs: CostItem[]
  variableCosts: VariableCost[]
  /** Share of the revenue set aside every month (0..0.9). */
  reserveRate: number
  leftoverBase: LeftoverBase
  leftoverFixedCents: number | null
  tab: TaxTab
}

/** Revenue of the notes of one month. */
export interface MonthRevenue {
  month: string
  cents: number
  notes: number
  /** Part of it from notes typed "Bônus". */
  bonusCents: number
  bonusNotes: number
}

/** One month of "Receitas e projeção". */
export interface PlanMonth {
  month: string
  /** Revenue of the notes (null = no notes in this month). */
  notesCents: number | null
  notesCount: number
  bonusCents: number
  bonusCount: number
  projection: Projection | null
  source: MonthSource
  /** The chosen source was picked by the user (not the default). */
  sourceChosen: boolean
  /** Value of the chosen source; null when it has none (no notes, or "Previsto" empty). */
  cents: number | null
  included: boolean
  /** After the current month. */
  future: boolean
}

/** Variable costs of one month: the expense rows (not linked to a fixed cost) and the typed ones. */
export interface VariableMonth {
  month: string
  notesCents: number
  notesCount: number
  manualCents: number
  manualCount: number
  cents: number
}

/** Real amount of a linked fixed cost in one month. */
export interface LinkedMonth {
  month: string
  cents: number
  count: number
}

export const COST_SUGGESTIONS: { name: string; frequency: CostFrequency }[] = [
  { name: 'Contador', frequency: 'monthly' },
  { name: 'Internet', frequency: 'monthly' },
  { name: 'Endereço fiscal/virtual', frequency: 'monthly' },
  { name: 'Certificado digital', frequency: 'yearly' },
  { name: 'Tarifas bancárias', frequency: 'monthly' },
  { name: 'Softwares e assinaturas', frequency: 'monthly' },
  { name: 'Plano de saúde', frequency: 'monthly' },
  { name: 'Coworking', frequency: 'monthly' },
]

const STORE_NAME = 'planning'
/** 2026 minimum wage, only used to prefill "Valor fixo" before any report arrived. */
const MINIMUM_WAGE_CENTS = 162100
const REPORT_DEBOUNCE_MS = 300
export const NOTE_MAX = 40
export const COST_NAME_MAX = 60
export const RESERVE_MAX = 0.9

const REGIMES: TaxRegime[] = ['mei', 'simples', 'presumido']
const TABS: TaxTab[] = ['summary', 'revenue', 'costs', 'prolabore', 'cashflow']
const BASES: LeftoverBase[] = ['average', 'noBonus', 'fixed']

// ---------------------------------------------------------------- defaults and validation

const DEFAULT_SETTINGS: TaxSettings = {
  regime: 'simples',
  cnae: null,
  activity: 'fatorR',
  proLaboreMode: 'auto',
  proLaboreCents: null,
  payrollCents: 0,
  dependents: 0,
  issRate: 0.05,
  openingMonth: null,
  desiredNetCents: null,
}

function defaultPlanning(): Planning {
  return {
    version: 1,
    settings: { ...DEFAULT_SETTINGS },
    preset: 'all',
    rangeFrom: null,
    rangeTo: null,
    included: [],
    projections: [],
    drafts: [],
    sources: {},
    costs: [],
    variableCosts: [],
    reserveRate: 0,
    leftoverBase: 'average',
    leftoverFixedCents: null,
    tab: 'summary',
  }
}

const isCents = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0
const centsOrNull = (v: unknown): number | null => (isCents(v) ? v : null)
const clip = (v: unknown, max: number): string => (typeof v === 'string' ? v.slice(0, max) : '')
const byMonth = <T extends { month: string }>(list: T[]): T[] =>
  [...list].sort((a, b) => (a.month < b.month ? -1 : a.month > b.month ? 1 : 0))
const sameType = (a: string | null | undefined, b: string | null | undefined) =>
  !!a && !!b && normalizeText(a) === normalizeText(b)

let idSeq = 0
function newId(): string {
  return `c${Date.now().toString(36)}${(idSeq++).toString(36)}${Math.random().toString(36).slice(2, 6)}`
}

/** Ids unique within a list (a missing or repeated one gets a new id). */
function uniqueId(raw: unknown, seen: Set<string>): string {
  let id = typeof raw === 'string' && raw ? raw.slice(0, 40) : newId()
  if (seen.has(id)) id = newId()
  seen.add(id)
  return id
}

function parseSettings(raw: unknown): TaxSettings {
  const r = isRecord(raw) ? raw : {}
  const d = DEFAULT_SETTINGS
  return {
    regime: REGIMES.includes(r.regime as TaxRegime) ? (r.regime as TaxRegime) : d.regime,
    cnae: typeof r.cnae === 'string' && r.cnae ? r.cnae.slice(0, 20) : d.cnae,
    activity: r.activity === 'annexIii' ? 'annexIii' : 'fatorR',
    proLaboreMode: r.proLaboreMode === 'fixed' ? 'fixed' : 'auto',
    proLaboreCents: centsOrNull(r.proLaboreCents),
    payrollCents: isCents(r.payrollCents) ? r.payrollCents : d.payrollCents,
    dependents:
      typeof r.dependents === 'number' && Number.isInteger(r.dependents) && r.dependents >= 0 && r.dependents <= 20
        ? r.dependents
        : d.dependents,
    issRate: typeof r.issRate === 'number' && r.issRate >= 0 && r.issRate <= 0.2 ? r.issRate : d.issRate,
    openingMonth: isMonthKey(r.openingMonth) ? r.openingMonth : d.openingMonth,
    desiredNetCents: centsOrNull(r.desiredNetCents),
  }
}

function parseProjections(raw: unknown): Projection[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  const out: Projection[] = []
  for (const p of raw) {
    if (!isRecord(p) || !isMonthKey(p.month) || seen.has(p.month)) continue
    seen.add(p.month)
    out.push({ month: p.month, cents: centsOrNull(p.cents), note: clip(p.note, NOTE_MAX) })
  }
  return byMonth(out)
}

function parseDrafts(raw: unknown): DraftRow[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  return raw.filter(isRecord).map((d) => ({
    id: uniqueId(d.id, seen),
    month: isMonthKey(d.month) ? d.month : null,
    cents: centsOrNull(d.cents),
    note: clip(d.note, NOTE_MAX),
  }))
}

function parseCosts(raw: unknown): CostItem[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  return raw.filter(isRecord).map((c) => ({
    id: uniqueId(c.id, seen),
    name: clip(c.name, COST_NAME_MAX),
    cents: centsOrNull(c.cents),
    frequency: c.frequency === 'yearly' ? 'yearly' : 'monthly',
    linkedType: typeof c.linkedType === 'string' && c.linkedType.trim() ? c.linkedType.slice(0, 40) : null,
    start: isMonthKey(c.start) ? c.start : null,
    end: isMonthKey(c.end) ? c.end : null,
  }))
}

function parseVariableCosts(raw: unknown): VariableCost[] {
  if (!Array.isArray(raw)) return []
  const seen = new Set<string>()
  return raw.filter(isRecord).map((v) => ({
    id: uniqueId(v.id, seen),
    month: isMonthKey(v.month) ? v.month : null,
    description: clip(v.description, COST_NAME_MAX),
    cents: centsOrNull(v.cents),
  }))
}

function parseSources(raw: unknown): Record<string, MonthSource> {
  const out: Record<string, MonthSource> = {}
  if (!isRecord(raw)) return out
  for (const [month, source] of Object.entries(raw)) {
    if (isMonthKey(month) && (source === 'notes' || source === 'forecast')) out[month] = source
  }
  return out
}

/** The stored month selection: a period preset (the first versions said "year"). */
function parsePreset(raw: Record<string, unknown>): Period {
  const preset = raw.preset === 'year' ? 'thisYear' : raw.preset
  return parsePeriod({ preset, from: raw.rangeFrom, to: raw.rangeTo })
}

/** Validates a stored planning, keeping every well-typed field (null when it is not a planning). */
export function parsePlanning(raw: unknown): Planning | null {
  if (!isRecord(raw) || typeof raw.version !== 'number') return null
  const d = defaultPlanning()
  const period = parsePreset(raw)
  return {
    version: 1,
    settings: parseSettings(raw.settings),
    preset: period.preset,
    rangeFrom: period.from,
    rangeTo: period.to,
    included: Array.isArray(raw.included) ? [...new Set(raw.included.filter(isMonthKey))] : [],
    projections: parseProjections(raw.projections),
    drafts: parseDrafts(raw.drafts),
    sources: parseSources(raw.sources),
    costs: parseCosts(raw.costs),
    variableCosts: parseVariableCosts(raw.variableCosts),
    reserveRate:
      typeof raw.reserveRate === 'number' && raw.reserveRate >= 0 && raw.reserveRate <= RESERVE_MAX
        ? raw.reserveRate
        : d.reserveRate,
    leftoverBase: BASES.includes(raw.leftoverBase as LeftoverBase) ? (raw.leftoverBase as LeftoverBase) : d.leftoverBase,
    leftoverFixedCents: centsOrNull(raw.leftoverFixedCents),
    tab: TABS.includes(raw.tab as TaxTab) ? (raw.tab as TaxTab) : d.tab,
  }
}

/** The settings of the first version, kept in localStorage ("nfsetable.tax"). */
function legacyPlanning(): Planning | null {
  const raw = loadJSON<Record<string, unknown> | null>('tax', null, (v): v is Record<string, unknown> => isRecord(v))
  if (!raw) return null
  const plan = defaultPlanning()
  plan.settings = parseSettings(raw)
  plan.preset = parsePreset(raw).preset
  if (Array.isArray(raw.months)) plan.included = raw.months.filter(isMonthKey)
  if (isCents(raw.monthlyCostsCents) && raw.monthlyCostsCents > 0)
    plan.costs = [
      { id: newId(), name: 'Custos mensais', cents: raw.monthlyCostsCents, frequency: 'monthly', linkedType: null, start: null, end: null },
    ]
  return plan
}

/** Amount of a revenue row for the taxes: typed value, else gross value, else net value. */
export function revenueCents(row: Row): number | null {
  return row.valueEdited ? row.cents : (row.serviceCents ?? row.cents)
}

/** Monthly amount of a cost item (yearly ones divided by 12). */
export function monthlyCost(item: CostItem): number {
  if (item.cents == null) return 0
  return item.frequency === 'yearly' ? Math.round(item.cents / 12) : item.cents
}

/** The item has a start or an end month (sent month by month). */
export function isDated(item: CostItem): boolean {
  return item.start != null || item.end != null
}

/** "Fim" before "Início": the item is never active until it is fixed. */
export function datesInvalid(item: CostItem): boolean {
  return item.start != null && item.end != null && item.end < item.start
}

/** The item applies in this month. */
export function activeIn(item: CostItem, month: string): boolean {
  if (datesInvalid(item)) return false
  return (!item.start || month >= item.start) && (!item.end || month <= item.end)
}

/** "sempre", "desde 03/2026", "até 12/2026", "03/2026 – 12/2026". */
export function costValidity(item: CostItem): string {
  if (item.start && item.end) return item.start === item.end ? `só ${formatMonth(item.start)}` : `${formatMonth(item.start)} – ${formatMonth(item.end)}`
  if (item.start) return `desde ${formatMonth(item.start)}`
  if (item.end) return `até ${formatMonth(item.end)}`
  return 'sempre'
}

// ---------------------------------------------------------------- store

export class TaxStore {
  plan = $state<Planning>(defaultPlanning())
  catalog = $state.raw<TaxCatalog | null>(null)
  catalogError = $state<string | null>(null)
  report = $state.raw<TaxReport | null>(null)
  /** A report request is scheduled or running. */
  loading = $state(false)
  error = $state<string | null>(null)
  /** Saving of the planning (debounced; paused when the saved document cannot be read). */
  doc = new StoreDoc<Planning>({
    name: STORE_NAME,
    readWhat: 'o planejamento salvo',
    saveWhat: 'o planejamento',
    snapshot: () => $state.snapshot(this.plan) as Planning,
    notify: (message, kind, action) => store.toast(message, kind, action),
  })

  #timer: ReturnType<typeof setTimeout> | undefined
  #seq = 0
  #lastKey: string | null = null
  #catalogLoading = false

  /** Current month and year (the presets "Ano atual", "Últimos 12 meses", "Próximos meses"). */
  readonly currentMonth = monthKeyOf(new Date())
  readonly currentYear = this.currentMonth.slice(0, 4)

  get settings(): TaxSettings {
    return this.plan.settings
  }

  /** The months considered, as a period ("Seleção manual" when ticked one by one). */
  get period(): Period {
    return { preset: this.plan.preset, from: this.plan.rangeFrom, to: this.plan.rangeTo }
  }

  get saveState() {
    return this.doc.saveState
  }

  get saveError() {
    return this.doc.saveError
  }

  get readBlocked() {
    return this.doc.readBlocked
  }

  // ------------------------------------------------------------ revenue and months

  /** Revenue of the notes (revenue rows) per competence month and the counted rows without a month. */
  revenue = $derived.by(() => {
    const map = new Map<string, MonthRevenue>()
    const missing: Row[] = []
    for (const row of store.rows) {
      if (row.kind === 'expense' || !row.counted || row.duplicateOf) continue
      const cents = revenueCents(row)
      if (cents == null) continue
      if (!row.competence) {
        missing.push(row)
        continue
      }
      const entry = map.get(row.competence) ?? { month: row.competence, cents: 0, notes: 0, bonusCents: 0, bonusNotes: 0 }
      entry.cents += cents
      entry.notes++
      if (isBonusType(row.docType)) {
        entry.bonusCents += cents
        entry.bonusNotes++
      }
      map.set(row.competence, entry)
    }
    return { byMonth: map, months: byMonth([...map.values()]), missing }
  })

  /** Average of the months that have notes ("preencher com a média"); null without notes. */
  notesAverage = $derived.by(() => {
    const months = this.revenue.months
    if (!months.length) return null
    return Math.round(months.reduce((sum, m) => sum + m.cents, 0) / months.length)
  })

  /** Months with notes or a projection, oldest first. */
  monthKeys = $derived.by(() => {
    const keys = new Set(this.revenue.byMonth.keys())
    for (const p of this.plan.projections) keys.add(p.month)
    return [...keys].sort()
  })

  monthKeySet = $derived(new Set(this.monthKeys))

  /** Months included by the preset (or picked one by one). */
  includedSet = $derived(new Set(this.presetMonths(this.plan.preset)))

  planMonths = $derived.by((): PlanMonth[] => {
    const notes = this.revenue.byMonth
    const projections = new Map(this.plan.projections.map((p) => [p.month, p]))
    return this.monthKeys.map((month) => {
      const n = notes.get(month)
      const projection = projections.get(month) ?? null
      const chosen = this.plan.sources[month]
      const source: MonthSource = chosen ?? (n ? 'notes' : 'forecast')
      const cents = source === 'notes' ? (n ? n.cents : null) : (projection?.cents ?? null)
      return {
        month,
        notesCents: n ? n.cents : null,
        notesCount: n?.notes ?? 0,
        bonusCents: n?.bonusCents ?? 0,
        bonusCount: n?.bonusNotes ?? 0,
        projection,
        source,
        sourceChosen: chosen != null,
        cents,
        included: this.includedSet.has(month),
        future: month > this.currentMonth,
      }
    })
  })

  /** Included months that have a value: the report input. */
  consideredMonths = $derived(this.planMonths.filter((m) => m.included && m.cents != null))
  /** The reference month: the last month considered (its tables, and its costs in the typical month). */
  referenceMonth = $derived.by((): string | null => {
    const months = this.consideredMonths
    return months.length ? months[months.length - 1].month : null
  })
  /** Included months still without a value (a projection not filled in, or no notes). */
  emptyMonths = $derived(this.planMonths.filter((m) => m.included && m.cents == null))
  /**
   * Every month of "Receitas e projeção" with a value (each with its chosen source), included or
   * not: the RBT12 of each month of the cash flow looks at the 12 months before it.
   */
  revenueHistory = $derived<MonthAmount[]>(
    this.planMonths.filter((m) => m.cents != null).map((m) => ({ month: m.month, cents: m.cents ?? 0 })),
  )
  /** Months considered that use the forecast ("previsto"). */
  forecastSet = $derived(new Set(this.consideredMonths.filter((m) => m.source === 'forecast').map((m) => m.month)))

  /**
   * How many of these months use the forecast. Texts next to report numbers count over the report
   * months, so they stay consistent while a newer report is on its way.
   */
  forecastAmong(months: readonly { month: string }[]): number {
    const set = this.forecastSet
    return months.reduce((n, m) => n + (set.has(m.month) ? 1 : 0), 0)
  }

  /** Average of the months considered ("média dos meses"). */
  averageCents = $derived.by(() => {
    const months = this.consideredMonths
    return months.length ? Math.round(months.reduce((sum, m) => sum + (m.cents ?? 0), 0) / months.length) : 0
  })

  /** "Média sem bônus": the months considered without the notes typed "Bônus". */
  noBonus = $derived.by(() => {
    const months = this.consideredMonths
    let total = 0
    let excludedCents = 0
    let excludedNotes = 0
    for (const m of months) {
      const bonus = m.source === 'notes' ? m.bonusCents : 0
      total += (m.cents ?? 0) - bonus
      excludedCents += bonus
      if (m.source === 'notes') excludedNotes += m.bonusCount
    }
    return {
      available: excludedNotes > 0,
      averageCents: months.length ? Math.round(total / months.length) : 0,
      excludedCents,
      excludedNotes,
    }
  })

  // ------------------------------------------------------------ costs

  /** Fixed costs of every month: the items without dates (yearly ones divided by 12). */
  costsMonthlyCents = $derived(this.plan.costs.reduce((sum, c) => sum + (isDated(c) ? 0 : monthlyCost(c)), 0))

  /** Dated fixed items, month by month over the months considered (zero months left out). */
  fixedCostsByMonth = $derived.by((): MonthAmount[] => {
    const dated = this.plan.costs.filter(isDated)
    if (!dated.length) return []
    const out: MonthAmount[] = []
    for (const m of this.consideredMonths) {
      const cents = dated.reduce((sum, c) => sum + (activeIn(c, m.month) ? monthlyCost(c) : 0), 0)
      if (cents > 0) out.push({ month: m.month, cents })
    }
    return out
  })

  /**
   * Fixed costs of the typical month: the undated items plus the dated ones in effect in the
   * reference month, in full (one that ended counts nothing; never an average).
   */
  fixedReferenceCents = $derived.by(() => {
    const ref = this.referenceMonth
    const dated = ref ? (this.fixedCostsByMonth.find((m) => m.month === ref)?.cents ?? 0) : 0
    return this.costsMonthlyCents + dated
  })

  /**
   * Expense rows of the table by competence month: the variable ones, the real amount of each
   * linked fixed cost, and the rows without a month (left out, reported).
   */
  expenses = $derived.by(() => {
    const linkedIds = new Map<string, string>()
    for (const c of this.plan.costs) {
      const key = c.linkedType ? normalizeText(c.linkedType) : ''
      if (key && !linkedIds.has(key)) linkedIds.set(key, c.id)
    }
    const variable = new Map<string, { cents: number; count: number }>()
    const linked = new Map<string, Map<string, LinkedMonth>>()
    const missing: Row[] = []
    let count = 0
    for (const row of store.rows) {
      if (row.kind !== 'expense' || !row.counted || row.duplicateOf || row.cents == null) continue
      count++
      if (!row.competence) {
        missing.push(row)
        continue
      }
      const id = linkedIds.get(normalizeText(row.docType))
      if (id) {
        const months = linked.get(id) ?? new Map<string, LinkedMonth>()
        const m = months.get(row.competence) ?? { month: row.competence, cents: 0, count: 0 }
        m.cents += row.cents
        m.count++
        months.set(row.competence, m)
        linked.set(id, months)
      } else {
        const m = variable.get(row.competence) ?? { cents: 0, count: 0 }
        m.cents += row.cents
        m.count++
        variable.set(row.competence, m)
      }
    }
    return { variable, linked, missing, count }
  })

  /** Real amounts of a linked fixed cost, oldest month first. */
  linkedMonths(id: string): LinkedMonth[] {
    return byMonth([...(this.expenses.linked.get(id)?.values() ?? [])])
  }

  /** Types of the expense rows (and of the rules that mark expenses), for "Ligar a um tipo de nota". */
  expenseTypes = $derived.by(() => {
    const types = new Map<string, string>()
    const add = (type: string | null | undefined) => {
      const key = type ? normalizeText(type) : ''
      if (key && !types.has(key)) types.set(key, type!)
    }
    for (const row of store.rows) if (row.kind === 'expense') add(row.docType)
    for (const p of store.profiles) if (p.kind === 'expense') add(p.docType)
    for (const c of this.plan.costs) add(c.linkedType)
    return [...types.values()].sort((a, b) => a.localeCompare(b, 'pt-BR'))
  })

  /** Variable costs per month (expense rows not linked + typed), oldest first. */
  variableByMonth = $derived.by((): VariableMonth[] => {
    const map = new Map<string, VariableMonth>()
    const get = (month: string): VariableMonth => {
      let m = map.get(month)
      if (!m) {
        m = { month, notesCents: 0, notesCount: 0, manualCents: 0, manualCount: 0, cents: 0 }
        map.set(month, m)
      }
      return m
    }
    for (const [month, e] of this.expenses.variable) {
      const m = get(month)
      m.notesCents += e.cents
      m.notesCount += e.count
    }
    for (const v of this.plan.variableCosts) {
      if (!v.month || v.cents == null) continue
      const m = get(v.month)
      m.manualCents += v.cents
      m.manualCount++
    }
    for (const m of map.values()) m.cents = m.notesCents + m.manualCents
    return byMonth([...map.values()])
  })

  /** Report input: the variable costs of every month (the typical month uses the reference month's). */
  costsByMonth = $derived<MonthAmount[]>(
    this.variableByMonth.filter((m) => m.cents > 0).map((m) => ({ month: m.month, cents: m.cents })),
  )

  /**
   * Variable costs of the reference month: what the typical month uses (each expense counts in the
   * month of its note; the cash flow shows every month's).
   */
  variableReferenceCents = $derived.by(() => {
    const ref = this.referenceMonth
    return ref ? (this.variableByMonth.find((m) => m.month === ref)?.cents ?? 0) : 0
  })

  // ------------------------------------------------------------ CNAE, report input

  /** The chosen CNAE (or the catalog default), when the catalog is loaded. */
  cnaeInfo = $derived.by((): CnaeInfo | null => {
    const catalog = this.catalog
    if (!catalog?.cnaes.length) return null
    const code = this.plan.settings.cnae ?? catalog.defaultCnae
    return (
      catalog.cnaes.find((c) => c.code === code) ??
      catalog.cnaes.find((c) => c.code === catalog.defaultCnae) ??
      catalog.cnaes[0]
    )
  })

  /** Annex rule sent to the backend: from the CNAE, or the manual choice without a catalog. */
  activity = $derived<SimplesActivity>(this.cnaeInfo?.activity ?? this.plan.settings.activity)

  /** Revenue of "sobra do mês": null = the average. */
  leftoverRevenueCents = $derived.by((): number | null => {
    const p = this.plan
    if (p.leftoverBase === 'noBonus') return this.noBonus.available ? this.noBonus.averageCents : null
    if (p.leftoverBase === 'fixed') return p.leftoverFixedCents
    return null
  })

  /** Input of the report; null when no included month has a value. */
  input = $derived.by((): TaxInput | null => {
    const months = this.consideredMonths
    if (!months.length) return null
    const s = this.plan.settings
    return {
      referenceMonth: months[months.length - 1].month,
      revenueByMonth: months.map((m) => ({ month: m.month, cents: m.cents ?? 0 })),
      regime: s.regime,
      activity: this.activity,
      openingMonth: s.openingMonth,
      proLaboreCents: s.proLaboreMode === 'fixed' ? (s.proLaboreCents ?? MINIMUM_WAGE_CENTS) : null,
      revenueHistory: this.revenueHistory,
      payrollCents: s.payrollCents,
      dependents: s.dependents,
      monthlyCostsCents: this.costsMonthlyCents,
      fixedCostsByMonth: this.fixedCostsByMonth,
      costsByMonth: this.costsByMonth,
      leftoverRevenueCents: this.leftoverRevenueCents,
      reserveRate: this.plan.reserveRate,
      issRate: s.issRate,
      desiredNetCents: s.desiredNetCents,
    }
  })

  /** Months a preset includes, among the months listed (notes months and projections alike). */
  presetMonths(preset: MonthPreset): string[] {
    const keys = this.monthKeys
    if (preset === 'custom') {
      const chosen = new Set(this.plan.included)
      return keys.filter((m) => chosen.has(m))
    }
    const range = periodRange({ preset, from: this.plan.rangeFrom, to: this.plan.rangeTo }, this.currentMonth)
    return range ? keys.filter((m) => inRange(m, range)) : keys
  }

  // ------------------------------------------------------------ edits (every one is saved)

  updateSettings(patch: Partial<TaxSettings>) {
    this.plan.settings = { ...this.plan.settings, ...patch }
    this.#changed()
  }

  /** "Valor fixo" starts from the last value typed, else from the current automatic amount. */
  setProLaboreMode(mode: ProLaboreMode) {
    if (mode === 'fixed' && this.plan.settings.proLaboreCents == null) {
      const auto = this.report?.proLabore.automatic ? this.report.proLabore.grossCents : null
      this.updateSettings({ proLaboreMode: mode, proLaboreCents: auto ?? MINIMUM_WAGE_CENTS })
    } else {
      this.updateSettings({ proLaboreMode: mode })
    }
  }

  setPreset(preset: Exclude<MonthPreset, 'custom' | 'range'>) {
    this.plan.preset = preset
    this.#changed()
  }

  /**
   * The period of the page (Resumo, Fluxo mês a mês): a preset, "Personalizado" (range) or "Seleção
   * manual", which keeps the months included right now so they can be ticked one by one.
   */
  setPeriod(period: Period) {
    if (period.preset === 'custom') {
      this.plan.included = this.presetMonths(this.plan.preset)
      this.plan.preset = 'custom'
    } else {
      const p = parsePeriod(period)
      this.plan.preset = p.preset
      this.plan.rangeFrom = p.from
      this.plan.rangeTo = p.to
    }
    this.#changed()
  }

  /** Includes or leaves out one month; the selection becomes "custom". */
  toggleMonth(month: string) {
    const chosen = new Set(this.includedSet)
    if (chosen.has(month)) chosen.delete(month)
    else chosen.add(month)
    this.plan.included = this.monthKeys.filter((m) => chosen.has(m))
    this.plan.preset = 'custom'
    this.#changed()
  }

  /** "Usar": the notes or the projection of a month. */
  setSource(month: string, source: MonthSource) {
    const fallback: MonthSource = this.revenue.byMonth.has(month) ? 'notes' : 'forecast'
    const next = { ...this.plan.sources }
    if (source === fallback) delete next[month]
    else next[month] = source
    this.plan.sources = next
    this.#changed()
  }

  /** Sets the "Previsto" of a month (creating the projection when needed). */
  setProjection(month: string, patch: { cents?: number | null; note?: string }) {
    if (!isMonthKey(month)) return
    const list = this.plan.projections
    const current = list.find((p) => p.month === month)
    const next: Projection = {
      month,
      cents: patch.cents !== undefined ? patch.cents : (current?.cents ?? null),
      note: clip(patch.note !== undefined ? patch.note : (current?.note ?? ''), NOTE_MAX),
    }
    this.plan.projections = current
      ? list.map((p) => (p.month === month ? next : p))
      : byMonth([...list, next])
    this.#changed()
  }

  /**
   * Adds projections for new months; they are included right away (the period stays when it
   * already covers them, else the selection becomes manual). Returns the months added.
   */
  #insertProjections(list: Projection[]): string[] {
    const existing = this.monthKeySet
    const seen = new Set<string>()
    const added = list.filter((p) => isMonthKey(p.month) && !existing.has(p.month) && !seen.has(p.month) && seen.add(p.month))
    if (!added.length) return []
    const included = this.presetMonths(this.plan.preset)
    this.plan.projections = byMonth([...this.plan.projections, ...added])
    const covered = new Set(this.presetMonths(this.plan.preset))
    if (this.plan.preset === 'custom' || !added.every((p) => covered.has(p.month))) {
      this.plan.included = [...new Set([...included, ...added.map((p) => p.month)])].sort()
      this.plan.preset = 'custom'
    }
    return added.map((p) => p.month)
  }

  /** Adds months with an empty "Previsto"; they are included right away. Returns the new ones. */
  addMonths(months: string[]): string[] {
    const added = this.#insertProjections(months.map((month) => ({ month, cents: null, note: '' })))
    if (added.length) this.#changed()
    return added
  }

  /**
   * "Próximos N meses": the N months from the current one (or from the month after the latest
   * notes, if later); months already listed are kept as they are.
   */
  addNextMonths(count: number): string[] {
    const notes = this.revenue.months
    const latestNotes = notes.length ? notes[notes.length - 1].month : null
    const start = latestNotes && latestNotes >= this.currentMonth ? addMonths(latestNotes, 1) : this.currentMonth
    return this.addMonths(Array.from({ length: count }, (_, i) => addMonths(start, i)))
  }

  /** "Adicionar linha": an empty row at the end of the list. Returns its id. */
  addDraft(): string {
    const id = newId()
    this.plan.drafts = [...this.plan.drafts, { id, month: null, cents: null, note: '' }]
    this.#changed()
    return id
  }

  /**
   * Edits a row being added. As soon as its month is valid and not in the list yet, it becomes a
   * projection (with its value and note) and the month is returned; otherwise null.
   */
  updateDraft(id: string, patch: { month?: string | null; cents?: number | null; note?: string }): string | null {
    const draft = this.plan.drafts.find((d) => d.id === id)
    if (!draft) return null
    const next: DraftRow = {
      ...draft,
      ...patch,
      note: clip(patch.note !== undefined ? patch.note : draft.note, NOTE_MAX),
    }
    if (next.month && !this.monthKeySet.has(next.month)) {
      this.plan.drafts = this.plan.drafts.filter((d) => d.id !== id)
      this.#insertProjections([{ month: next.month, cents: next.cents, note: next.note }])
      this.#changed()
      return next.month
    }
    this.plan.drafts = this.plan.drafts.map((d) => (d.id === id ? next : d))
    this.#changed()
    return null
  }

  /** Removes a row being added (undo in a toast when it had something typed). */
  removeDraft(id: string) {
    const index = this.plan.drafts.findIndex((d) => d.id === id)
    if (index < 0) return
    const draft = this.plan.drafts[index]
    this.plan.drafts = this.plan.drafts.filter((d) => d.id !== id)
    this.#changed()
    if (draft.month == null && draft.cents == null && !draft.note) return
    store.toast('Linha removida.', 'info', {
      label: 'Desfazer',
      run: () => {
        const list = [...this.plan.drafts]
        list.splice(Math.min(index, list.length), 0, draft)
        this.plan.drafts = list
        this.#changed()
      },
    })
  }

  /** Rows being added whose month became free (the other row was removed) turn into projections. */
  #promoteDrafts() {
    for (const d of this.plan.drafts) {
      if (d.month && !this.monthKeySet.has(d.month)) this.updateDraft(d.id, {})
    }
  }

  /** Fills the empty "Previsto" of the months without notes with the average of the notes. */
  fillEmptyWithAverage(): number {
    const average = this.notesAverage
    if (average == null) return 0
    let filled = 0
    this.plan.projections = this.plan.projections.map((p) => {
      if (p.cents != null || this.revenue.byMonth.has(p.month)) return p
      filled++
      return { ...p, cents: average }
    })
    if (filled) this.#changed()
    return filled
  }

  /** Removes the projection of a month (a month without notes leaves the list); undo in a toast. */
  removeProjection(month: string) {
    const projection = this.plan.projections.find((p) => p.month === month)
    if (!projection) return
    const before = {
      source: this.plan.sources[month],
      included: this.plan.included,
      preset: this.plan.preset,
    }
    this.plan.projections = this.plan.projections.filter((p) => p.month !== month)
    if (this.plan.sources[month] === 'forecast') {
      const next = { ...this.plan.sources }
      delete next[month]
      this.plan.sources = next
    }
    if (!this.revenue.byMonth.has(month)) this.plan.included = this.plan.included.filter((m) => m !== month)
    this.#changed()
    this.#promoteDrafts()
    store.toast('Previsto removido.', 'info', {
      label: 'Desfazer',
      run: () => {
        if (this.monthKeySet.has(month) && !this.revenue.byMonth.has(month)) return
        this.plan.projections = byMonth([...this.plan.projections.filter((p) => p.month !== month), projection])
        if (before.source) this.plan.sources = { ...this.plan.sources, [month]: before.source }
        this.plan.included = before.included
        this.plan.preset = before.preset
        this.#changed()
      },
    })
  }

  addCost(item: { name?: string; cents?: number | null; frequency?: CostFrequency; linkedType?: string | null } = {}): string {
    const id = newId()
    this.plan.costs = [
      ...this.plan.costs,
      {
        id,
        name: clip(item.name ?? '', COST_NAME_MAX),
        cents: item.cents ?? null,
        frequency: item.frequency ?? 'monthly',
        linkedType: item.linkedType ?? null,
        start: null,
        end: null,
      },
    ]
    this.#changed()
    return id
  }

  updateCost(
    id: string,
    patch: {
      name?: string
      cents?: number | null
      frequency?: CostFrequency
      linkedType?: string | null
      start?: string | null
      end?: string | null
    },
  ) {
    this.plan.costs = this.plan.costs.map((c) =>
      c.id === id ? { ...c, ...patch, name: clip(patch.name !== undefined ? patch.name : c.name, COST_NAME_MAX) } : c,
    )
    this.#changed()
  }

  /** The fixed cost already linked to this type, if any. */
  costLinkedTo(type: string): CostItem | null {
    return this.plan.costs.find((c) => sameType(c.linkedType, type)) ?? null
  }

  /** Removes a cost item; undo in a toast. */
  removeCost(id: string) {
    const index = this.plan.costs.findIndex((c) => c.id === id)
    if (index < 0) return
    const item = this.plan.costs[index]
    this.plan.costs = this.plan.costs.filter((c) => c.id !== id)
    this.#changed()
    store.toast(`“${item.name || 'Custo'}” removido.`, 'info', {
      label: 'Desfazer',
      run: () => {
        const list = [...this.plan.costs]
        list.splice(Math.min(index, list.length), 0, item)
        this.plan.costs = list
        this.#changed()
      },
    })
  }

  /** "Adicionar linha" of the variable costs: an empty row. Returns its id. */
  addVariableCost(item: { month?: string | null; description?: string; cents?: number | null } = {}): string {
    const id = newId()
    this.plan.variableCosts = [
      ...this.plan.variableCosts,
      { id, month: item.month ?? null, description: clip(item.description ?? '', COST_NAME_MAX), cents: item.cents ?? null },
    ]
    this.#changed()
    return id
  }

  updateVariableCost(id: string, patch: { month?: string | null; description?: string; cents?: number | null }) {
    this.plan.variableCosts = this.plan.variableCosts.map((v) =>
      v.id === id
        ? { ...v, ...patch, description: clip(patch.description !== undefined ? patch.description : v.description, COST_NAME_MAX) }
        : v,
    )
    this.#changed()
  }

  /** Removes a typed variable cost; undo in a toast when it had something typed. */
  removeVariableCost(id: string) {
    const index = this.plan.variableCosts.findIndex((v) => v.id === id)
    if (index < 0) return
    const item = this.plan.variableCosts[index]
    this.plan.variableCosts = this.plan.variableCosts.filter((v) => v.id !== id)
    this.#changed()
    if (item.month == null && item.cents == null && !item.description) return
    store.toast(`“${item.description || 'Gasto'}” removido.`, 'info', {
      label: 'Desfazer',
      run: () => {
        const list = [...this.plan.variableCosts]
        list.splice(Math.min(index, list.length), 0, item)
        this.plan.variableCosts = list
        this.#changed()
      },
    })
  }

  setReserveRate(rate: number) {
    this.plan.reserveRate = Math.min(RESERVE_MAX, Math.max(0, Math.round(rate * 10000) / 10000))
    this.#changed()
  }

  setLeftoverBase(base: LeftoverBase) {
    this.plan.leftoverBase = base
    this.#changed()
  }

  setLeftoverFixed(cents: number | null) {
    this.plan.leftoverFixedCents = cents
    this.#changed()
  }

  setTab(tab: TaxTab) {
    if (this.plan.tab === tab) return
    this.plan.tab = tab
    this.#changed()
  }

  // ------------------------------------------------------------ persistence

  /**
   * Reads the planning of the active company (at startup, when the company or the data folder
   * changes). With `legacy`, the settings of the first version are migrated once.
   */
  async load(legacy = true) {
    const planning = parsePlanning(await this.doc.read())
    const migrated = planning || !legacy ? null : legacyPlanning()
    this.plan = planning ?? migrated ?? defaultPlanning()
    this.doc.markLoaded()
    if (migrated && !this.doc.readBlocked && (await this.doc.saveNow())) removeKey('tax')
  }

  /** "Não salvo" clicked: a failed write is tried again; an unreadable file offers the overwrite. */
  retrySave() {
    this.doc.retry()
  }

  #changed() {
    this.doc.changed()
  }

  /** Writes the planning now. */
  saveNow(): Promise<boolean> {
    return this.doc.saveNow()
  }

  /** Saves right away if a save is waiting (the window is being hidden or closed). */
  flush(): Promise<void> {
    return this.doc.flush()
  }

  // ------------------------------------------------------------ catalog and report

  async loadCatalog() {
    if (this.catalog || this.#catalogLoading) return
    this.#catalogLoading = true
    try {
      this.catalog = await api.taxCatalog()
      this.catalogError = null
    } catch (e) {
      this.catalogError = errorMessage(e)
    } finally {
      this.#catalogLoading = false
    }
  }

  /**
   * Asks the backend for the report of `input` (debounced while the user types; immediate for the
   * first report). Answers to older requests are ignored; the previous report stays on screen while
   * the next one is computed.
   */
  request(input: TaxInput | null) {
    const key = input ? JSON.stringify(input) : ''
    if (key === this.#lastKey) return
    this.#lastKey = key
    clearTimeout(this.#timer)
    const seq = ++this.#seq
    if (!input) {
      this.loading = false
      this.error = null
      this.report = null
      return
    }
    this.loading = true
    this.#timer = setTimeout(
      async () => {
        try {
          const report = await api.taxReport(input)
          if (seq !== this.#seq) return
          this.report = report
          this.error = null
        } catch (e) {
          if (seq !== this.#seq) return
          this.error = errorMessage(e)
        } finally {
          if (seq === this.#seq) this.loading = false
        }
      },
      this.report ? REPORT_DEBOUNCE_MS : 0,
    )
  }

  /** Forgets the last request so the next `request` call asks again (the "Tentar de novo" button). */
  retry() {
    this.#lastKey = null
    this.request(this.input)
  }
}

export const tax = new TaxStore()

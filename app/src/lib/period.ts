// "Período": a range of competence months, used by the notes page (a scope for the cards, the
// table, the totals per type and the exports) and by the tax page (the months considered).

import { addMonths, formatMonthShort, isMonthKey } from './format'
import { isRecord } from './storage'

/**
 * `range` = "Personalizado" (de/até, either side may be open); `custom` = "Seleção manual" (the
 * months ticked one by one, tax page only); `next` = from the current month on (tax page only).
 */
export type PeriodPreset =
  | 'all'
  | 'thisMonth'
  | 'lastMonth'
  | 'last3'
  | 'last12'
  | 'thisYear'
  | 'lastYear'
  | 'next'
  | 'range'
  | 'custom'

export interface Period {
  preset: PeriodPreset
  /** Only for `range`: first and last month (`yyyy-mm`); null = open. */
  from: string | null
  to: string | null
}

/** Month bounds (inclusive); a null side is open. */
export interface MonthRange {
  from: string | null
  to: string | null
}

export const PERIOD_LABEL: Record<PeriodPreset, string> = {
  all: 'Todos',
  thisMonth: 'Este mês',
  lastMonth: 'Mês passado',
  last3: 'Últimos 3 meses',
  last12: 'Últimos 12 meses',
  thisYear: 'Este ano',
  lastYear: 'Ano passado',
  next: 'Próximos meses',
  range: 'Personalizado',
  custom: 'Seleção manual',
}

/** Presets of the notes page, in menu order. */
export const NOTES_PRESETS: PeriodPreset[] = ['all', 'thisMonth', 'lastMonth', 'last3', 'last12', 'thisYear', 'lastYear']
/** Presets of the tax page, in menu order. */
export const TAX_PRESETS: PeriodPreset[] = [
  'all',
  'thisMonth',
  'lastMonth',
  'last3',
  'last12',
  'thisYear',
  'lastYear',
  'next',
]

const ALL_PRESETS = Object.keys(PERIOD_LABEL) as PeriodPreset[]

export const ALL_PERIOD: Period = { preset: 'all', from: null, to: null }


/**
 * The months of a period relative to `current` (`yyyy-mm`). Null = no bound at all ("Todos") or a
 * manual selection, which the caller resolves.
 */
export function periodRange(p: Period, current: string): MonthRange | null {
  const year = Number(current.slice(0, 4))
  switch (p.preset) {
    case 'all':
    case 'custom':
      return null
    case 'thisMonth':
      return { from: current, to: current }
    case 'lastMonth': {
      const m = addMonths(current, -1)
      return { from: m, to: m }
    }
    case 'last3':
      return { from: addMonths(current, -2), to: current }
    case 'last12':
      return { from: addMonths(current, -11), to: current }
    case 'thisYear':
      return { from: `${year}-01`, to: `${year}-12` }
    case 'lastYear':
      return { from: `${year - 1}-01`, to: `${year - 1}-12` }
    case 'next':
      return { from: current, to: null }
    case 'range':
      return p.from || p.to ? { from: p.from, to: p.to } : null
  }
}

export function inRange(month: string, range: MonthRange): boolean {
  return (!range.from || month >= range.from) && (!range.to || month <= range.to)
}

/** "Este ano", "mai/2026 – jul/2026", "desde mai/2026", "até jul/2026"... */
export function periodLabel(p: Period): string {
  if (p.preset !== 'range') return PERIOD_LABEL[p.preset]
  if (p.from && p.to) return p.from === p.to ? formatMonthShort(p.from, 'full') : `${formatMonthShort(p.from, 'full')} – ${formatMonthShort(p.to, 'full')}`
  if (p.from) return `desde ${formatMonthShort(p.from, 'full')}`
  if (p.to) return `até ${formatMonthShort(p.to, 'full')}`
  return PERIOD_LABEL.range
}

/** A stored period, keeping only a known preset (and valid months for "Personalizado"). */
export function parsePeriod(raw: unknown, allowed: PeriodPreset[] = ALL_PRESETS): Period {
  if (!isRecord(raw) || !allowed.includes(raw.preset as PeriodPreset)) return { ...ALL_PERIOD }
  const from = isMonthKey(raw.from) ? raw.from : null
  const to = isMonthKey(raw.to) ? raw.to : null
  const preset = raw.preset as PeriodPreset
  if (preset !== 'range') return { preset, from: null, to: null }
  // A reversed range is swapped; an empty one means "Todos".
  if (from && to && from > to) return { preset, from: to, to: from }
  return from || to ? { preset, from, to } : { ...ALL_PERIOD }
}

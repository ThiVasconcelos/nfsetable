// Formatting and parsing helpers. Money is always integer cents; floats only appear in the final
// Intl formatting step.

const brl = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' })
const integer = new Intl.NumberFormat('pt-BR')

/** 123456 -> "R$ 1.234,56" (Intl output, with a no-break space). */
export function formatBRL(cents: number): string {
  return brl.format(cents / 100)
}

const brlCompact = new Intl.NumberFormat('pt-BR', {
  style: 'currency',
  currency: 'BRL',
  notation: 'compact',
  maximumFractionDigits: 1,
})

/** 985000 -> "R$ 9,9 mil" (chips and chart labels). */
export function formatBRLCompact(cents: number): string {
  return brlCompact.format(cents / 100)
}

/** Same as formatBRL but with regular spaces (for the clipboard / spreadsheets). */
export function formatBRLPlain(cents: number): string {
  return formatBRL(cents).replaceAll('\u00a0', ' ').replaceAll('\u202f', ' ')
}

/** 123456 -> "1.234,56" (no currency symbol), used to prefill the value editor. */
export function formatAmount(cents: number): string {
  const negative = cents < 0
  const abs = Math.abs(cents)
  const units = Math.trunc(abs / 100)
  const rest = String(abs % 100).padStart(2, '0')
  return `${negative ? '-' : ''}${integer.format(units)},${rest}`
}

export function formatInt(n: number): string {
  return integer.format(n)
}

/** "1 nota" / "3 notas". */
export function plural(n: number, singular: string, pluralForm: string): string {
  return `${formatInt(n)} ${n === 1 ? singular : pluralForm}`
}

export type MoneyInput = { kind: 'empty' } | { kind: 'value'; cents: number } | { kind: 'invalid' }

/**
 * Parses what the user typed in the value editor, without floats.
 * Accepts "1.234,56", "1234,56", "1234.56", "R$ 1.234,56", "1234", "-10,00", "1.234".
 */
export function parseMoneyInput(input: string): MoneyInput {
  let s = withoutCurrency(input.replace(/\s|\u00a0/g, ''))
  if (s === '') return { kind: 'empty' }
  let negative = false
  if (s.startsWith('-')) {
    negative = true
    s = s.slice(1)
  }
  s = withoutCurrency(s)
  if (!/^[\d.,]+$/.test(s)) return { kind: 'invalid' }

  let intPart: string
  let decPart = ''
  const lastComma = s.lastIndexOf(',')
  const lastDot = s.lastIndexOf('.')
  if (lastComma >= 0) {
    // Brazilian style: dots are thousands separators, the comma is the decimal separator.
    if (s.indexOf(',') !== lastComma) return { kind: 'invalid' }
    intPart = s.slice(0, lastComma).replaceAll('.', '')
    decPart = s.slice(lastComma + 1)
  } else if (lastDot >= 0) {
    const tail = s.slice(lastDot + 1)
    const single = s.indexOf('.') === lastDot
    if (single && tail.length > 0 && tail.length <= 2) {
      // "1234.5" / "1234.56": a decimal point.
      intPart = s.slice(0, lastDot)
      decPart = tail
    } else {
      // "1.234" / "1.234.567": thousands separators.
      intPart = s.replaceAll('.', '')
    }
  } else {
    intPart = s
  }
  if (intPart === '') intPart = '0'
  if (!/^\d+$/.test(intPart) || !/^\d{0,2}$/.test(decPart)) return { kind: 'invalid' }
  if (intPart.length > 13) return { kind: 'invalid' }
  const cents = Number(intPart) * 100 + Number(decPart.padEnd(2, '0'))
  if (!Number.isSafeInteger(cents)) return { kind: 'invalid' }
  return { kind: 'value', cents: negative ? -cents : cents }
}

/** "R$1.234,56" -> "1.234,56" (a leading "R$", any case). */
function withoutCurrency(text: string): string {
  return text.slice(0, 2).toUpperCase() === 'R$' ? text.slice(2) : text
}

// ---------------------------------------------------------------- months (competence)

/** A month key `yyyy-mm` ("2026-07"). */
const MONTH_RE = /^(?<year>\d{4})-(?<month>0[1-9]|1[0-2])$/
const MONTHS_SHORT = ['jan', 'fev', 'mar', 'abr', 'mai', 'jun', 'jul', 'ago', 'set', 'out', 'nov', 'dez']
const MONTHS_LONG = [
  'janeiro',
  'fevereiro',
  'março',
  'abril',
  'maio',
  'junho',
  'julho',
  'agosto',
  'setembro',
  'outubro',
  'novembro',
  'dezembro',
]

/** True for a `yyyy-mm` month key. */
export function isMonthKey(value: unknown): value is string {
  return typeof value === 'string' && MONTH_RE.test(value)
}

/** ISO date "2026-07-15" -> month key "2026-07" (null when missing or malformed). */
export function monthOfDate(date: string | null | undefined): string | null {
  if (!date) return null
  const month = date.slice(0, 7)
  return isMonthKey(month) ? month : null
}

/** Year and month (1-12) of a month key; null when it is not one. */
function monthParts(month: string): { year: string; index: number } | null {
  const groups = MONTH_RE.exec(month)?.groups
  return groups ? { year: groups.year, index: Number(groups.month) - 1 } : null
}

/** `yyyy-mm` of a date (today by default). */
export function monthKeyOf(date: Date = new Date()): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}`
}

/** "2026-07" -> "07/2026". */
export function formatMonth(month: string): string {
  const m = monthParts(month)
  return m ? `${String(m.index + 1).padStart(2, '0')}/${m.year}` : month
}

/**
 * "2026-07" -> "jul"; with `year`, "jul/26" (`'short'`) or "jul/2026" (`'full'`). The one month label
 * of tables, chips and texts.
 */
export function formatMonthShort(month: string, year?: 'short' | 'full'): string {
  const m = monthParts(month)
  if (!m) return month
  const name = MONTHS_SHORT[m.index]
  if (year === 'full') return `${name}/${m.year}`
  return year === 'short' ? `${name}/${m.year.slice(2)}` : name
}

/** A span of months: "jul/26", "abr a jul/26" (same year), "dez/25 a jul/26". */
export function formatMonthRange(first: string, last: string): string {
  if (first === last) return formatMonthShort(last, 'short')
  const sameYear = first.slice(0, 4) === last.slice(0, 4)
  return `${formatMonthShort(first, sameYear ? undefined : 'short')} a ${formatMonthShort(last, 'short')}`
}

/** "2026-07" -> "julho de 2026". */
export function formatMonthLong(month: string): string {
  const m = monthParts(month)
  return m ? `${MONTHS_LONG[m.index]} de ${m.year}` : month
}

/** Adds `delta` months to a month key: ("2026-01", -1) -> "2025-12". */
export function addMonths(month: string, delta: number): string {
  const m = monthParts(month)
  if (!m) return month
  const index = Number(m.year) * 12 + m.index + delta
  return `${Math.floor(index / 12)}-${String((index % 12) + 1).padStart(2, '0')}`
}

export type MonthInput = { kind: 'empty' } | { kind: 'value'; month: string } | { kind: 'invalid' }

/** Month number (1-12) of a pt-BR month name or abbreviation ("mar", "março", "marco"); 0 if none. */
function monthOfName(word: string): number {
  if (word.length < 3) return 0
  const index = MONTHS_LONG.findIndex((name) => normalizeText(name).startsWith(word))
  return index + 1
}

/**
 * Parses a month typed by the user. Accepts "07/2026", "7/2026", "07/26", "3/26", "072026",
 * "07-2026", "07.2026", ISO "2026-07" and pt-BR month names: "mar/26", "mar 2026", "março 2026",
 * "março de 2026" (accents and case ignored). Years from 2000 to 2099.
 */
export function parseMonthInput(input: string): MonthInput {
  const text = normalizeText(input)
  if (text === '') return { kind: 'empty' }
  let month: number
  let year: number
  const named = /^([a-z]+)\.?(?:\s*(?:de|\/|-|\.)\s*|\s+)?(\d{2}|\d{4})$/.exec(text)
  const s = text.replaceAll(' ', '')
  let m: RegExpExecArray | null
  if (named) {
    month = monthOfName(named[1])
    if (!month) return { kind: 'invalid' }
    year = named[2].length === 2 ? 2000 + Number(named[2]) : Number(named[2])
  } else if ((m = /^(\d{4})[-/.](\d{1,2})$/.exec(s))) {
    year = Number(m[1])
    month = Number(m[2])
  } else if ((m = /^(\d{1,2})[-/.](\d{2}|\d{4})$/.exec(s))) {
    month = Number(m[1])
    year = m[2].length === 2 ? 2000 + Number(m[2]) : Number(m[2])
  } else if ((m = /^(\d{2})(\d{4})$/.exec(s))) {
    month = Number(m[1])
    year = Number(m[2])
  } else {
    return { kind: 'invalid' }
  }
  if (month < 1 || month > 12 || year < 2000 || year > 2099) return { kind: 'invalid' }
  return { kind: 'value', month: `${year}-${String(month).padStart(2, '0')}` }
}

/**
 * Typing mask of the month fields: digits typed forward get the slash of "MM/AAAA" ("3" ->
 * "03/", "03" -> "03/", "032026" -> "03/2026"); anything with letters is left alone ("mar 26").
 * `grew` = the user added characters (deleting never re-inserts the slash).
 */
export function maskMonthTyping(raw: string, grew: boolean): string {
  if (!grew || !/^[\d/]*$/.test(raw)) return raw
  // A slash typed right after the one inserted by the mask ("3" -> "03/", then "/") is dropped.
  const value = raw.replace(/\/{2,}/g, '/')
  if (/^[2-9]$/.test(value)) return `0${value}/`
  if (/^\d{2}$/.test(value)) return Number(value) >= 1 && Number(value) <= 12 ? `${value}/` : value
  const digits = /^(\d{2})(\d{1,4})$/.exec(value)
  if (digits && Number(digits[1]) >= 1 && Number(digits[1]) <= 12) return `${digits[1]}/${digits[2]}`
  return value.slice(0, 7)
}

/** Local date and time for display and exports: "29/09/2026 14:05". */
export function formatDateTime(date: Date): string {
  const two = (n: number) => String(n).padStart(2, '0')
  const day = `${two(date.getDate())}/${two(date.getMonth() + 1)}/${date.getFullYear()}`
  return `${day} ${two(date.getHours())}:${two(date.getMinutes())}`
}

// ---------------------------------------------------------------- rates

const percent2 = new Intl.NumberFormat('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })

/** 0.0612 -> "6,12%" (always two decimals). */
export function formatPercent(rate: number): string {
  return `${percent2.format(Number.isFinite(rate) ? rate * 100 : 0)}%`
}

/** 0.05 -> "5" / 0.025 -> "2,5": a rate as the user types it (no % sign, up to 2 decimals). */
export function formatRateInput(rate: number): string {
  return (Math.round(rate * 10000) / 100).toLocaleString('pt-BR', { maximumFractionDigits: 2 })
}

/** "5" / "2,5" / "2.5%" -> 0.05 / 0.025; null when invalid. */
export function parseRateInput(input: string): number | null {
  const s = input.replace(/\s|%/g, '').replace(',', '.')
  if (!/^\d{1,2}(\.\d{1,2})?$/.test(s)) return null
  return Math.round(Number(s) * 100) / 10000
}

/** Runs of whitespace (collapsed to one space). */
const SPACES_RE = /\s+/g
/** Combining marks left by the NFD normalization (the accents). */
const MARKS_RE = /[\u0300-\u036f]/g

/** Whitespace runs as one space, trimmed: "  a \n b " -> "a b". */
export function collapseSpaces(text: string): string {
  return text.replace(SPACES_RE, ' ').trim()
}

/** Lowercase, accents removed, whitespace collapsed (same idea as the core `text::normalize`). */
export function normalizeText(text: string): string {
  return collapseSpaces(text.normalize('NFD').replace(MARKS_RE, '').toLowerCase())
}

/** The same text ignoring case, accents and spacing; empty or missing texts are never the same. */
export function sameText(a: string | null | undefined, b: string | null | undefined): boolean {
  return !!a && !!b && normalizeText(a) === normalizeText(b)
}

/** A sentence without its final period, for messages that go inside another one. */
export function withoutFinalPeriod(text: string): string {
  const trimmed = text.trimEnd()
  return trimmed.endsWith('.') ? trimmed.slice(0, -1) : trimmed
}

/** `normalizeText` without any space (the core `text::compact`): "Valor  Líquido" -> "valorliquido". */
export function compactText(text: string): string {
  return normalizeText(text).replaceAll(' ', '')
}

/** Runs of anything but lowercase letters and digits (one dash in a slug). */
const NON_SLUG_RE = /[^a-z0-9]+/g

/** "Nova SaaS Ltda." -> "nova-saas-ltda", at most `max` characters, no dash at the ends. */
export function slugify(text: string, max = Infinity): string {
  return trimDashes(trimDashes(normalizeText(text).replace(NON_SLUG_RE, '-')).slice(0, max))
}

function trimDashes(text: string): string {
  let start = 0
  let end = text.length
  while (start < end && text[start] === '-') start++
  while (end > start && text[end - 1] === '-') end--
  return text.slice(start, end)
}

function segments(path: string): string[] {
  return path.replaceAll('\\', '/').split('/').filter(Boolean)
}

/** Last path segment ("C:\\a\\b.pdf" -> "b.pdf"). */
export function baseName(path: string): string {
  const parts = segments(path)
  return parts.length ? parts[parts.length - 1] : path
}

/** The path or file name ends in ".pdf" (any case). */
export function isPdfPath(path: string): boolean {
  return path.toLowerCase().endsWith('.pdf')
}

/** File name without the .pdf extension. */
export function fileStem(name: string): string {
  return isPdfPath(name) ? name.slice(0, -4) : name
}

/** A path without the separators at its end: "C:/Notas/" -> "C:/Notas". */
export function trimTrailingSeparators(path: string): string {
  let end = path.length
  while (end > 0 && (path[end - 1] === '/' || path[end - 1] === '\\')) end--
  return path.slice(0, end)
}

/** Shortens a path to its last segments: "C:/Users/fulano/Notas/2026" -> "…/Notas/2026". */
export function shortenPath(path: string, keep = 2): string {
  const parts = segments(path)
  if (parts.length <= keep) return path
  const sep = path.includes('\\') && !path.includes('/') ? '\\' : '/'
  return `…${sep}${parts.slice(-keep).join(sep)}`
}

/** Key used to compare paths (separator- and case-insensitive, no trailing separator). */
export function pathKey(path: string): string {
  return trimTrailingSeparators(path).replaceAll('\\', '/').toLowerCase()
}

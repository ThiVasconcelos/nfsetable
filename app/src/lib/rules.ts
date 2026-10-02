// File name patterns of the profiles ("regras"), the same way the core matches them
// (crates/core/src/profile.rs), plus the suggestions used by "Criar regra a partir desta nota".

import { fileStem, normalizeText } from './format'
import type { Profile } from './types'

/**
 * True when `pattern` matches the file name: case- and accent-insensitive; `*` is any run of
 * characters and `?` one character (the whole name must match); a pattern without wildcards
 * matches anywhere in the name.
 */
export function namePatternMatches(pattern: string, fileName: string): boolean {
  const p = normalizeText(pattern)
  if (!p) return false
  const name = normalizeText(fileName)
  if (!p.includes('*') && !p.includes('?')) return name.includes(p)
  let source = '^'
  for (const c of p) {
    if (c === '*') source += '.*'
    else if (c === '?') source += '.'
    else source += c.replace(/[.*+?^${}()|[\]\\/]/g, '\\$&')
  }
  return new RegExp(`${source}$`, 'u').test(name)
}

/** True when at least one pattern matches, or when there is no pattern (any file). */
export function nameMatches(patterns: readonly string[], fileName: string): boolean {
  return !patterns.length || patterns.some((p) => namePatternMatches(p, fileName))
}

/** The profile classifies documents (sets a type or a kind). */
export function classifies(profile: Profile): boolean {
  return !!profile.docType || !!profile.kind
}

/** The profile has rules to read values (e.g. a region drawn on the page). */
export function hasValueRules(profile: Profile): boolean {
  return Object.values(profile.fields).some((rules) => rules.length > 0)
}

const MONTH_WORDS = new Set([
  'janeiro', 'fevereiro', 'marco', 'abril', 'maio', 'junho', 'julho', 'agosto', 'setembro', 'outubro', 'novembro',
  'dezembro', 'jan', 'fev', 'mar', 'abr', 'mai', 'jun', 'jul', 'ago', 'set', 'out', 'nov', 'dez',
])

/** Words of a file name that identify it (no numbers, dates or month names). */
function nameWords(fileName: string): string[] {
  return normalizeText(fileStem(fileName))
    .split(/[^a-z0-9]+/)
    .filter((w) => w.length >= 3 && !/^\d+$/.test(w) && !MONTH_WORDS.has(w))
}

/**
 * Name pattern suggested for the documents like this one: its first meaningful word, e.g.
 * "internet-2026-05.pdf" -> "internet", "aluguel-junho.pdf" -> "aluguel".
 */
export function suggestNamePattern(fileName: string): string {
  return nameWords(fileName)[0] ?? normalizeText(fileStem(fileName))
}

/** Rule name suggested from a note: its type when it has a meaningful one, else the file name word. */
export function suggestRuleName(fileName: string, docType: string): string {
  const generic = !docType || normalizeText(docType) === 'pdf' || normalizeText(docType) === 'nfs-e'
  const base = generic ? (nameWords(fileName)[0] ?? fileStem(fileName)) : docType
  return base.charAt(0).toUpperCase() + base.slice(1)
}

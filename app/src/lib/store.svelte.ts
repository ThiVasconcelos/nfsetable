// Application state (Svelte 5 runes). One store instance drives the whole UI.
//
// Data flow: sources/exclude change -> scan -> extract (in chunks, cached by file hash) -> rows.
// Rows combine the scan, the extraction results and the user's overrides (keyed by file hash, so
// manual edits survive rescans and renames).
//
// The notes data of the active company (sources, exclude terms, overrides, hidden copies, "somar
// duplicadas", the period) lives in the backend store document "notes-<company>" (in the data
// folder, so it can be backed up or synced); UI preferences (theme, sort, sidebar) stay in
// localStorage and are shared by every company. See ./companies.svelte.ts.

import { SvelteSet } from 'svelte/reactivity'
import * as api from './api'
import { errorMessage, isTauri } from './api'
import { copyText } from './clipboard'
import {
  baseName,
  collapseSpaces,
  fileStem,
  formatBRLPlain,
  formatDateTime,
  isMonthKey,
  monthKeyOf,
  monthOfDate,
  normalizeText,
  pathKey,
  plural,
} from './format'
import { EXPORT_FORMATS, STATUS_LABEL, exportFileName, originLabel, type RowStatus } from './labels'
import { StoreDoc } from './persist.svelte'
import {
  ALL_PERIOD,
  NOTES_PRESETS,
  inRange,
  parsePeriod,
  periodRange,
  type MonthRange,
  type Period,
} from './period'
import { suggestNamePattern, suggestRuleName } from './rules'
import { isRecord, isStringArray, loadJSON, removeKey, saveJSON } from './storage'
import type {
  AppInfo,
  DocKind,
  DocResult,
  ExportFormat,
  ExportRequest,
  Origin,
  Profile,
  Progress,
  Rect,
  RegionRead,
  Rule,
  RuleTest,
  ScanResult,
  ScannedFile,
  Source,
  SourceInfo,
  Status,
} from './types'
import { COMPETENCE_FIELD, NET_VALUE_FIELD, SERVICE_VALUE_FIELD } from './types'

export type ThemeChoice = 'system' | 'light' | 'dark'
/** Filters of the segmented control. */
export type BaseFilter = 'all' | 'ok' | 'attention' | 'edited'
/** `noCompetence`: the rows sent from the tax page to have their competence filled in. */
export type Filter = BaseFilter | 'noCompetence'
/** Receitas / Despesas / Todas, combined with the other filters. */
export type KindFilter = 'all' | DocKind
/** Top-level page. */
export type View = 'notes' | 'taxes'

export type SortKey = 'file' | 'type' | 'kind' | 'competence' | 'value' | 'status' | 'origin'
export type SortDir = 'asc' | 'desc'
export interface SortState {
  key: SortKey
  dir: SortDir
}

/** User changes to one file, keyed by its hash. */
export interface Override {
  /** Value set by the user (typed, or applied from a region test). */
  cents?: number
  origin?: 'manual' | 'region'
  /** Where a region value was read (for the preview highlight). */
  raw?: string
  page?: number
  bbox?: Rect
  docType?: string
  /** Revenue or expense chosen by the user. */
  kind?: DocKind
  /** Competence month typed by the user, `yyyy-mm`. */
  competence?: string
  removed?: boolean
}

/** "Período" of the notes page: a scope for the cards, the table, the totals and the exports. */
export interface NotesPeriod extends Period {
  /** Rows without a competence month are listed too (always with "Todos"). */
  includeUndated: boolean
}

/** The notes document of a company in the backend store ("notes-<company>"). */
export interface NotesDoc {
  version: 1
  sources: Source[]
  exclude: string[]
  overrides: Record<string, Override>
  /** Duplicate copies hidden by the user (by path). */
  hiddenCopies: string[]
  includeDuplicates: boolean
  period: NotesPeriod
}

/** The company whose notes are open (name for the exports, slug for the file names). */
export interface CompanyTag {
  name: string
  slug: string
}

/** Prefill of the rule editor ("Nova regra", "Criar regra a partir desta nota"). */
export interface RuleSeed {
  name: string
  namePatterns: string[]
  fingerprint: string[]
  docType: string
  kind: DocKind | null
}

export interface Highlight {
  page: number
  bbox: Rect
}

export interface Row {
  path: string
  name: string
  dir: string
  /** Identity of the file for the user's edits (see fileKey). */
  key: string
  /** Normalized name for search. */
  searchKey: string
  duplicateOf: string | null
  duplicateName: string | null
  result: DocResult | null
  extractionStatus: Status | null
  status: RowStatus
  cents: number | null
  origin: Origin | null
  highlight: Highlight | null
  defaultType: string
  docType: string
  /** Revenue or expense: the user's choice, else the one given by the profiles (default revenue). */
  kind: DocKind
  defaultKind: DocKind
  /** Gross value read from the document ("Valor do serviço"), if any. */
  serviceCents: number | null
  /** Competence month (`yyyy-mm`): the user's, else the one read from the document. */
  competence: string | null
  /** Competence month read from the document. */
  extractedCompetence: string | null
  valueEdited: boolean
  typeEdited: boolean
  kindEdited: boolean
  competenceEdited: boolean
  edited: boolean
  /** Needs the user: extraction finished without a value (and no manual value). */
  attention: boolean
  /** Typed as a cancelled note: shown, but never summed. */
  cancelled: boolean
  /** Counts towards the totals. */
  counted: boolean
  message: string | null
  pageCount: number
}

export interface TypeTotal {
  type: string
  kind: DocKind
  count: number
  cents: number
}

export interface Toast {
  id: number
  kind: 'info' | 'success' | 'error'
  message: string
  action?: { label: string; run: () => void }
}

export interface RegionSelection {
  seq: number
  path: string
  page: number
  rect: Rect
  loading: boolean
  read: RegionRead | null
  error: string | null
}

export interface RuleTestState {
  seq: number
  rule: Rule
  scope: 'attention' | 'all'
  sourcePath: string
  paths: string[]
  running: boolean
  progress: Progress
  results: RuleTest[]
  error: string | null
  saving: boolean
}

const CHUNK_SIZE = 8
/** While files are read, the table is rebuilt at most this often (once per chunk is O(n²)). */
const PUBLISH_MS = 200
const DEFAULT_EXCLUDE = ['cancelada']
/** Types offered by the type editor. "Bônus" notes count as revenue like any other. */
export const BASE_TYPES = ['NFS-e', 'PDF', 'Cancelada', 'Bônus']
const SORT_KEYS: SortKey[] = ['file', 'type', 'kind', 'competence', 'value', 'status', 'origin']
/** Keys of the first versions, kept in localStorage; migrated once into the "notes" document. */
const LEGACY_NOTE_KEYS = ['sources', 'exclude', 'overrides', 'hiddenCopies', 'includeDuplicates']
export const KIND_LABEL: Record<DocKind, string> = { revenue: 'Receita', expense: 'Despesa' }

// ---------------------------------------------------------------- persistence validators

function isSources(value: unknown): value is Source[] {
  return (
    Array.isArray(value) &&
    value.every((s) => isRecord(s) && typeof s.path === 'string' && typeof s.recursive === 'boolean')
  )
}

function isTheme(value: unknown): value is ThemeChoice {
  return value === 'system' || value === 'light' || value === 'dark'
}

function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean'
}

function isRect(value: unknown): value is Rect {
  return isRecord(value) && ['x', 'y', 'w', 'h'].every((k) => typeof value[k] === 'number')
}

function isSort(value: unknown): value is SortState | null {
  return (
    value === null ||
    (isRecord(value) && SORT_KEYS.includes(value.key as SortKey) && (value.dir === 'asc' || value.dir === 'desc'))
  )
}

function isKind(value: unknown): value is DocKind {
  return value === 'revenue' || value === 'expense'
}

/**
 * Identity of a file for the cache and the user's edits: its SHA-256, or its path when the scan
 * could not read it (locked by another program, no permission), since every such file has the
 * same empty hash.
 */
function fileKey(f: ScannedFile): string {
  return f.hash || `path:${pathKey(f.path)}`
}

/**
 * Key of a file's extraction result: its content and its name, since profiles may match by name
 * (the same content under another name can read differently). Empty when the scan could not read
 * the file.
 */
function cacheKey(f: ScannedFile): string {
  return f.hash ? `${f.hash}:${normalizeText(f.name)}` : ''
}

/** Overrides keeping only well-typed fields. */
function parseOverrides(raw: unknown): Record<string, Override> {
  const out: Record<string, Override> = {}
  if (!isRecord(raw)) return out
  for (const [hash, value] of Object.entries(raw)) {
    // "" was the shared key of unreadable files in the first versions: it belongs to no file.
    if (!hash || !isRecord(value)) continue
    const o: Override = {}
    if (typeof value.cents === 'number' && Number.isSafeInteger(value.cents)) {
      o.cents = value.cents
      o.origin = value.origin === 'region' ? 'region' : 'manual'
      if (typeof value.raw === 'string') o.raw = value.raw
      if (typeof value.page === 'number') o.page = value.page
      if (isRect(value.bbox)) o.bbox = value.bbox
    }
    if (typeof value.docType === 'string' && value.docType.trim()) o.docType = value.docType
    if (isKind(value.kind)) o.kind = value.kind
    if (isMonthKey(value.competence)) o.competence = value.competence
    if (value.removed === true) o.removed = true
    if (Object.keys(o).length) out[hash] = o
  }
  return out
}

function defaultPeriod(): NotesPeriod {
  return { ...ALL_PERIOD, includeUndated: false }
}

function parseNotesPeriod(raw: unknown): NotesPeriod {
  const period = parsePeriod(raw, [...NOTES_PRESETS, 'range'])
  return { ...period, includeUndated: isRecord(raw) && raw.includeUndated === true }
}

function defaultNotes(): NotesDoc {
  return {
    version: 1,
    sources: [],
    exclude: [...DEFAULT_EXCLUDE],
    overrides: {},
    hiddenCopies: [],
    includeDuplicates: false,
    period: defaultPeriod(),
  }
}

/** Validates a stored notes document, keeping every well-typed field (null when it is not one). */
export function parseNotes(raw: unknown): NotesDoc | null {
  if (!isRecord(raw) || typeof raw.version !== 'number') return null
  const d = defaultNotes()
  return {
    version: 1,
    sources: isSources(raw.sources) ? raw.sources.map((s) => ({ path: s.path, recursive: s.recursive })) : d.sources,
    exclude: isStringArray(raw.exclude) ? raw.exclude : d.exclude,
    overrides: parseOverrides(raw.overrides),
    hiddenCopies: isStringArray(raw.hiddenCopies) ? raw.hiddenCopies : d.hiddenCopies,
    includeDuplicates: isBoolean(raw.includeDuplicates) ? raw.includeDuplicates : d.includeDuplicates,
    period: parseNotesPeriod(raw.period),
  }
}

const isPresent = (v: unknown): v is unknown => v !== undefined

/** The notes data of the first versions (localStorage), or null when there is none. */
function legacyNotes(): NotesDoc | null {
  if (!LEGACY_NOTE_KEYS.some((key) => loadJSON<unknown>(key, undefined, isPresent) !== undefined)) return null
  const d = defaultNotes()
  return {
    version: 1,
    sources: loadJSON('sources', d.sources, isSources),
    exclude: loadJSON('exclude', d.exclude, isStringArray),
    overrides: parseOverrides(loadJSON<Record<string, unknown>>('overrides', {}, isRecord)),
    hiddenCopies: loadJSON('hiddenCopies', d.hiddenCopies, isStringArray),
    includeDuplicates: loadJSON('includeDuplicates', d.includeDuplicates, isBoolean),
    period: defaultPeriod(),
  }
}

/** "Cancelada" (any case/accents) marks a cancelled note, which never counts towards the total. */
const cancelledTypes = new Map<string, boolean>()

/** Typed as a cancelled note (remembered: the table asks for every row on every rebuild). */
export function isCancelledType(type: string): boolean {
  let cancelled = cancelledTypes.get(type)
  if (cancelled === undefined) {
    cancelled = normalizeText(type).startsWith('cancelad')
    cancelledTypes.set(type, cancelled)
  }
  return cancelled
}

/** "Bônus" (any case/accents): revenue that "sobra do mês" can leave out of its base. */
export function isBonusType(type: string): boolean {
  return normalizeText(type).startsWith('bonus')
}

/** `nameKey`: the normalized file name. */
function defaultTypeFor(nameKey: string, result: DocResult | null): string {
  if (nameKey.includes('cancelad')) return 'Cancelada'
  return result?.docType ?? ''
}

// ---------------------------------------------------------------- sorting

const collator = new Intl.Collator('pt-BR', { numeric: true, sensitivity: 'base' })

function sortValue(row: Row, key: SortKey): string | number | null {
  switch (key) {
    case 'file':
      return row.name
    case 'type':
      return row.docType || null
    case 'kind':
      return KIND_LABEL[row.kind]
    case 'competence':
      return row.competence
    case 'value':
      return row.cents
    case 'status':
      return STATUS_LABEL[row.status]
    case 'origin':
      return originLabel(row.origin) || null
  }
}

/** Stable sort; empty values go last in both directions (like spreadsheets). */
function sortRows(rows: Row[], sort: SortState): Row[] {
  const keyed = rows.map((row, index) => ({ row, index, value: sortValue(row, sort.key) }))
  const dir = sort.dir === 'asc' ? 1 : -1
  keyed.sort((a, b) => {
    if (a.value == null || b.value == null) {
      if (a.value == null && b.value == null) return a.index - b.index
      return a.value == null ? 1 : -1
    }
    const c =
      typeof a.value === 'number' && typeof b.value === 'number'
        ? a.value - b.value
        : collator.compare(String(a.value), String(b.value))
    return c !== 0 ? c * dir : a.index - b.index
  })
  return keyed.map((k) => k.row)
}

// ---------------------------------------------------------------- store

export class AppStore {
  // Boot
  boot = $state<'loading' | 'ready' | 'failed'>('loading')
  bootError = $state<string | null>(null)
  info = $state.raw<AppInfo | null>(null)

  // Notes data (the "notes" store document, read at startup) and preferences (localStorage)
  sources = $state<Source[]>([])
  exclude = $state<string[]>([...DEFAULT_EXCLUDE])
  theme = $state<ThemeChoice>(loadJSON('theme', 'system', isTheme))
  overrides = $state<Record<string, Override>>({})
  includeDuplicates = $state<boolean>(false)
  /** Duplicate copies hidden by the user (by path; the original document stays listed). */
  hiddenCopies = new SvelteSet<string>()
  period = $state<NotesPeriod>(defaultPeriod())
  /** The active company (set by ./companies.svelte.ts). */
  company = $state.raw<CompanyTag | null>(null)
  /** No company is open yet ("Escolha a empresa"): files dropped on the window are ignored. */
  locked = $state(false)
  /** Saving of the notes data (debounced; paused when the saved document cannot be read). */
  notesDoc = new StoreDoc<NotesDoc>({
    name: 'notes',
    readWhat: 'os dados das notas',
    saveWhat: 'os dados das notas',
    snapshot: () => this.#notesSnapshot(),
    notify: (message, kind, action) => this.toast(message, kind, action),
  })

  // Scan + extraction
  scan = $state.raw<ScanResult | null>(null)
  results = $state.raw<Record<string, DocResult>>({})
  phase = $state<'idle' | 'scanning' | 'extracting'>('idle')
  progress = $state<Progress>({ done: 0, total: 0, run: 0 })

  // Page
  view = $state<View>('notes')

  // Table
  filter = $state<Filter>('all')
  kindFilter = $state<KindFilter>('all')
  search = $state('')
  sort = $state<SortState | null>(loadJSON('sort', null, isSort))
  selected = new SvelteSet<string>()
  activePath = $state<string | null>(null)
  /** Row whose value editor should open (set by the table's Enter key). */
  editingPath = $state<string | null>(null)
  /** Row whose competence editor should open (set by a double-click on the cell). */
  editingCompetencePath = $state<string | null>(null)
  /** Rows sent from the tax page to have their competence filled in (filter `noCompetence`). */
  competenceTodo = $state.raw<ReadonlySet<string> | null>(null)

  // Profiles, dialogs, region tools
  profiles = $state.raw<Profile[]>([])
  profilesOpen = $state(false)
  /** Rule editor to open with the "Perfis e regras" dialog (null = the list). */
  ruleSeed = $state.raw<RuleSeed | null>(null)
  settingsOpen = $state(false)
  excludedOpen = $state(false)
  region = $state<RegionSelection | null>(null)
  ruleTest = $state<RuleTestState | null>(null)
  dragging = $state(false)
  toasts = $state<Toast[]>([])
  /** Sidebar collapsed to a rail: null = automatic (collapses while the preview is open). */
  sidebarOverride = $state<boolean | null>(null)

  /** Current month: the presets of the period ("Este mês", "Últimos 3 meses"...). */
  readonly currentMonth = monthKeyOf()

  // Internals (not reactive)
  #cache = new Map<string, DocResult>()
  /** Bumped by forgetCache(): a run that started before it neither caches nor shows its results. */
  #generation = 0
  #dirty = false
  #running = false
  #chunkBase = 0
  #seq = 0
  /** Last row toggled by the user: the other end of a Shift+click range. */
  #anchor: string | null = null

  // ------------------------------------------------------------ derived state

  /** Normalized file names, once per scan (not once per row on every rebuild). */
  #nameKeys = $derived.by(() => new Map((this.scan?.files ?? []).map((f) => [f.path, normalizeText(f.name)])))

  rows = $derived.by(() => this.#buildRows())

  /** Months of the period (null = every row, "Todos"). */
  periodRange = $derived<MonthRange | null>(periodRange(this.period, this.currentMonth))

  /**
   * Rows in the period: its scope for the cards, the table, the totals and the exports. Rows without
   * a competence month only with "Todos" or "Incluir notas sem competência".
   */
  periodRows = $derived.by(() => {
    const range = this.periodRange
    if (!range) return this.rows
    const undated = this.period.includeUndated
    return this.rows.filter((r) => (r.competence ? inRange(r.competence, range) : undated))
  })

  /** What the period leaves out: rows outside its months and rows without a month. */
  periodInfo = $derived.by(() => {
    const range = this.periodRange
    let outside = 0
    let undated = 0
    let undatedAll = 0
    for (const r of this.rows) {
      if (!r.competence) {
        undatedAll++
        if (range && !this.period.includeUndated) undated++
      } else if (range && !inRange(r.competence, range)) outside++
    }
    return { active: !!range, outside, undated, undatedAll, left: outside + undated }
  })

  /** Rows of the chosen kind (Receitas / Despesas / Todas), in the period. */
  kindRows = $derived.by(() =>
    this.kindFilter === 'all' ? this.periodRows : this.periodRows.filter((r) => r.kind === this.kindFilter),
  )

  /**
   * Rows shown in the table: period, filters and search applied, in the current sort. The rows sent
   * from the tax page to get a competence are shown whatever the period (they have no month yet).
   */
  visibleRows = $derived.by(() => {
    const q = normalizeText(this.search)
    const base = this.filter === 'noCompetence' ? this.rows : this.kindRows
    const list = base.filter((r) => this.#matchesFilter(r, this.filter) && (!q || r.searchKey.includes(q)))
    return this.sort ? sortRows(list, this.sort) : list
  })

  /** Selected rows (in table order, including rows hidden by the filter). */
  selectedRows = $derived.by(() => this.rows.filter((r) => this.selected.has(r.path)))

  /** Counts of the status filters, among the rows of the chosen kind. */
  counts = $derived.by(() => {
    const rows = this.kindRows
    return {
      all: rows.length,
      ok: rows.filter((r) => r.status === 'ok').length,
      attention: rows.filter((r) => r.attention).length,
      edited: rows.filter((r) => r.edited).length,
    }
  })

  /** Totals of the rows in the period, revenue and expenses apart. */
  totals = $derived.by(() => {
    let revenue = 0
    let revenueCount = 0
    let expense = 0
    let expenseCount = 0
    let expenseRows = 0
    let selectedRevenue = 0
    let selectedExpense = 0
    let selectedCount = 0
    let duplicates = 0
    let cancelled = 0
    let attention = 0
    for (const r of this.periodRows) {
      const isSelected = this.selected.has(r.path)
      if (isSelected) selectedCount++
      if (r.duplicateOf) duplicates++
      if (r.cancelled) cancelled++
      if (r.attention) attention++
      if (r.kind === 'expense') expenseRows++
      if (!r.counted || r.cents == null) continue
      if (r.kind === 'expense') {
        expense += r.cents
        expenseCount++
        if (isSelected) selectedExpense += r.cents
      } else {
        revenue += r.cents
        revenueCount++
        if (isSelected) selectedRevenue += r.cents
      }
    }
    return {
      revenue,
      revenueCount,
      expense,
      expenseCount,
      expenseRows,
      selectedRevenue,
      selectedExpense,
      selectedCount,
      duplicates,
      cancelled,
      attention,
    }
  })

  /** Subtotals per type in the period, revenue types first (then expenses), largest first. */
  typeTotals = $derived.by(() => {
    const map = new Map<string, TypeTotal>()
    for (const r of this.periodRows) {
      // Cancelled notes are left out of the total but still get their own subtotal.
      const listed = r.counted || (r.cancelled && (!r.duplicateOf || this.includeDuplicates))
      if (!listed || r.cents == null) continue
      const type = r.docType || 'Sem tipo'
      const key = `${r.kind}:${type}`
      const t = map.get(key) ?? { type, kind: r.kind, count: 0, cents: 0 }
      t.count++
      t.cents += r.cents
      map.set(key, t)
    }
    return [...map.values()].sort((a, b) => (a.kind === b.kind ? b.cents - a.cents : a.kind === 'revenue' ? -1 : 1))
  })

  /** Types offered by the type editors (base types, the table's, the rules'). */
  knownTypes = $derived.by(() => {
    const set = new Set(BASE_TYPES)
    for (const r of this.rows) if (r.docType) set.add(r.docType)
    for (const o of Object.values(this.overrides)) if (o.docType) set.add(o.docType)
    for (const p of this.profiles) if (p.docType) set.add(p.docType)
    return [...set]
  })

  activeRow = $derived.by(() => this.rows.find((r) => r.path === this.activePath) ?? null)

  /** Rows a region rule can fix: text found but no value ("Testar nas com erro"). */
  fixableRows = $derived.by(() =>
    this.rows.filter((r) => r.extractionStatus === 'notFound' && r.cents == null && !r.duplicateOf),
  )

  /** Rows of "Testar em todas": every file once (a duplicate would give the same answer). */
  testableRows = $derived.by(() => this.rows.filter((r) => !r.duplicateOf))

  removedCount = $derived.by(
    () =>
      (this.scan?.files ?? []).filter(
        (f) => this.overrides[fileKey(f)]?.removed || (f.duplicateOf && this.hiddenCopies.has(f.path)),
      ).length,
  )

  busy = $derived(this.phase !== 'idle')

  // ------------------------------------------------------------ boot

  /** The backend's information; reports the profile files left out. */
  setInfo(info: AppInfo) {
    this.info = info
    const problems = info.profileErrors ?? []
    if (problems.length) {
      const what = plural(problems.length, 'perfil não pôde ser lido', 'perfis não puderam ser lidos')
      this.toast(`${what} e ${problems.length === 1 ? 'ficou' : 'ficaram'} de fora das regras: ${problems.join('; ')}`, 'error')
    }
  }

  /**
   * Starts the app. `loadData` reads the documents of the data folder (the companies, then the
   * notes and the planning of the active one) before anything is shown.
   */
  async init(loadData: () => Promise<void> = () => this.loadNotes()) {
    this.applyTheme()
    let info: AppInfo
    try {
      info = await api.appInfo()
    } catch (e) {
      this.bootError = errorMessage(e)
      this.boot = 'failed'
      return
    }
    this.setInfo(info)
    await loadData()
    this.boot = 'ready'
    if (!info.pdfiumOk) return

    const safely = async (what: () => Promise<unknown>) => {
      try {
        await what()
      } catch (e) {
        console.warn(errorMessage(e))
      }
    }
    await safely(() =>
      api.onExtractProgress((p) => {
        if (this.phase !== 'extracting') return
        this.progress = { done: Math.min(this.progress.total, this.#chunkBase + p.done), total: this.progress.total, run: 0 }
      }),
    )
    await safely(() =>
      api.onTestProgress((p) => {
        const test = this.ruleTest
        if (test?.running && p.run === test.seq) test.progress = { done: p.done, total: p.total, run: p.run }
      }),
    )
    await safely(() => api.onDragDrop((s) => this.#onDragDrop(s)))
    void this.loadProfiles()
    if (this.sources.length) this.refresh()
  }

  // ------------------------------------------------------------ notes data (store document)

  /**
   * Reads the notes document of the active company (sources, edits...). With `legacy`, the
   * localStorage keys of the first versions are migrated once (removed only after the write).
   */
  async loadNotes(legacy = true) {
    const doc = parseNotes(await this.notesDoc.read())
    const old = doc || !legacy ? null : legacyNotes()
    this.#applyNotes(doc ?? old ?? defaultNotes())
    this.notesDoc.markLoaded()
    if (old && !this.notesDoc.readBlocked && (await this.notesDoc.saveNow())) {
      for (const key of LEGACY_NOTE_KEYS) removeKey(key)
    }
  }

  /** Another company: nothing of the previous one stays on screen (UI preferences are kept). */
  resetView() {
    // "Desfazer" acts on whatever company is open when it is clicked, so it dies with this one.
    this.toasts = this.toasts.filter((t) => !t.action)
    this.clearSelection()
    this.closePreview()
    this.ruleTest = null
    this.editingPath = null
    this.editingCompetencePath = null
    this.competenceTodo = null
    this.filter = 'all'
    this.kindFilter = 'all'
    this.search = ''
    this.scan = null
    this.results = {}
  }

  #applyNotes(notes: NotesDoc) {
    this.sources = notes.sources
    this.exclude = notes.exclude
    this.overrides = notes.overrides
    this.includeDuplicates = notes.includeDuplicates
    this.period = notes.period
    this.hiddenCopies.clear()
    for (const path of notes.hiddenCopies) this.hiddenCopies.add(path)
  }

  #notesSnapshot(): NotesDoc {
    return {
      version: 1,
      sources: $state.snapshot(this.sources),
      exclude: $state.snapshot(this.exclude),
      overrides: $state.snapshot(this.overrides),
      hiddenCopies: [...this.hiddenCopies],
      includeDuplicates: this.includeDuplicates,
      period: $state.snapshot(this.period),
    }
  }

  // ------------------------------------------------------------ sources & filters

  async pickFolders() {
    try {
      const paths = await api.pickFolders()
      if (paths.length) this.addSources(paths)
    } catch (e) {
      this.toast(errorMessage(e), 'error')
    }
  }

  async pickFiles() {
    try {
      const paths = await api.pickFiles()
      if (paths.length) this.addSources(paths)
    } catch (e) {
      this.toast(errorMessage(e), 'error')
    }
  }

  addSources(paths: string[]) {
    const known = new Set(this.sources.map((s) => pathKey(s.path)))
    const added: Source[] = []
    for (const path of paths) {
      const key = pathKey(path)
      if (!key || known.has(key)) continue
      known.add(key)
      added.push({ path, recursive: false })
    }
    if (!added.length) {
      if (paths.length) this.toast(paths.length === 1 ? 'Essa fonte já está na lista.' : 'Essas fontes já estão na lista.')
      return
    }
    this.sources = [...this.sources, ...added]
    this.#sourcesChanged()
  }

  removeSource(path: string) {
    this.sources = this.sources.filter((s) => s.path !== path)
    this.#sourcesChanged()
  }

  setRecursive(path: string, recursive: boolean) {
    this.sources = this.sources.map((s) => (s.path === path ? { ...s, recursive } : s))
    this.#sourcesChanged()
  }

  addExclude(term: string): boolean {
    const t = term.trim()
    if (!t) return false
    if (this.exclude.some((x) => normalizeText(x) === normalizeText(t))) return false
    this.exclude = [...this.exclude, t]
    this.#sourcesChanged()
    return true
  }

  removeExclude(term: string) {
    this.exclude = this.exclude.filter((x) => x !== term)
    this.#sourcesChanged()
  }

  /** Info returned by the scan for a source (matched by path, then by position). */
  sourceInfo(source: Source): SourceInfo | null {
    const infos = this.scan?.sources ?? []
    const key = pathKey(source.path)
    const byPath = infos.find((i) => pathKey(i.path) === key)
    if (byPath) return byPath
    const index = this.sources.findIndex((s) => s.path === source.path)
    return infos.length === this.sources.length && index >= 0 ? infos[index] : null
  }

  #sourcesChanged() {
    this.notesDoc.changed()
    this.refresh()
  }

  #onDragDrop(state: api.DragDropState) {
    if (this.locked) return
    if (state.type === 'over') this.dragging = true
    else if (state.type === 'leave') this.dragging = false
    else {
      this.dragging = false
      if (state.paths.length) this.addSources(state.paths)
    }
  }

  // ------------------------------------------------------------ scan + extraction pipeline

  /** Schedules a scan + extraction; runs are serialized and coalesced. */
  refresh() {
    this.#dirty = true
    if (!this.#running) void this.#runLoop()
  }

  /**
   * Forgets the results kept in memory (e.g. the profiles changed). The backend keeps its own,
   * valid only for the profiles they were read with, so the next scan reads what changed.
   */
  forgetCache() {
    this.#cache.clear()
    this.#generation++
  }

  /** Forgets cached results (e.g. after profiles changed) and reads everything again. */
  reextract() {
    this.forgetCache()
    this.refresh()
  }

  async #runLoop() {
    this.#running = true
    try {
      while (this.#dirty) {
        this.#dirty = false
        try {
          await this.#runOnce()
        } catch (e) {
          this.toast(`Não foi possível ler as fontes: ${errorMessage(e)}`, 'error')
        }
      }
    } finally {
      this.#running = false
      this.phase = 'idle'
    }
  }

  async #runOnce() {
    const generation = this.#generation
    const sources = $state.snapshot(this.sources)
    if (!sources.length) {
      this.scan = null
      this.results = {}
      this.#prune()
      return
    }
    this.phase = 'scanning'
    const scan = await api.scanSources({ sources, exclude: $state.snapshot(this.exclude) })
    if (this.#dirty || generation !== this.#generation) return

    // Results of earlier runs (same profiles) come from the backend instead of being read again.
    const unknown = [...new Set(scan.files.map(cacheKey).filter((k) => k && !this.#cache.has(k)))]
    if (unknown.length) {
      const known = await api.cachedResults(unknown).catch((): Record<string, DocResult> => ({}))
      if (this.#dirty || generation !== this.#generation) return
      for (const [hash, result] of Object.entries(known)) this.#cache.set(hash, result)
    }

    const previousHash = new Map((this.scan?.files ?? []).map((f) => [f.path, f.hash]))
    const results: Record<string, DocResult> = {}
    const todo: string[] = []
    for (const f of scan.files) {
      const key = cacheKey(f)
      const cached = key ? this.#cache.get(key) : undefined
      if (cached) {
        results[f.path] = cached.path === f.path ? cached : { ...cached, path: f.path }
        continue
      }
      if (!f.duplicateOf) todo.push(f.path)
      // Keep showing the previous result while the file is read again (no flicker).
      const old = this.results[f.path]
      if (old && previousHash.get(f.path) === f.hash) results[f.path] = old
    }
    fillDuplicates(scan.files, results)
    this.scan = scan
    this.results = results
    this.#prune()
    if (!todo.length) return

    this.phase = 'extracting'
    this.progress = { done: 0, total: todo.length, run: 0 }
    const fileByPath = new Map(scan.files.map((f) => [f.path, f]))
    type Pending = { start: number; chunk: string[]; docs: Promise<DocResult[]> }
    const request = (start: number): Pending => {
      const chunk = todo.slice(start, start + CHUNK_SIZE)
      const keys = chunk.map((path) => {
        const f = fileByPath.get(path)
        return f ? cacheKey(f) : ''
      })
      this.#chunkBase = start
      return { start, chunk, docs: api.extractDocuments(chunk, keys) }
    }
    let working = { ...this.results }
    let lastPublish = performance.now()
    const publish = () => {
      fillDuplicates(scan.files, working)
      this.results = working
      working = { ...working }
      lastPublish = performance.now()
    }
    let inflight: Pending | null = request(0)
    while (inflight) {
      const { start, chunk }: Pending = inflight
      let docs: DocResult[]
      let failed = false
      try {
        docs = await inflight.docs
      } catch (e) {
        failed = true
        const message = errorMessage(e)
        docs = chunk.map((path) => ({ path, status: 'error', message, docType: 'PDF', kind: 'revenue', fields: {}, pageCount: 0 }))
        this.toast(`Falha ao ler ${plural(chunk.length, 'arquivo', 'arquivos')}: ${message}`, 'error')
      }
      // The profiles changed while this chunk was read: its results are stale, and the run that
      // forgetCache() scheduled (if any) reads these files again.
      if (generation !== this.#generation) return
      // The backend reads the next chunk while the table is updated.
      const next: number = start + CHUNK_SIZE
      inflight = next < todo.length && !this.#dirty ? request(next) : null
      docs.forEach((doc, k) => {
        // Trust the order of the answer; fall back to the returned path.
        const path = docs.length === chunk.length ? chunk[k] : doc.path
        const result = doc.path === path ? doc : { ...doc, path }
        working[path] = result
        const file = fileByPath.get(path)
        const key = file ? cacheKey(file) : ''
        if (key && !failed) this.#cache.set(key, result)
      })
      const done = Math.max(this.progress.done, Math.min(todo.length, start + chunk.length))
      this.progress = { done, total: todo.length, run: 0 }
      if (!inflight || performance.now() - lastPublish >= PUBLISH_MS) publish()
    }
    if (this.#dirty) return
    // Next time the app opens, these files are not read again.
    void api.saveCachedResults().catch(() => {})
  }

  /** Drops selection/preview state that points to files no longer listed. */
  #prune() {
    const paths = new Set((this.scan?.files ?? []).map((f) => f.path))
    for (const p of [...this.selected]) if (!paths.has(p)) this.selected.delete(p)
    if (this.activePath && !paths.has(this.activePath)) this.closePreview()
  }

  // ------------------------------------------------------------ rows

  #buildRows(): Row[] {
    const scan = this.scan
    if (!scan) return []
    const nameByPath = new Map(scan.files.map((f) => [f.path, f.name]))
    const nameKeys = this.#nameKeys
    const rows: Row[] = []
    for (const f of scan.files) {
      const nameKey = nameKeys.get(f.path) ?? normalizeText(f.name)
      const key = fileKey(f)
      const o = this.overrides[key]
      if (o?.removed || (f.duplicateOf && this.hiddenCopies.has(f.path))) continue
      const result = this.results[f.path] ?? null
      const extracted = result?.fields[NET_VALUE_FIELD] ?? null
      const valueEdited = o?.cents != null
      const cents = valueEdited ? o!.cents! : (extracted?.cents ?? null)

      let origin: Origin | null = null
      let highlight: Highlight | null = null
      if (valueEdited) {
        origin = o!.origin === 'region' ? { type: 'region' } : { type: 'manual' }
        if (o!.origin === 'region' && o!.bbox) highlight = { page: o!.page ?? 0, bbox: o!.bbox }
      } else if (extracted) {
        origin = extracted.origin
        highlight = { page: extracted.page, bbox: extracted.bbox }
      }

      const defaultType = defaultTypeFor(nameKey, result)
      const typeEdited = !!o?.docType && o.docType !== defaultType
      const docType = typeEdited ? o!.docType! : defaultType
      const cancelled = isCancelledType(docType)

      const defaultKind: DocKind = result?.kind === 'expense' ? 'expense' : 'revenue'
      const kindEdited = !!o?.kind && o.kind !== defaultKind
      const kind: DocKind = kindEdited ? o!.kind! : defaultKind

      const extractedCompetence = monthOfDate(result?.fields[COMPETENCE_FIELD]?.date)
      const competenceEdited = !!o?.competence && o.competence !== extractedCompetence
      const competence = competenceEdited ? o!.competence! : extractedCompetence

      let status: RowStatus
      if (f.duplicateOf) status = 'duplicate'
      else if (!result) status = 'pending'
      else if (valueEdited) status = 'ok'
      else if (result.status === 'ok' && cents == null) status = 'notFound'
      else status = result.status

      rows.push({
        path: f.path,
        name: f.name,
        dir: f.dir,
        key,
        searchKey: nameKey,
        duplicateOf: f.duplicateOf,
        duplicateName: f.duplicateOf ? (nameByPath.get(f.duplicateOf) ?? baseName(f.duplicateOf)) : null,
        result,
        extractionStatus: result?.status ?? null,
        status,
        cents,
        origin,
        highlight,
        defaultType,
        docType,
        kind,
        defaultKind,
        serviceCents: result?.fields[SERVICE_VALUE_FIELD]?.cents ?? null,
        competence,
        extractedCompetence,
        valueEdited,
        typeEdited,
        kindEdited,
        competenceEdited,
        edited: valueEdited || typeEdited || kindEdited || competenceEdited,
        attention: !f.duplicateOf && !!result && cents == null,
        cancelled,
        counted: cents != null && (!f.duplicateOf || this.includeDuplicates) && !cancelled,
        message: result?.message ?? null,
        pageCount: result?.pageCount ?? 0,
      })
    }
    return rows
  }

  #matchesFilter(row: Row, filter: Filter): boolean {
    switch (filter) {
      case 'all':
        return true
      case 'ok':
        return row.status === 'ok'
      case 'attention':
        return row.attention
      case 'edited':
        return row.edited
      case 'noCompetence':
        return this.competenceTodo?.has(row.path) ?? false
    }
  }

  // ------------------------------------------------------------ overrides (edits)

  #patchOverride(hash: string, patch: Partial<Override>, remove: (keyof Override)[] = [], persist = true) {
    const next: Override = { ...(this.overrides[hash] ?? {}), ...patch }
    for (const key of remove) delete next[key]
    if (Object.keys(next).length) this.overrides[hash] = next
    else delete this.overrides[hash]
    if (persist) this.#saveOverrides()
  }

  #saveOverrides() {
    this.notesDoc.changed()
  }

  /** Sets a manual value; `null` reverts to the extracted value. */
  setValue(row: Row, cents: number | null) {
    const extracted = row.result?.fields[NET_VALUE_FIELD]?.cents ?? null
    if (cents == null || (cents === extracted && extracted != null)) {
      this.#patchOverride(row.key, {}, ['cents', 'origin', 'raw', 'page', 'bbox'])
    } else if (cents !== row.cents || !row.valueEdited) {
      this.#patchOverride(row.key, { cents, origin: 'manual' }, ['raw', 'page', 'bbox'])
    }
  }

  setType(row: Row, type: string) {
    const t = type.trim()
    if (!t || t === row.defaultType) this.#patchOverride(row.key, {}, ['docType'])
    else this.#patchOverride(row.key, { docType: t })
  }

  /** Sets the same type on several rows, with one undo for the whole batch. */
  setTypeMany(rows: Row[], type: string) {
    const t = type.trim()
    if (!t || !rows.length) return
    const previous = new Map<string, string | undefined>()
    for (const row of rows) {
      if (!previous.has(row.key)) previous.set(row.key, this.overrides[row.key]?.docType)
      if (t === row.defaultType) this.#patchOverride(row.key, {}, ['docType'], false)
      else this.#patchOverride(row.key, { docType: t }, [], false)
    }
    this.#saveOverrides()
    this.toast(`Tipo “${t}” definido em ${plural(rows.length, 'nota', 'notas')}.`, 'success', {
      label: 'Desfazer',
      run: () => {
        for (const [hash, docType] of previous) {
          if (docType) this.#patchOverride(hash, { docType }, [], false)
          else this.#patchOverride(hash, {}, ['docType'], false)
        }
        this.#saveOverrides()
      },
    })
  }

  /**
   * Marks rows as revenue or expense, with one undo for the whole batch. Rows whose profiles already
   * give that kind just drop the user's choice.
   */
  setKindMany(rows: Row[], kind: DocKind) {
    const list = rows.filter((r) => r.kind !== kind)
    if (!list.length) {
      this.toast(rows.length === 1 ? `Esta nota já é ${KIND_LABEL[kind].toLowerCase()}.` : `As notas já são ${kind === 'expense' ? 'despesas' : 'receitas'}.`)
      return
    }
    const previous = new Map<string, DocKind | undefined>()
    for (const row of list) {
      if (!previous.has(row.key)) previous.set(row.key, this.overrides[row.key]?.kind)
      if (kind === row.defaultKind) this.#patchOverride(row.key, {}, ['kind'], false)
      else this.#patchOverride(row.key, { kind }, [], false)
    }
    this.#saveOverrides()
    const what = kind === 'expense' ? 'despesa' : 'receita'
    const message =
      list.length === 1
        ? `“${list[0].name}” marcada como ${what}.`
        : `${plural(list.length, 'nota marcada', 'notas marcadas')} como ${what}.`
    this.toast(message, 'success', {
      label: 'Desfazer',
      run: () => {
        for (const [hash, before] of previous) {
          if (before) this.#patchOverride(hash, { kind: before }, [], false)
          else this.#patchOverride(hash, {}, ['kind'], false)
        }
        this.#saveOverrides()
      },
    })
  }

  /** Receita <-> Despesa for one row (undo in the toast). */
  toggleKind(row: Row) {
    this.setKindMany([row], row.kind === 'expense' ? 'revenue' : 'expense')
  }

  /** Sets the competence month (`yyyy-mm`); `null` reverts to the one read from the document. */
  setCompetence(row: Row, month: string | null) {
    if (month == null || month === row.extractedCompetence) this.#patchOverride(row.key, {}, ['competence'])
    else if (month !== row.competence || !row.competenceEdited) this.#patchOverride(row.key, { competence: month })
  }

  /**
   * Removes a row from the list. A regular file is removed by hash (the document leaves the list,
   * with any identical copy). A duplicate copy only hides that copy, so the original keeps counting.
   */
  removeRow(row: Row) {
    this.removeRows([row])
  }

  /** Removes several rows (same rules as `removeRow`), with one undo toast for the whole batch. */
  removeRows(rows: Row[]) {
    if (!rows.length) return
    const copies: string[] = []
    const hashes: string[] = []
    for (const row of rows) {
      if (row.duplicateOf) {
        if (!this.hiddenCopies.has(row.path)) {
          this.hiddenCopies.add(row.path)
          copies.push(row.path)
        }
      } else if (!this.overrides[row.key]?.removed) {
        this.#patchOverride(row.key, { removed: true }, [], false)
        hashes.push(row.key)
      }
      this.selected.delete(row.path)
      if (this.activePath === row.path) this.closePreview()
    }
    this.#saveOverrides()
    this.#saveHiddenCopies()
    const message =
      rows.length === 1
        ? `“${rows[0].name}” saiu da lista.`
        : `${plural(rows.length, 'nota saiu', 'notas saíram')} da lista.`
    this.toast(message, 'info', {
      label: 'Desfazer',
      run: () => {
        for (const path of copies) this.hiddenCopies.delete(path)
        for (const hash of hashes) this.#patchOverride(hash, {}, ['removed'], false)
        this.#saveOverrides()
        this.#saveHiddenCopies()
      },
    })
  }

  restoreRemoved() {
    for (const f of this.scan?.files ?? []) {
      const key = fileKey(f)
      if (this.overrides[key]?.removed) this.#patchOverride(key, {}, ['removed'], false)
      this.hiddenCopies.delete(f.path)
    }
    this.#saveOverrides()
    this.#saveHiddenCopies()
  }

  #saveHiddenCopies() {
    this.notesDoc.changed()
  }

  setIncludeDuplicates(value: boolean) {
    this.includeDuplicates = value
    this.notesDoc.changed()
  }

  // ------------------------------------------------------------ selection & preview

  toggleSelected(path: string) {
    if (this.selected.has(path)) this.selected.delete(path)
    else this.selected.add(path)
    this.#anchor = path
  }

  /**
   * Shift+click: sets every visible row between the last toggled row and `path` (inclusive, in the
   * current filter and sort) to `on`. Without an anchor in view it acts on `path` alone.
   */
  selectRange(path: string, on: boolean) {
    const rows = this.visibleRows
    const to = rows.findIndex((r) => r.path === path)
    if (to < 0) return
    const from = this.#anchor ? rows.findIndex((r) => r.path === this.#anchor) : -1
    const [a, b] = from < 0 ? [to, to] : from <= to ? [from, to] : [to, from]
    for (let i = a; i <= b; i++) {
      if (on) this.selected.add(rows[i].path)
      else this.selected.delete(rows[i].path)
    }
    this.#anchor = path
  }

  setVisibleSelected(on: boolean) {
    for (const r of this.visibleRows) {
      if (on) this.selected.add(r.path)
      else this.selected.delete(r.path)
    }
  }

  clearSelection() {
    this.selected.clear()
    this.#anchor = null
  }

  /** A plain click on a row: the start of the next Shift+click range. */
  setAnchor(path: string) {
    this.#anchor = path
  }

  // ------------------------------------------------------------ sorting, filters, pages

  /** Header click: ascending -> descending -> original order. */
  cycleSort(key: SortKey) {
    const current = this.sort
    const next: SortState | null =
      !current || current.key !== key ? { key, dir: 'asc' } : current.dir === 'asc' ? { key, dir: 'desc' } : null
    this.sort = next
    if (next) saveJSON('sort', next)
    else removeKey('sort')
  }

  setFilter(filter: BaseFilter) {
    this.filter = filter
    this.competenceTodo = null
  }

  /** Receitas / Despesas / Todas (combined with the other filters). */
  setKindFilter(kind: KindFilter) {
    this.kindFilter = kind
  }

  /** "Período" of the notes page (saved with the company's notes). */
  setPeriod(period: Period) {
    const p = parseNotesPeriod({ ...period, includeUndated: this.period.includeUndated })
    this.period = p
    this.notesDoc.changed()
  }

  /** "Incluir notas sem competência" while a period is on. */
  setIncludeUndated(include: boolean) {
    this.period = { ...this.period, includeUndated: include }
    this.notesDoc.changed()
  }

  setView(view: View) {
    this.view = view
  }

  /** From the tax page: shows the table with only these rows, to fill in their competence. */
  reviewCompetence(paths: string[]) {
    this.competenceTodo = new Set(paths)
    this.filter = 'noCompetence'
    this.kindFilter = 'all'
    this.search = ''
    this.view = 'notes'
  }

  openPreview(path: string) {
    if (this.activePath !== path) this.region = null
    this.activePath = path
  }

  closePreview() {
    this.activePath = null
    this.region = null
  }

  async openFile(row: Row) {
    try {
      await api.openPath(row.path)
    } catch (e) {
      this.toast(errorMessage(e), 'error')
    }
  }

  async revealFile(path: string) {
    try {
      await api.revealItemInDir(path)
    } catch (e) {
      this.toast(errorMessage(e), 'error')
    }
  }

  // ------------------------------------------------------------ region tool

  async readRegion(path: string, page: number, rect: Rect) {
    const seq = ++this.#seq
    this.region = { seq, path, page, rect, loading: true, read: null, error: null }
    try {
      const read = await api.readRegion(path, page, rect)
      if (this.region?.seq !== seq) return
      this.region.read = read
      this.region.loading = false
    } catch (e) {
      if (this.region?.seq !== seq) return
      this.region.error = errorMessage(e)
      this.region.loading = false
    }
  }

  clearRegion() {
    this.region = null
  }

  /** Uses the value read in the region for the file where it was drawn. */
  applyRegionHere() {
    const value = this.region?.read?.value
    const row = this.rows.find((r) => r.path === this.region?.path)
    if (!value || value.cents == null || !row) return
    this.#patchOverride(row.key, { cents: value.cents, origin: 'region', raw: value.raw, page: value.page, bbox: value.bbox })
    this.toast('Valor aplicado nesta nota.', 'success')
  }

  async startRuleTest(scope: 'attention' | 'all') {
    const region = this.region
    const read = region?.read
    if (!region || !read) return
    const rows = scope === 'attention' ? this.fixableRows : this.testableRows
    const paths = rows.map((r) => r.path)
    const seq = ++this.#seq
    const rule = $state.snapshot(read.suggestedRule) as Rule
    this.ruleTest = {
      seq,
      rule,
      scope,
      sourcePath: region.path,
      paths,
      running: true,
      progress: { done: 0, total: paths.length, run: seq },
      results: [],
      error: null,
      saving: false,
    }
    try {
      const results = await api.testRule(paths, rule, seq)
      if (this.ruleTest?.seq !== seq) return
      this.ruleTest.results = results
    } catch (e) {
      if (this.ruleTest?.seq !== seq) return
      this.ruleTest.error = errorMessage(e)
    } finally {
      if (this.ruleTest?.seq === seq) this.ruleTest.running = false
    }
  }

  /** "Cancelar": closes the dialog and stops a test still running in the backend. */
  closeRuleTest() {
    const test = this.ruleTest
    if (test?.running) void api.cancelTest(test.seq).catch(() => {})
    this.ruleTest = null
  }

  /**
   * Applies the tested values to the chosen files (origin "region") and optionally saves the rule
   * as a profile (fingerprint [] = any document; future extractions use it automatically).
   */
  async applyRuleTest(paths: string[], profileName: string | null): Promise<boolean> {
    const state = this.ruleTest
    if (!state) return false
    const chosen = new Set(paths)
    const rowByPath = new Map(this.rows.map((r) => [r.path, r]))
    let applied = 0
    for (const t of state.results) {
      if (!chosen.has(t.path) || !t.value || t.value.cents == null) continue
      const row = rowByPath.get(t.path)
      if (!row) continue
      const v = t.value
      this.#patchOverride(row.key, { cents: v.cents!, origin: 'region', raw: v.raw, page: v.page, bbox: v.bbox }, [], false)
      applied++
    }
    this.#saveOverrides()

    if (profileName) {
      state.saving = true
      state.error = null
      try {
        const saved = await api.saveProfile({
          id: '',
          name: profileName,
          builtin: false,
          fingerprint: [],
          namePatterns: [],
          docType: null,
          kind: null,
          fields: { [NET_VALUE_FIELD]: [$state.snapshot(state.rule) as Rule] },
        })
        this.toast(`Perfil “${saved.name}” salvo. As próximas leituras vão usá-lo.`, 'success')
        await this.loadProfiles()
        this.reextract()
      } catch (e) {
        state.saving = false
        state.error = `Não foi possível salvar o perfil: ${errorMessage(e)}`
        return false
      }
    }
    this.ruleTest = null
    if (applied) this.toast(`${plural(applied, 'valor aplicado', 'valores aplicados')}.`, 'success')
    return true
  }

  /** Profile name suggested from the file where the region was drawn. */
  suggestProfileName(path: string): string {
    const stem = collapseSpaces(
      fileStem(baseName(path))
        .replace(/[_\-.]+/g, ' ')
        .replace(/\b\d+\b/g, ' '),
    )
    const name = stem ? stem.charAt(0).toUpperCase() + stem.slice(1) : 'Meu layout'
    return `Layout ${name}`
  }

  // ------------------------------------------------------------ profiles

  async loadProfiles() {
    try {
      this.profiles = await api.listProfiles()
    } catch (e) {
      this.toast(`Não foi possível carregar os perfis: ${errorMessage(e)}`, 'error')
    }
  }

  async deleteProfile(profile: Profile) {
    try {
      await api.deleteProfile(profile.id)
      this.toast(`“${profile.name}” excluído.`, 'success')
      await this.loadProfiles()
      this.reextract()
    } catch (e) {
      this.toast(`Não foi possível excluir: ${errorMessage(e)}`, 'error')
    }
  }

  /** Opens "Perfis e regras" (with the rule editor when `seed` is given). */
  openRules(seed: RuleSeed | null = null) {
    this.ruleSeed = seed
    this.profilesOpen = true
  }

  /** "Criar regra a partir desta nota": a name pattern from the file name, its type and kind. */
  ruleFromRow(row: Row) {
    const generic = !row.docType || ['pdf', 'nfs-e'].includes(normalizeText(row.docType))
    this.openRules({
      name: suggestRuleName(row.name, row.docType),
      namePatterns: [suggestNamePattern(row.name)],
      fingerprint: [],
      docType: generic ? '' : row.docType,
      kind: row.kind === 'expense' ? 'expense' : null,
    })
  }

  /** Saves a profile or rule; the files are read again with it. Throws the backend message. */
  async saveRule(profile: Profile): Promise<Profile> {
    const saved = await api.saveProfile(profile)
    await this.loadProfiles()
    this.reextract()
    this.toast(`“${saved.name}” salva. As notas estão sendo lidas de novo.`, 'success')
    return saved
  }

  // ------------------------------------------------------------ header actions

  /** Exports the visible rows, in the current sort, in `format` (asks where to save first). */
  async exportTable(format: ExportFormat) {
    const rows = this.visibleRows
    if (!rows.length || this.busy) return
    const total = this.rows.length
    let path: string | null
    try {
      path = await api.pickExportPath(format, exportFileName(format, this.company?.slug ?? null))
    } catch (e) {
      this.toast(errorMessage(e), 'error')
      return
    }
    if (!path) return
    const request: ExportRequest = {
      format,
      path,
      rows: rows.map((r) => ({
        file: r.name,
        path: r.path,
        docType: r.docType,
        kind: r.kind,
        competence: r.competence,
        // Rows outside the totals (duplicates, cancelled notes) are exported without value, so the
        // exported total matches the app.
        cents: r.counted ? r.cents : null,
        status: STATUS_LABEL[r.status],
        origin: originLabel(r.origin),
      })),
      generatedAt: formatDateTime(new Date()),
      companyName: this.company?.name ?? null,
    }
    try {
      await api.exportTable(request)
      const noun = EXPORT_FORMATS[format].noun
      const target = path
      this.toast(
        rows.length < total
          ? `${noun} exportado com ${rows.length} de ${total} notas (filtro ativo).`
          : `${noun} exportado: ${baseName(path)}`,
        'success',
        isTauri ? { label: 'Mostrar na pasta', run: () => void this.revealFile(target) } : undefined,
      )
    } catch (e) {
      this.toast(`Não foi possível exportar: ${errorMessage(e)}`, 'error')
    }
  }

  /** Copies the total of the receitas (of the despesas while only they are shown). */
  async copyTotal() {
    const expenses = this.kindFilter === 'expense'
    const text = formatBRLPlain(expenses ? this.totals.expense : this.totals.revenue)
    const ok = await copyText(text)
    const what = expenses ? 'das despesas' : 'das receitas'
    this.toast(ok ? `Total ${what} copiado: ${text}` : 'Não foi possível copiar o total.', ok ? 'success' : 'error')
  }

  // ------------------------------------------------------------ theme & toasts

  setTheme(theme: ThemeChoice) {
    this.theme = theme
    saveJSON('theme', theme)
    this.applyTheme()
  }

  applyTheme() {
    const root = document.documentElement
    if (this.theme === 'system') delete root.dataset.theme
    else root.dataset.theme = this.theme
  }

  toast(message: string, kind: Toast['kind'] = 'info', action?: Toast['action']) {
    const id = ++this.#seq
    this.toasts = [...this.toasts.slice(-2), { id, kind, message, action }]
    setTimeout(() => this.dismissToast(id), action ? 6500 : kind === 'error' ? 6000 : 3500)
  }

  dismissToast(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id)
  }
}

/** Duplicates are not extracted: they reuse the result of the first file with the same hash. */
function fillDuplicates(files: ScannedFile[], results: Record<string, DocResult>) {
  for (const f of files) {
    if (!f.duplicateOf) continue
    const original = results[f.duplicateOf]
    if (original) results[f.path] = { ...original, path: f.path }
  }
}

export const store = new AppStore()

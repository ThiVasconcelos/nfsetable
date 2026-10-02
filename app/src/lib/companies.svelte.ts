// Companies ("Empresas"). Each company has its own notes document ("notes-<id>": sources, exclude
// terms, overrides, hidden copies, "somar duplicadas", period) and planning document
// ("planning-<id>"); the rules/profiles and the UI preferences are shared by all of them.
//
// Store layout: the "companies" document `{ version: 1, activeId, companies: [...] }` plus the
// per-company documents. The first run after the single-company versions creates "Minha empresa"
// ("principal") and moves the old "notes"/"planning" documents under it: the new documents are
// written first and the old ones are deleted only after every write succeeded.
//
// This module also starts the app (the companies must be read before the notes and the planning).
// With two or more companies, the app asks which one to open ("Escolha a empresa") before reading
// or scanning anything of them, unless the preference says to open the last one used.

import * as api from './api'
import { errorMessage } from './api'
import { slugify } from './format'
import { StoreDoc } from './persist.svelte'
import { isRecord, loadJSON, saveJSON } from './storage'
import { store } from './store.svelte'
import { tax } from './tax.svelte'

export interface Company {
  id: string
  name: string
  /** Optional, as typed (formatted 00.000.000/0000-00). */
  cnpj: string | null
  /** ISO date and time. */
  createdAt: string
}

export interface CompaniesDoc {
  version: 1
  activeId: string
  companies: Company[]
}

export type CompanyDialog = { mode: 'create' } | { mode: 'rename'; id: string } | { mode: 'delete'; id: string }

/** "Ao abrir o app": ask which company to open, or open the last one used. */
export type StartupChoice = 'ask' | 'last'
const isStartupChoice = (v: unknown): v is StartupChoice => v === 'ask' || v === 'last'

/** Id of the company created by the migration of the single-company versions. */
export const MIGRATED_ID = 'principal'
export const MIGRATED_NAME = 'Minha empresa'
export const COMPANY_NAME_MAX = 60
/** "planning-" + id must fit in the 40 characters of a store name. */
const ID_MAX = 30
const ID_RE = /^[a-z0-9](?:[a-z0-9-]{0,28}[a-z0-9])?$/

export const notesDocName = (id: string) => `notes-${id}`
export const planningDocName = (id: string) => `planning-${id}`


/** A stored companies document, keeping the well-formed companies (null when there is none). */
export function parseCompanies(raw: unknown): CompaniesDoc | null {
  if (!isRecord(raw) || typeof raw.version !== 'number' || !Array.isArray(raw.companies)) return null
  const seen = new Set<string>()
  const companies: Company[] = []
  for (const c of raw.companies) {
    if (!isRecord(c) || typeof c.id !== 'string' || !ID_RE.test(c.id) || seen.has(c.id)) continue
    seen.add(c.id)
    const name = typeof c.name === 'string' && c.name.trim() ? c.name.trim().slice(0, COMPANY_NAME_MAX) : 'Empresa'
    const cnpj = typeof c.cnpj === 'string' && c.cnpj.trim() ? c.cnpj.trim().slice(0, 18) : null
    const createdAt = typeof c.createdAt === 'string' ? c.createdAt : ''
    companies.push({ id: c.id, name, cnpj, createdAt })
  }
  if (!companies.length) return null
  const activeId = companies.some((c) => c.id === raw.activeId) ? (raw.activeId as string) : companies[0].id
  return { version: 1, activeId, companies }
}

// ---------------------------------------------------------------- CNPJ

/** Value of a CNPJ character for the check digits (digits and, since 2026, letters). */
const cnpjValue = (c: string) => c.charCodeAt(0) - 48

/** Anything but digits and uppercase letters (the mask and the separators of a typed CNPJ). */
const NON_CNPJ_RE = /[^0-9A-Z]/g
/** The letters and digits of a typed CNPJ, uppercase. */
const cnpjChars = (input: string) => input.toUpperCase().replace(NON_CNPJ_RE, '')

/**
 * Mask "00.000.000/0000-00" while typing: 12 letters or digits (the alphanumeric CNPJ of 2026)
 * and 2 check digits.
 */
export function maskCnpj(input: string): string {
  const raw = cnpjChars(input)
  let chars = ''
  for (const c of raw) {
    if (chars.length >= 14) break
    if (chars.length >= 12 && !/[0-9]/.test(c)) continue
    chars += c
  }
  const parts = [chars.slice(0, 2), chars.slice(2, 5), chars.slice(5, 8), chars.slice(8, 12), chars.slice(12, 14)]
  let out = parts[0]
  if (parts[1]) out += `.${parts[1]}`
  if (parts[2]) out += `.${parts[2]}`
  if (parts[3]) out += `/${parts[3]}`
  if (parts[4]) out += `-${parts[4]}`
  return out
}

/** "ok", "incomplete" or "invalid" (check digits), for a masked or plain CNPJ. */
export function checkCnpj(input: string): 'ok' | 'incomplete' | 'invalid' {
  const chars = cnpjChars(input)
  if (chars.length < 14) return 'incomplete'
  if (!/^[0-9A-Z]{12}[0-9]{2}$/.test(chars) || /^(.)\1+$/.test(chars)) return 'invalid'
  const digit = (base: string) => {
    let weight = 2
    let sum = 0
    for (let i = base.length - 1; i >= 0; i--) {
      sum += cnpjValue(base[i]) * weight
      weight = weight === 9 ? 2 : weight + 1
    }
    const r = sum % 11
    return r < 2 ? 0 : 11 - r
  }
  const d1 = digit(chars.slice(0, 12))
  const d2 = digit(chars.slice(0, 12) + d1)
  return chars.endsWith(`${d1}${d2}`) ? 'ok' : 'invalid'
}

// ---------------------------------------------------------------- store

class CompaniesStore {
  list = $state.raw<Company[]>([])
  activeId = $state('')
  /** Switching, creating or deleting: the menu waits. */
  busy = $state(false)
  dialog = $state<CompanyDialog | null>(null)
  /** "Escolha a empresa" is shown (at startup with 2+ companies, or from the header menu). */
  #picking = $state(false)
  /** UI preference (localStorage, shared by every company). */
  startup = $state<StartupChoice>(loadJSON('startup', 'ask', isStartupChoice))
  doc = new StoreDoc<CompaniesDoc>({
    name: 'companies',
    readWhat: 'a lista de empresas',
    saveWhat: 'a lista de empresas',
    snapshot: () => ({ version: 1, activeId: this.activeId, companies: $state.snapshot(this.list) as Company[] }),
    notify: (message, kind, action) => store.toast(message, kind, action),
  })

  active = $derived(this.list.find((c) => c.id === this.activeId) ?? null)

  get picking(): boolean {
    return this.#picking
  }

  set picking(value: boolean) {
    this.#picking = value
    store.locked = value
  }

  /**
   * Reads the companies (or migrates the single-company documents) and points the notes and the
   * planning at the active company. Does not read them: see `startApp` and `switchTo`.
   */
  async load() {
    const parsed = parseCompanies(await this.doc.read())
    if (parsed) {
      this.list = parsed.companies
      this.activeId = parsed.activeId
      this.doc.markLoaded()
    } else if (this.doc.readBlocked) {
      // The list cannot be read: work on the first company without saving over the file.
      this.list = [{ id: MIGRATED_ID, name: MIGRATED_NAME, cnpj: null, createdAt: '' }]
      this.activeId = MIGRATED_ID
      this.doc.markLoaded()
    } else {
      await this.#migrate()
    }
    this.#bind()
  }

  /** "Minha empresa" gets the documents of the single-company versions. */
  async #migrate() {
    this.list = [{ id: MIGRATED_ID, name: MIGRATED_NAME, cnpj: null, createdAt: new Date().toISOString() }]
    this.activeId = MIGRATED_ID
    this.doc.markLoaded()
    const moved: string[] = []
    for (const [from, to] of [
      ['notes', notesDocName(MIGRATED_ID)],
      ['planning', planningDocName(MIGRATED_ID)],
    ]) {
      try {
        const value = await api.readStore(from)
        if (value == null) continue
        // A document already moved by an earlier (interrupted) run is not replaced.
        if ((await api.readStore(to)) == null) await api.writeStore(to, value)
        moved.push(from)
      } catch (e) {
        // Unreadable or not written: the old document stays where it is.
        store.toast(`Não foi possível mover os dados antigos (${from}): ${errorMessage(e)}`, 'error')
      }
    }
    if (!(await this.doc.saveNow())) return
    for (const name of moved) {
      try {
        await api.deleteStore(name)
      } catch {
        // Harmless: the moved copy is the one in use.
      }
    }
  }

  /** Notes, planning and exports follow the active company. */
  #bind() {
    const company = this.active
    store.notesDoc.name = notesDocName(this.activeId)
    tax.doc.name = planningDocName(this.activeId)
    store.company = company ? { name: company.name, slug: slugify(company.name, 40) || company.id } : null
  }

  /** Reads the notes and the planning of the active company. */
  async loadActive() {
    const legacy = this.activeId === MIGRATED_ID
    await Promise.all([store.loadNotes(legacy), tax.load(legacy)])
  }

  setStartup(choice: StartupChoice) {
    this.startup = choice
    saveJSON('startup', choice)
  }

  /** At startup: ask which company to open (2+ companies and the preference says so). */
  get shouldAsk(): boolean {
    return this.list.length >= 2 && this.startup === 'ask'
  }

  /**
   * Opens the company chosen in "Escolha a empresa": the normal path (active company saved, its
   * notes and planning read, its sources scanned). The company already open just closes the picker.
   */
  async open(id: string) {
    if (this.busy || !this.list.some((c) => c.id === id)) return
    if (id === this.activeId && store.notesDoc.loaded) {
      this.picking = false
      return
    }
    this.busy = true
    try {
      await Promise.all([store.notesDoc.flush(), tax.flush()])
      this.activeId = id
      this.#bind()
      store.resetView()
      await this.loadActive()
      store.refresh()
      void this.doc.saveNow()
      this.picking = false
    } finally {
      this.busy = false
    }
  }

  /**
   * Back to "Escolha a empresa" after the data folder changed: the notes and the planning still in
   * memory belong to the old folder, so they are never saved into the new one and are read again
   * when a company is opened (even one with the same id).
   */
  pickAgain() {
    store.notesDoc.loaded = false
    tax.doc.loaded = false
    this.picking = true
  }

  /** "Ver todas as empresas": back to the picker (pending saves land first). */
  async showPicker() {
    await Promise.all([store.notesDoc.flush(), tax.flush()])
    this.picking = true
  }

  /** Opens another company: pending saves land first, then its documents are read and scanned. */
  async switchTo(id: string): Promise<boolean> {
    if (id === this.activeId || this.busy || !this.list.some((c) => c.id === id)) return false
    this.busy = true
    try {
      await Promise.all([store.notesDoc.flush(), tax.flush()])
      this.activeId = id
      this.#bind()
      store.resetView()
      await this.loadActive()
      store.refresh()
      void this.doc.saveNow()
      return true
    } finally {
      this.busy = false
    }
  }

  /** "Nova empresa": starts empty (no sources, default planning) and becomes the active one. */
  async create(name: string, cnpj: string | null): Promise<boolean> {
    const clean = name.trim().slice(0, COMPANY_NAME_MAX)
    if (!clean || this.busy) return false
    const id = await this.#newId(clean)
    this.list = [...this.list, { id, name: clean, cnpj: cnpj?.trim() || null, createdAt: new Date().toISOString() }]
    if (!(await this.doc.saveNow())) {
      this.list = this.list.filter((c) => c.id !== id)
      return false
    }
    if (this.picking) await this.open(id)
    else await this.switchTo(id)
    store.toast(`Empresa “${clean}” criada. Adicione as pastas com as notas dela.`, 'success')
    return true
  }

  async rename(id: string, name: string, cnpj: string | null): Promise<boolean> {
    const clean = name.trim().slice(0, COMPANY_NAME_MAX)
    if (!clean) return false
    const before = this.list
    this.list = this.list.map((c) => (c.id === id ? { ...c, name: clean, cnpj: cnpj?.trim() || null } : c))
    if (!(await this.doc.saveNow())) {
      this.list = before
      return false
    }
    this.#bind()
    return true
  }

  /**
   * Deletes a company and its documents (never the last one). The active company is left first;
   * the PDFs and the shared rules are not touched.
   */
  async remove(id: string): Promise<boolean> {
    const company = this.list.find((c) => c.id === id)
    if (!company || this.list.length <= 1 || this.busy) return false
    if (id === this.activeId) {
      const next = this.list.find((c) => c.id !== id)!
      await this.switchTo(next.id)
    }
    const before = this.list
    this.list = this.list.filter((c) => c.id !== id)
    if (!(await this.doc.saveNow())) {
      this.list = before
      return false
    }
    const results = await Promise.allSettled([api.deleteStore(notesDocName(id)), api.deleteStore(planningDocName(id))])
    const failed = results.find((r) => r.status === 'rejected') as PromiseRejectedResult | undefined
    if (failed) {
      store.toast(`“${company.name}” saiu da lista, mas parte dos dados não foi apagada: ${errorMessage(failed.reason)}`, 'error')
    } else {
      store.toast(`Empresa “${company.name}” excluída.`, 'success')
    }
    return true
  }

  /**
   * A new id from the name, unique in the list and not used by documents left behind (a company
   * whose documents could not be deleted never gets them back).
   */
  async #newId(name: string): Promise<string> {
    const base = slugify(name, ID_MAX) || 'empresa'
    for (let n = 1; n < 100; n++) {
      const suffix = n === 1 ? '' : `-${n}`
      const id = `${slugify(base, ID_MAX - suffix.length)}${suffix}`
      if (!ID_RE.test(id) || this.list.some((c) => c.id === id)) continue
      try {
        const [notes, planning] = await Promise.all([api.readStore(notesDocName(id)), api.readStore(planningDocName(id))])
        if (notes == null && planning == null) return id
      } catch {
        // Unreadable leftovers: try the next id.
      }
    }
    return `empresa-${Date.now().toString(36)}`
  }
}

export const companies = new CompaniesStore()

/**
 * Starts the app: app info, the companies, then the notes and the planning of the active one.
 * With 2+ companies (and "Perguntar a empresa"), it stops at the picker: nothing of any company is
 * read or scanned before the choice.
 */
export function startApp(): Promise<void> {
  return store.init(async () => {
    await companies.load()
    if (companies.shouldAsk) {
      companies.picking = true
      return
    }
    await companies.loadActive()
  })
}

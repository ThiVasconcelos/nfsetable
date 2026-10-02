// One document of the backend store (`read_store` / `write_store`), saved with a short delay after
// each change. Used for the notes ("notes": sources, edits...) and the tax planning ("planning").
//
// - Writes are serialized; a queued write supersedes the older ones (only the latest state lands).
// - A document that exists but cannot be read (e.g. a corrupt file) pauses saving, so the file is
//   never replaced behind the user's back; "Salvar por cima" (toast, or the "Não salvo" indicator)
//   resumes it with what is on screen.
// - Save errors are reported once per distinct message (a missing command would fail on every
//   change).

import * as api from './api'
import { errorMessage } from './api'
import { withoutFinalPeriod } from './format'

export type SaveState = 'idle' | 'pending' | 'saved' | 'error'

type Notify = (message: string, kind: 'info' | 'success' | 'error', action?: { label: string; run: () => void }) => void

export interface StoreDocOptions<T> {
  /** Store name (`[a-z0-9-]{1,40}`). */
  name: string
  /** Completes "Não foi possível ler …", e.g. "o planejamento salvo". */
  readWhat: string
  /** Completes "Não foi possível salvar …", e.g. "o planejamento". */
  saveWhat: string
  /** The current value to write. */
  snapshot: () => T
  notify: Notify
  delayMs?: number
}

const DEFAULT_DELAY_MS = 400

export class StoreDoc<T> {
  /** Store name of the document; per-company documents change it when the company changes. */
  name: string
  saveState = $state<SaveState>('idle')
  saveError = $state<string | null>(null)
  /** The saved document exists but could not be read: saving is paused. */
  readBlocked = $state(false)
  /** The document was read and applied (changes are only saved after that). */
  loaded = $state(false)

  #options: StoreDocOptions<T>
  #timer: ReturnType<typeof setTimeout> | undefined
  #seq = 0
  #writes: Promise<unknown> = Promise.resolve()
  #toastedError: string | null = null

  constructor(options: StoreDocOptions<T>) {
    this.#options = options
    this.name = options.name
  }

  /**
   * Reads the document: its JSON value, or null when it was never written. A read error pauses
   * saving, is reported with the way out ("Salvar por cima") and also returns null. Call
   * `markLoaded()` once the value is applied.
   */
  async read(): Promise<unknown> {
    clearTimeout(this.#timer)
    this.loaded = false
    this.readBlocked = false
    this.saveState = 'idle'
    this.saveError = null
    this.#toastedError = null
    try {
      return await api.readStore(this.name)
    } catch (e) {
      this.readBlocked = true
      this.saveState = 'error'
      this.saveError =
        `Não foi possível ler ${this.#options.readWhat}: ${withoutFinalPeriod(errorMessage(e))}. ` +
        'Para não apagar o arquivo, as mudanças não estão sendo salvas.'
      this.offerOverwrite()
      return null
    }
  }

  /** From now on, changes are saved. */
  markLoaded() {
    this.loaded = true
  }

  /** Something changed: saves after a short delay (nothing before the document was read). */
  changed() {
    if (!this.loaded || this.readBlocked) return
    clearTimeout(this.#timer)
    this.saveState = 'pending'
    this.#timer = setTimeout(() => void this.saveNow(), this.#options.delayMs ?? DEFAULT_DELAY_MS)
  }

  /** Writes the document now. Resolves to false when it could not be saved. */
  saveNow(): Promise<boolean> {
    clearTimeout(this.#timer)
    if (this.readBlocked) return Promise.resolve(false)
    const seq = ++this.#seq
    this.saveState = 'pending'
    // The target is fixed now: a later change of document (another company) does not redirect it.
    const name = this.name
    const value = this.#options.snapshot()
    const write = async (): Promise<boolean> => {
      if (seq !== this.#seq) return true // a newer write is queued and will save this state
      try {
        await api.writeStore(name, value)
        if (seq === this.#seq) {
          this.saveState = 'saved'
          this.saveError = null
        }
        this.#toastedError = null
        return true
      } catch (e) {
        const message = errorMessage(e)
        if (seq === this.#seq) {
          this.saveState = 'error'
          this.saveError = message
        }
        if (message !== this.#toastedError) {
          this.#options.notify(`Não foi possível salvar ${this.#options.saveWhat}: ${message}`, 'error')
        }
        this.#toastedError = message
        return false
      }
    }
    const result = this.#writes.then(write)
    this.#writes = result.catch(() => false)
    return result
  }

  /** Saves right away if a save is waiting; resolves when every write is done. */
  async flush(): Promise<void> {
    if (this.loaded && this.saveState === 'pending') await this.saveNow()
    else await this.#writes
  }

  /** Error toast with the way out when the saved document could not be read. */
  offerOverwrite() {
    this.#options.notify(this.saveError ?? `Não foi possível ler ${this.#options.readWhat}.`, 'error', {
      label: 'Salvar por cima',
      run: () => {
        this.readBlocked = false
        void this.saveNow()
      },
    })
  }

  /** "Não salvo" clicked: a failed write is tried again; an unreadable file offers the overwrite. */
  retry() {
    if (this.readBlocked) this.offerOverwrite()
    else void this.saveNow()
  }
}

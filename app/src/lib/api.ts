// Typed access to the Tauri backend. Every command of app/src-tauri is wrapped here.
//
// Inside the desktop app (window.__TAURI_INTERNALS__ exists) calls go through Tauri `invoke`/`listen`
// and the dialog/opener plugins. In a normal browser (`npm run dev` / screenshots) everything falls
// back to ./mock.ts, which is loaded lazily so it never ships in the code path of the real app.
//
// Rust commands are snake_case; their argument names are passed in camelCase (Tauri's default).

import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWebview } from '@tauri-apps/api/webview'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { open, save } from '@tauri-apps/plugin-dialog'
import { openPath as openerOpenPath, revealItemInDir as openerReveal } from '@tauri-apps/plugin-opener'
import { EXPORT_FORMATS } from './labels'
import type {
  AppInfo,
  DocResult,
  ExportFormat,
  ExportRequest,
  Profile,
  Progress,
  Rect,
  RegionRead,
  RenderedPage,
  Rule,
  RuleTest,
  ScanOptions,
  ScanResult,
  TaxCatalog,
  TaxInput,
  TaxReport,
} from './types'

/** Stops listening to an event. */
export type Unlisten = () => void

/** Simplified drag & drop state for the UI (files/folders dragged from the OS). */
export type DragDropState = { type: 'over' } | { type: 'leave' } | { type: 'drop'; paths: string[] }

/** Everything the UI needs from the backend. Implemented by Tauri below and by ./mock.ts. */
export interface Backend {
  appInfo(): Promise<AppInfo>
  setDataDir(path: string | null, copy: boolean): Promise<AppInfo>
  scanSources(options: ScanOptions): Promise<ScanResult>
  cachedResults(keys: string[]): Promise<Record<string, DocResult>>
  extractDocuments(paths: string[], keys: string[]): Promise<DocResult[]>
  saveCachedResults(): Promise<void>
  renderPage(path: string, page: number, width: number): Promise<RenderedPage>
  readRegion(path: string, page: number, rect: Rect): Promise<RegionRead>
  testRule(paths: string[], rule: Rule, run: number): Promise<RuleTest[]>
  cancelTest(run: number): Promise<void>
  listProfiles(): Promise<Profile[]>
  saveProfile(profile: Profile): Promise<Profile>
  deleteProfile(id: string): Promise<void>
  exportTable(request: ExportRequest): Promise<void>
  taxReport(input: TaxInput): Promise<TaxReport>
  taxCatalog(): Promise<TaxCatalog>
  readStore(name: string): Promise<unknown>
  writeStore(name: string, value: unknown): Promise<void>
  deleteStore(name: string): Promise<void>
  onExtractProgress(handler: (progress: Progress) => void): Promise<Unlisten>
  onTestProgress(handler: (progress: Progress) => void): Promise<Unlisten>
  pickFolders(): Promise<string[]>
  pickFiles(): Promise<string[]>
  pickDataDir(): Promise<string | null>
  pickExportPath(format: ExportFormat, defaultName: string): Promise<string | null>
  openPath(path: string): Promise<void>
  revealItemInDir(path: string): Promise<void>
  onDragDrop(handler: (state: DragDropState) => void): Promise<Unlisten>
  onCloseRequested(handler: () => Promise<void>): Promise<Unlisten>
}

/** True inside the Tauri webview. */
export const isTauri: boolean = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

/** True when running on the synthetic mock backend (normal browser). */
export const isMock = !isTauri

function toList(value: string | string[] | null): string[] {
  if (value == null) return []
  return Array.isArray(value) ? value : [value]
}

const tauriBackend: Backend = {
  appInfo: () => invoke<AppInfo>('app_info'),
  setDataDir: (path, copy) => invoke<AppInfo>('set_data_dir', { path, copy }),
  scanSources: (options) => invoke<ScanResult>('scan_sources', { options }),
  cachedResults: (keys) => invoke<Record<string, DocResult>>('cached_results', { keys }),
  extractDocuments: (paths, keys) => invoke<DocResult[]>('extract_documents', { paths, keys }),
  saveCachedResults: () => invoke<void>('save_cached_results'),
  renderPage: (path, page, width) => invoke<RenderedPage>('render_page', { path, page, width }),
  readRegion: (path, page, rect) => invoke<RegionRead>('read_region', { path, page, rect }),
  testRule: (paths, rule, run) => invoke<RuleTest[]>('test_rule', { paths, rule, run }),
  cancelTest: (run) => invoke<void>('cancel_test', { run }),
  listProfiles: () => invoke<Profile[]>('list_profiles'),
  saveProfile: (profile) => invoke<Profile>('save_profile', { profile }),
  deleteProfile: (id) => invoke<void>('delete_profile', { id }),
  exportTable: (request) => invoke<void>('export_table', { request }),
  taxReport: (input) => invoke<TaxReport>('tax_report', { input }),
  taxCatalog: () => invoke<TaxCatalog>('tax_catalog'),
  readStore: (name) => invoke<unknown>('read_store', { name }),
  writeStore: (name, value) => invoke<void>('write_store', { name, value }),
  deleteStore: (name) => invoke<void>('delete_store', { name }),

  onExtractProgress: (handler) => listen<Progress>('extract-progress', (e) => handler(e.payload)),
  onTestProgress: (handler) => listen<Progress>('test-progress', (e) => handler(e.payload)),

  pickFolders: async () =>
    toList(await open({ directory: true, multiple: true, title: 'Escolha as pastas com as notas' })),
  pickFiles: async () =>
    toList(
      await open({
        multiple: true,
        title: 'Escolha os PDFs das notas',
        filters: [{ name: 'PDF', extensions: ['pdf'] }],
      }),
    ),
  pickDataDir: async () => {
    const chosen = await open({ directory: true, multiple: false, title: 'Escolha a pasta dos dados' })
    return Array.isArray(chosen) ? (chosen[0] ?? null) : chosen
  },
  pickExportPath: (format, defaultName) => {
    const info = EXPORT_FORMATS[format]
    return save({
      title: info.dialogTitle,
      defaultPath: defaultName,
      filters: [{ name: info.filterName, extensions: [info.extension] }],
    })
  },

  openPath: (path) => openerOpenPath(path),
  revealItemInDir: (path) => openerReveal(path),

  onDragDrop: (handler) =>
    getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload
      if (payload.type === 'drop') handler({ type: 'drop', paths: payload.paths })
      else if (payload.type === 'leave') handler({ type: 'leave' })
      else handler({ type: 'over' })
    }),

  // The window is destroyed once the handler resolves.
  onCloseRequested: (handler) => getCurrentWindow().onCloseRequested(() => handler()),
}

let mockBackend: Promise<Backend> | null = null

function backend(): Promise<Backend> {
  if (isTauri) return Promise.resolve(tauriBackend)
  mockBackend ??= import('./mock').then((m) => m.createMockBackend())
  return mockBackend
}

/** Turns anything thrown by invoke/plugins (usually a pt-BR string from Rust) into a message. */
export function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message: unknown }).message
    if (typeof message === 'string') return message
  }
  try {
    return JSON.stringify(error)
  } catch {
    return String(error)
  }
}

// ---------------------------------------------------------------- commands

/** `app_info()` */
export const appInfo = async (): Promise<AppInfo> => (await backend()).appInfo()

/**
 * `set_data_dir(path, copy)`: moves the persistent data (profiles and stored documents) to `path`
 * (null = back to the default folder). With `copy`, the current data is copied there first;
 * otherwise whatever is already there is used. Returns the new app info.
 */
export const setDataDir = async (path: string | null, copy: boolean): Promise<AppInfo> =>
  (await backend()).setDataDir(path, copy)

/** `scan_sources(options)` */
export const scanSources = async (options: ScanOptions): Promise<ScanResult> =>
  (await backend()).scanSources(options)

/** `cached_results(keys)`: results of earlier runs by cache key, valid for the current profiles. */
export const cachedResults = async (keys: string[]): Promise<Record<string, DocResult>> =>
  (await backend()).cachedResults(keys)

/**
 * `extract_documents(paths, keys)`: `keys[i]` is the cache key of `paths[i]` (kept for the next
 * runs; empty when the scan could not read the file); emits "extract-progress".
 */
export const extractDocuments = async (paths: string[], keys: string[]): Promise<DocResult[]> =>
  (await backend()).extractDocuments(paths, keys)

/** `save_cached_results()`: writes the results kept by `extract_documents` to disk. */
export const saveCachedResults = async (): Promise<void> => (await backend()).saveCachedResults()

/** `render_page(path, page, width)`; `width` is the target width in pixels. */
export const renderPage = async (path: string, page: number, width: number): Promise<RenderedPage> =>
  (await backend()).renderPage(path, page, width)

/** `read_region(path, page, rect)`; `rect` in absolute points, top-left origin. */
export const readRegion = async (path: string, page: number, rect: Rect): Promise<RegionRead> =>
  (await backend()).readRegion(path, page, rect)

/**
 * `test_rule(paths, rule, run)`: tests a rule as test number `run`; emits "test-progress" tagged
 * with it. Starting a test stops the previous one.
 */
export const testRule = async (paths: string[], rule: Rule, run: number): Promise<RuleTest[]> =>
  (await backend()).testRule(paths, rule, run)

/** `cancel_test(run)`: stops test `run` before its next file. */
export const cancelTest = async (run: number): Promise<void> => (await backend()).cancelTest(run)

/** `list_profiles()`: built-ins first, then user profiles by name. */
export const listProfiles = async (): Promise<Profile[]> => (await backend()).listProfiles()

/** `save_profile(profile)`: the backend assigns an id when empty and forces builtin=false. */
export const saveProfile = async (profile: Profile): Promise<Profile> =>
  (await backend()).saveProfile(profile)

/** `delete_profile(id)` */
export const deleteProfile = async (id: string): Promise<void> => (await backend()).deleteProfile(id)

/** `export_table(request)`: writes the table as CSV, Excel, PDF or SQL (`request.format`). */
export const exportTable = async (request: ExportRequest): Promise<void> =>
  (await backend()).exportTable(request)

/** `tax_report(input)`: MEI / Simples / Lucro Presumido estimates for a typical month. */
export const taxReport = async (input: TaxInput): Promise<TaxReport> => (await backend()).taxReport(input)

/** `tax_catalog()`: the CNAE codes known to the tax calculations and the suggested default. */
export const taxCatalog = async (): Promise<TaxCatalog> => (await backend()).taxCatalog()

/**
 * `read_store(name)`: a JSON value saved with `writeStore` (null when there is none). The backend
 * keeps it in `<data folder>/store/<name>.json`; names match `[a-z0-9-]{1,40}`. A corrupt file is
 * an error (never silently replaced).
 */
export const readStore = async (name: string): Promise<unknown> => (await backend()).readStore(name)

/** `write_store(name, value)`: saves a JSON value (up to 2 MB), atomically. */
export const writeStore = async (name: string, value: unknown): Promise<void> =>
  (await backend()).writeStore(name, value)

/** `delete_store(name)`: removes a saved document (deleting one that does not exist is fine). */
export const deleteStore = async (name: string): Promise<void> => (await backend()).deleteStore(name)

// ---------------------------------------------------------------- events

/** Listens to "extract-progress" { done, total }. */
export const onExtractProgress = async (handler: (progress: Progress) => void): Promise<Unlisten> =>
  (await backend()).onExtractProgress(handler)

/** Listens to "test-progress" { done, total }. */
export const onTestProgress = async (handler: (progress: Progress) => void): Promise<Unlisten> =>
  (await backend()).onTestProgress(handler)

/** Files/folders dragged from the OS onto the window (Tauri drag & drop). */
export const onDragDrop = async (handler: (state: DragDropState) => void): Promise<Unlisten> =>
  (await backend()).onDragDrop(handler)

/** The window is about to close: it waits for `handler` (e.g. pending saves) first. */
export const onCloseRequested = async (handler: () => Promise<void>): Promise<Unlisten> =>
  (await backend()).onCloseRequested(handler)

// ---------------------------------------------------------------- dialogs & opener

/** Folder picker (multiple). Returns [] when cancelled. */
export const pickFolders = async (): Promise<string[]> => (await backend()).pickFolders()

/** PDF file picker (multiple). Returns [] when cancelled. */
export const pickFiles = async (): Promise<string[]> => (await backend()).pickFiles()

/** Folder picker for the data folder (single). Null when cancelled. */
export const pickDataDir = async (): Promise<string | null> => (await backend()).pickDataDir()

/** Save dialog for an export (title, suggested file name and filter of the format). Null when cancelled. */
export const pickExportPath = async (format: ExportFormat, defaultName: string): Promise<string | null> =>
  (await backend()).pickExportPath(format, defaultName)

/** Opens a file with the system's default app (PDF viewer). */
export const openPath = async (path: string): Promise<void> => (await backend()).openPath(path)

/** Shows a file in the system file manager. */
export const revealItemInDir = async (path: string): Promise<void> =>
  (await backend()).revealItemInDir(path)

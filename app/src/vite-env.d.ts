/// <reference types="svelte" />
/// <reference types="vite/client" />

import type { MockControls } from './lib/mock'

declare global {
  interface Window {
    /** Injected by Tauri inside the desktop webview. */
    __TAURI_INTERNALS__?: unknown
    /** Dev/screenshot helpers, only present when running on the browser mock. */
    __mock?: MockControls
  }
}

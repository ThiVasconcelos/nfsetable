import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// Minimal typing for the Node global, so this file type-checks without @types/node.
declare const process: { env: Record<string, string | undefined> }

// Set by `tauri dev` when developing on a mobile device / remote host.
const host = process.env.TAURI_DEV_HOST

/**
 * JavaScript target per webview. WebView2 (Windows) is evergreen Chromium. WKWebView (macOS) can be
 * an old Safari on an old system, so it keeps a conservative target. WebKitGTK (Linux; Tauri 2
 * needs 4.1) and the browser build support ES2022, so class private fields (every $state field of
 * a store) are not compiled into WeakMap lookups.
 */
function buildTarget(platform: string | undefined): string {
  if (platform === 'windows') return 'chrome105'
  if (platform === 'darwin' || platform === 'macos') return 'es2021'
  return 'es2022'
}

// https://vite.dev/config/ — shape recommended by the Tauri 2 docs.
export default defineConfig({
  plugins: [svelte()],

  // Keep Rust errors visible in the terminal.
  clearScreen: false,

  server: {
    // Must match `build.devUrl` in src-tauri/tauri.conf.json.
    port: 5173,
    // Tauri expects a fixed port: fail instead of picking another one.
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 5174 } : undefined,
    watch: {
      // The Rust side is rebuilt by the Tauri CLI, not by Vite.
      ignored: ['**/src-tauri/**'],
    },
  },

  // Variables with these prefixes are exposed through `import.meta.env`.
  envPrefix: ['VITE_', 'TAURI_ENV_*'],

  build: {
    target: buildTarget(process.env.TAURI_ENV_PLATFORM),
    outDir: 'dist',
    emptyOutDir: true,
    // Readable output and source maps for debug builds of the desktop app.
    minify: process.env.TAURI_ENV_DEBUG ? false : true,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})

import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// Minimal typing for the Node global, so this file type-checks without @types/node.
declare const process: { env: Record<string, string | undefined> }

// Set by `tauri dev` when developing on a mobile device / remote host.
const host = process.env.TAURI_DEV_HOST

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
    // WebView2 (Windows) is evergreen Chromium; WebKitGTK/WKWebView get a conservative target.
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'es2021',
    outDir: 'dist',
    emptyOutDir: true,
    // Readable output and source maps for debug builds of the desktop app.
    minify: process.env.TAURI_ENV_DEBUG ? false : true,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
})

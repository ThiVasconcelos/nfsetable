<script module lang="ts">
  import type { RenderedPage } from '../lib/types'

  // Small in-memory cache of rendered pages (data URLs), most recent last, bounded by count and by
  // size (a page rendered at a high zoom is hundreds of kilobytes).
  const cache = new Map<string, RenderedPage>()
  const CACHE_LIMIT = 24
  const CACHE_CHARS = 6_000_000
  let cachedChars = 0

  function remember(key: string, page: RenderedPage) {
    forget(key)
    cache.set(key, page)
    cachedChars += page.dataUrl.length
    while (cache.size > 1 && (cache.size > CACHE_LIMIT || cachedChars > CACHE_CHARS)) {
      forget(cache.keys().next().value as string)
    }
  }

  function forget(key: string) {
    const old = cache.get(key)
    if (!old) return
    cache.delete(key)
    cachedChars -= old.dataUrl.length
  }

  // The preview is rebuilt for every row the keyboard passes over. A request that comes soon after
  // the previous one waits a moment, so only the row the user stops on is rendered; one that comes
  // after a pause (a click) is sent right away.
  const SETTLE_MS = 200
  const BURST_GAP_MS = 300
  let lastRequestAt = -Infinity
  /** Newest render request of any preview: older answers are dropped, not shown or cached. */
  let latest = 0
</script>

<script lang="ts">
  import { errorMessage, renderPage } from '../lib/api'
  import type { Rect } from '../lib/types'
  import Icon from './Icon.svelte'

  interface Props {
    path: string
    page: number
    zoom: number
    drawMode: boolean
    /** Box of the extracted value on this page (points), if any. */
    highlight: Rect | null
    /** Region drawn by the user on this page (points), if any. */
    drawn: Rect | null
    onrendered?: (page: RenderedPage) => void
    onregion: (rect: Rect) => void
  }

  let { path, page, zoom, drawMode, highlight, drawn, onrendered, onregion }: Props = $props()

  const PADDING = 16

  let viewport: HTMLDivElement | undefined = $state()
  let viewportWidth = $state(0)
  let viewportHeight = $state(0)
  let rendered = $state.raw<RenderedPage | null>(null)
  let error = $state<string | null>(null)
  let loading = $state(false)
  let highlightEl: HTMLDivElement | undefined = $state()
  let drawnEl: HTMLDivElement | undefined = $state()
  let layer: HTMLDivElement | undefined = $state()
  let drag = $state<{ x0: number; y0: number; x1: number; y1: number } | null>(null)

  const measured = $derived(viewportWidth > 0)
  const pageWidth = $derived(Math.max(120, (viewportWidth - PADDING * 2) * zoom))

  // Pixel width requested from the backend: bucketed so small resizes do not re-render.
  const targetWidth = $derived.by(() => {
    const dpr = typeof window === 'undefined' ? 1 : Math.min(window.devicePixelRatio || 1, 2.5)
    const px = pageWidth * dpr
    return Math.min(3072, Math.max(512, Math.ceil(px / 256) * 256))
  })

  // Depends on the bucketed width only: a few pixels of scrollbar do not render the page again.
  $effect(() => {
    if (!measured) return
    const [p, pg, width] = [path, page, targetWidth]
    const mine = ++latest
    const hit = cache.get(`${p}|${pg}|${width}`)
    if (hit) {
      loading = false
      show(hit)
      return
    }
    const now = performance.now()
    const delay = now - lastRequestAt < BURST_GAP_MS ? SETTLE_MS : 0
    lastRequestAt = now
    loading = true
    const timer = setTimeout(() => void load(p, pg, width, mine), delay)
    return () => clearTimeout(timer)
  })

  function show(page: RenderedPage) {
    rendered = page
    error = null
    onrendered?.(page)
  }

  async function load(p: string, pg: number, width: number, mine: number) {
    try {
      const result = await renderPage(p, pg, width)
      if (mine !== latest) return
      remember(`${p}|${pg}|${width}`, result)
      show(result)
    } catch (e) {
      if (mine !== latest) return
      rendered = null
      error = errorMessage(e)
    } finally {
      if (mine === latest) loading = false
    }
  }

  function pct(r: Rect, pad = 0) {
    if (!rendered) return ''
    const W = rendered.widthPt
    const H = rendered.heightPt
    return `left:${((r.x - pad) / W) * 100}%;top:${((r.y - pad) / H) * 100}%;width:${((r.w + pad * 2) / W) * 100}%;height:${((r.h + pad * 2) / H) * 100}%`
  }

  // Keep the highlighted value in view when the page, zoom or value changes...
  $effect(() => {
    void rendered
    void zoom
    void highlight
    void viewportHeight
    const el = highlightEl
    if (el) requestAnimationFrame(() => reveal(el))
  })

  // ...and the rectangle the user just drew.
  $effect(() => {
    void drawn
    void viewportHeight
    const el = drawnEl
    if (el) requestAnimationFrame(() => reveal(el))
  })

  /** Scrolls the viewport (only) so that `el` is visible, centered when it was hidden. */
  function reveal(el: HTMLElement) {
    if (!viewport || !el.isConnected) return
    const v = viewport.getBoundingClientRect()
    const h = el.getBoundingClientRect()
    const visible = h.top >= v.top + 8 && h.bottom <= v.bottom - 8 && h.left >= v.left && h.right <= v.right
    if (visible) return
    viewport.scrollTop += h.top + h.height / 2 - (v.top + v.height / 2)
    viewport.scrollLeft += h.left + h.width / 2 - (v.left + v.width / 2)
  }

  // ------------------------------------------------------------ drawing (screen px -> PDF points)

  function toPoints(event: PointerEvent) {
    const r = layer!.getBoundingClientRect()
    const x = Math.min(Math.max(event.clientX - r.left, 0), r.width)
    const y = Math.min(Math.max(event.clientY - r.top, 0), r.height)
    return { x: (x / r.width) * rendered!.widthPt, y: (y / r.height) * rendered!.heightPt }
  }

  function onPointerDown(event: PointerEvent) {
    if (!rendered || event.button !== 0) return
    event.preventDefault()
    const p = toPoints(event)
    drag = { x0: p.x, y0: p.y, x1: p.x, y1: p.y }
    layer?.setPointerCapture(event.pointerId)
  }

  function onPointerMove(event: PointerEvent) {
    if (!drag) return
    const p = toPoints(event)
    drag = { ...drag, x1: p.x, y1: p.y }
  }

  function onPointerUp(event: PointerEvent) {
    if (!drag) return
    const d = drag
    drag = null
    layer?.releasePointerCapture(event.pointerId)
    const rect = normalized(d)
    if (rect.w < 4 || rect.h < 3) return // a click, not a rectangle
    onregion(rect)
  }

  function normalized(d: { x0: number; y0: number; x1: number; y1: number }): Rect {
    const round = (n: number) => Math.round(n * 10) / 10
    return {
      x: round(Math.min(d.x0, d.x1)),
      y: round(Math.min(d.y0, d.y1)),
      w: round(Math.abs(d.x1 - d.x0)),
      h: round(Math.abs(d.y1 - d.y0)),
    }
  }
</script>

<div class="viewport" bind:this={viewport} bind:clientWidth={viewportWidth} bind:clientHeight={viewportHeight} class:zoomed={zoom > 1}>
  {#if error}
    <div class="state error">
      <Icon name="alertCircle" size={22} />
      <p>Não foi possível exibir a página.</p>
      <p class="detail">{error}</p>
    </div>
  {:else}
    <div
      class="page"
      class:loading={loading && !rendered}
      style="width: {pageWidth}px; aspect-ratio: {rendered ? `${rendered.widthPt} / ${rendered.heightPt}` : '595 / 842'}"
    >
      {#if rendered}
        <img src={rendered.dataUrl} alt="Página {page + 1} do PDF" draggable="false" />
        {#if highlight}
          <div bind:this={highlightEl} class="highlight" style={pct(highlight, 2)} title="Valor lido"></div>
        {/if}
        {#if drawn}
          <div bind:this={drawnEl} class="drawn" style={pct(drawn)}></div>
        {/if}
        {#if drawMode}
          <div
            bind:this={layer}
            class="draw-layer"
            role="application"
            aria-label="Arraste para desenhar um retângulo em volta do valor"
            onpointerdown={onPointerDown}
            onpointermove={onPointerMove}
            onpointerup={onPointerUp}
            onpointercancel={() => (drag = null)}
          >
            {#if drag}
              <div class="rubber" style={pct(normalized(drag))}></div>
            {/if}
          </div>
        {/if}
      {/if}
      {#if loading}
        <span class="busy"><span class="spinner"></span></span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .viewport {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 16px;
    background: var(--canvas-bg);
  }

  .page {
    position: relative;
    margin: 0 auto;
    background: #fff;
    box-shadow: var(--page-shadow);
    border-radius: 2px;
    user-select: none;
  }

  .page.loading {
    background: linear-gradient(90deg, #fafafa, #f0f0f2, #fafafa);
    background-size: 200% 100%;
    animation: shimmer 1.2s linear infinite;
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    border-radius: 2px;
  }

  .highlight {
    position: absolute;
    border: 2px solid var(--highlight-stroke);
    border-radius: 3px;
    background: var(--highlight-fill);
    box-shadow: 0 0 0 3px rgba(62, 99, 221, 0.18);
    pointer-events: none;
    animation: pulse 1.6s var(--ease) 1;
  }

  .drawn {
    position: absolute;
    border: 1.5px dashed #d9480f;
    background: rgba(217, 72, 15, 0.08);
    border-radius: 2px;
    pointer-events: none;
  }

  .draw-layer {
    position: absolute;
    inset: 0;
    cursor: crosshair;
    touch-action: none;
    background: rgba(62, 99, 221, 0.035);
    outline: 2px solid rgba(62, 99, 221, 0.35);
    outline-offset: -2px;
  }

  .rubber {
    position: absolute;
    border: 1.5px solid #d9480f;
    background: rgba(217, 72, 15, 0.1);
    border-radius: 2px;
  }

  .busy {
    position: absolute;
    top: 8px;
    right: 8px;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--surface);
    color: var(--accent-text);
    box-shadow: var(--page-shadow);
  }

  .state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-2);
    max-width: 320px;
    margin: var(--s-8) auto;
    text-align: center;
    color: var(--text-2);
    font-size: 13px;
  }

  .state.error :global(svg) {
    color: var(--danger);
  }

  .detail {
    color: var(--text-3);
    font-size: 12.5px;
  }

  @keyframes pulse {
    0% {
      box-shadow: 0 0 0 0 rgba(62, 99, 221, 0.45);
    }
    70% {
      box-shadow: 0 0 0 10px rgba(62, 99, 221, 0);
    }
    100% {
      box-shadow: 0 0 0 3px rgba(62, 99, 221, 0.18);
    }
  }

  @keyframes shimmer {
    from {
      background-position: 200% 0;
    }
    to {
      background-position: -200% 0;
    }
  }
</style>

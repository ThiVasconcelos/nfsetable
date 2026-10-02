<script lang="ts">
  // Revenue per month: one column per month (notes or projection). Months included are drawn in
  // the accent, the others in the de-emphasis grey; projected months ("previsto") are hatched and
  // lighter. The hairline is the average of the months considered. Each column is a toggle (click,
  // Enter or Space) with a tooltip on hover and focus; "Receitas e projeção" lists the same values.

  import { formatBRL, formatBRLCompact, formatMonthLong, formatMonthShort, plural } from '../lib/format'
  import type { PlanMonth } from '../lib/tax.svelte'

  interface Props {
    months: PlanMonth[]
    /** Average of the months considered (cents), for the reference line. */
    average: number | null
    ontoggle: (month: string) => void
  }

  let { months, average, ontoggle }: Props = $props()

  const HEIGHT = 148
  const PAD_TOP = 20
  const AXIS = 22
  const PLOT = HEIGHT - PAD_TOP - AXIS
  const BASE = PAD_TOP + PLOT
  /** Room at the right for the average label. */
  const GUTTER = 64
  const uid = $props.id()

  let width = $state(0)
  let hover = $state<number | null>(null)

  const max = $derived(Math.max(1, average ?? 0, ...months.map((m) => m.cents ?? 0)) * 1.08)
  const plotWidth = $derived(Math.max(0, width - GUTTER))
  const band = $derived(months.length ? plotWidth / months.length : 0)
  const barWidth = $derived(Math.max(6, Math.min(24, band * 0.58)))
  const multiYear = $derived(new Set(months.map((m) => m.month.slice(0, 4))).size > 1)
  /** Label every n-th month when they do not fit. */
  const labelEvery = $derived(Math.max(1, Math.ceil((multiYear ? 44 : 30) / Math.max(1, band))))
  const avgY = $derived(average ? y(average) : null)
  const hasForecast = $derived(months.some((m) => m.source === 'forecast'))
  const hasNotes = $derived(months.some((m) => m.source === 'notes' && m.cents != null))

  function y(cents: number): number {
    return BASE - (cents / max) * PLOT
  }

  /** Column with a 4px rounded top, square at the baseline. */
  function column(x: number, top: number, w: number): string {
    const h = BASE - top
    if (h <= 0.5) return ''
    const r = Math.min(4, w / 2, h)
    return `M${x},${BASE}V${top + r}Q${x},${top} ${x + r},${top}H${x + w - r}Q${x + w},${top} ${x + w},${top + r}V${BASE}Z`
  }

  function describe(m: PlanMonth): string {
    const value = m.cents != null ? formatBRL(m.cents) : 'sem valor'
    const kind = m.source === 'forecast' ? 'previsto' : 'notas'
    return `${formatMonthLong(m.month)}: ${value} (${kind})${m.included ? '' : ', fora do cálculo'}`
  }

  function onKeydown(event: KeyboardEvent, month: string) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      ontoggle(month)
    }
  }
</script>

<div class="chart" bind:clientWidth={width}>
  {#if hasForecast}
    <ul class="legend" aria-hidden="true">
      {#if hasNotes}<li><span class="key notes"></span>notas</li>{/if}
      <li><span class="key forecast"></span>previsto</li>
    </ul>
  {/if}
  {#if width > 0 && months.length}
    <svg {width} height={HEIGHT} role="group" aria-label="Faturamento por mês (clique para incluir ou tirar um mês)">
      <defs>
        <pattern id="hatch-{uid}" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
          <rect class="hatch-bg" width="6" height="6" />
          <rect class="hatch-line" width="2.4" height="6" />
        </pattern>
        <pattern id="hatch-off-{uid}" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
          <rect class="hatch-bg off" width="6" height="6" />
          <rect class="hatch-line off" width="2.4" height="6" />
        </pattern>
      </defs>
      <line class="base" x1="0" x2={plotWidth} y1={BASE + 0.5} y2={BASE + 0.5} />
      {#each months as m, i (m.month)}
        {@const x = i * band + (band - barWidth) / 2}
        {@const forecast = m.source === 'forecast'}
        <g
          class="col"
          class:on={m.included}
          class:hover={hover === i}
          role="button"
          tabindex="0"
          aria-pressed={m.included}
          aria-label={describe(m)}
          onclick={() => ontoggle(m.month)}
          onkeydown={(e) => onKeydown(e, m.month)}
          onpointerenter={() => (hover = i)}
          onpointerleave={() => (hover = null)}
          onfocus={() => (hover = i)}
          onblur={() => (hover = null)}
        >
          <rect class="hit" x={i * band + 1} y="0" width={Math.max(0, band - 2)} height={BASE + AXIS} rx="6" />
          {#if m.cents != null && m.cents > 0}
            <path
              class="bar"
              class:forecast
              d={column(x, y(m.cents), barWidth)}
              fill={forecast ? `url(#${m.included ? 'hatch' : 'hatch-off'}-${uid})` : undefined}
            />
          {:else}
            <rect class="empty" x={x} y={BASE - 3} width={barWidth} height="3" rx="1.5" />
          {/if}
          {#if i % labelEvery === 0 || i === months.length - 1}
            <text class="tick" x={x + barWidth / 2} y={HEIGHT - 6} text-anchor="middle">
              {formatMonthShort(m.month, multiYear ? 'short' : undefined)}
            </text>
          {/if}
        </g>
      {/each}
      {#if avgY != null && average}
        <line class="avg" x1="0" x2={plotWidth + 4} y1={avgY} y2={avgY} />
        <text class="avg-label" x={plotWidth + 8} y={avgY - 3}>
          <tspan class="avg-name">média</tspan>
          <tspan x={plotWidth + 8} dy="13">{formatBRLCompact(average)}</tspan>
        </text>
      {/if}
    </svg>

    {#if hover != null && months[hover]}
      {@const m = months[hover]}
      <div
        class="tip"
        role="presentation"
        style="left: {Math.min(Math.max(hover * band + band / 2, 80), width - 80)}px; top: {y(m.cents ?? 0) - 8}px"
      >
        <strong class="num">{m.cents != null ? formatBRL(m.cents) : 'Sem valor'}</strong>
        <span>
          {formatMonthLong(m.month)} ·
          {m.source === 'forecast'
            ? `previsto${m.projection?.note ? `: ${m.projection.note}` : ''}`
            : plural(m.notesCount, 'nota', 'notas')}
        </span>
        <span class="state" class:off={!m.included}>
          {m.included ? 'Considerado · clique para tirar' : 'Fora do cálculo · clique para incluir'}
        </span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .chart {
    position: relative;
    min-height: 148px;
  }

  svg {
    display: block;
    overflow: visible;
  }

  .legend {
    position: absolute;
    top: -2px;
    right: 0;
    z-index: 1;
    display: flex;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 11px;
    color: var(--text-3);
  }

  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .key {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .key.notes {
    background: var(--viz-1);
  }

  .key.forecast {
    background: repeating-linear-gradient(45deg, var(--viz-1) 0 1.6px, var(--viz-1-soft) 1.6px 4px);
  }

  .base {
    stroke: var(--viz-axis);
    stroke-width: 1;
  }

  .col {
    cursor: pointer;
    outline: none;
  }

  .hit {
    fill: transparent;
    stroke: none;
  }

  .col.hover .hit {
    fill: var(--surface-2);
  }

  .col:focus-visible .hit {
    stroke: var(--focus-ring);
    stroke-width: 2;
  }

  /* Forecast bars take their hatched fill from the attribute (a CSS fill would override it). */
  .bar:not(.forecast) {
    fill: var(--viz-muted);
    transition: fill 0.15s var(--ease);
  }

  .col.on .bar:not(.forecast) {
    fill: var(--viz-1);
  }

  .hatch-bg {
    fill: var(--viz-1);
    opacity: 0.28;
  }

  .hatch-line {
    fill: var(--viz-1);
    opacity: 0.85;
  }

  .hatch-bg.off {
    fill: var(--viz-muted);
    opacity: 0.6;
  }

  .hatch-line.off {
    fill: var(--viz-muted);
    opacity: 1;
  }

  .empty {
    fill: var(--viz-muted);
  }

  .col.hover .bar {
    opacity: 0.85;
  }

  .tick {
    fill: var(--text-3);
    font-size: 11px;
  }

  .col.on .tick {
    fill: var(--text-2);
  }

  .avg {
    stroke: var(--text-3);
    stroke-width: 1;
    opacity: 0.7;
    pointer-events: none;
  }

  .avg-label {
    fill: var(--text-2);
    font-size: 11px;
    font-weight: 600;
    pointer-events: none;
  }

  .avg-name {
    fill: var(--text-3);
    font-weight: 400;
  }

  .tip {
    position: absolute;
    z-index: 5;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    font-size: 12px;
    color: var(--text-2);
    white-space: nowrap;
    transform: translate(-50%, -100%);
    pointer-events: none;
  }

  .tip strong {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .state {
    color: var(--accent-text);
  }

  .state.off {
    color: var(--text-3);
  }
</style>

<script lang="ts">
  // "Período": a compact button (a pill with × when a period is on) that opens the presets, the
  // "Personalizado" range (de/até, either side may be empty) and, on the notes page, the
  // "Incluir notas sem competência" switch.
  import { tick } from 'svelte'
  import { plural } from '../lib/format'
  import { monthKeyOf } from '../lib/format'
  import { ALL_PERIOD, PERIOD_LABEL, periodLabel, periodRange, type Period, type PeriodPreset } from '../lib/period'
  import Icon from './Icon.svelte'
  import MonthInput from './MonthInput.svelte'

  interface Props {
    value: Period
    /** Options, in order ("Personalizado" is always offered after them). */
    presets: PeriodPreset[]
    /** Offer "Seleção manual" (the tax page: months ticked one by one). */
    manual?: boolean
    /** Notes page: rows without a competence month. */
    undated?: { include: boolean; count: number; left: number } | null
    /** A line under the options (e.g. "5 meses no período"). */
    summary?: string | null
    onchange: (period: Period) => void
    onundated?: (include: boolean) => void
  }

  let { value, presets, manual = false, undated = null, summary = null, onchange, onundated }: Props = $props()

  const WIDTH = 324
  const uid = $props.id()
  const current = monthKeyOf()

  let open = $state(false)
  let root: HTMLDivElement | undefined = $state()
  let button: HTMLButtonElement | undefined = $state()
  let popover: HTMLDivElement | undefined = $state()
  let position = $state({ top: 0, left: 0 })

  const active = $derived(value.preset !== 'all')
  const label = $derived(periodLabel(value))
  /** The de/até fields show the months of the chosen preset (typing there makes it "Personalizado"). */
  const shown = $derived(value.preset === 'range' ? { from: value.from, to: value.to } : periodRange(value, current))

  async function show() {
    if (!button) return
    const r = button.getBoundingClientRect()
    position = { top: r.bottom + 6, left: Math.max(8, Math.min(r.left, window.innerWidth - WIDTH - 8)) }
    open = true
    await tick()
    popover?.querySelector<HTMLInputElement>('input[type="radio"]:checked, input[type="radio"]')?.focus()
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus()
  }

  function choose(preset: PeriodPreset) {
    if (preset === 'range') {
      const r = periodRange(value, current) ?? { from: current, to: current }
      onchange({ preset: 'range', from: r.from, to: r.to })
    } else {
      onchange({ preset, from: null, to: null })
    }
  }

  function setRange(from: string | null, to: string | null) {
    onchange(from || to ? { preset: 'range', from, to } : { ...ALL_PERIOD })
  }

  $effect(() => {
    if (!open) return
    const onPointer = (e: PointerEvent) => {
      const target = e.target as Node
      if (!root?.contains(target)) close(false)
    }
    const onResize = () => close(false)
    document.addEventListener('pointerdown', onPointer, true)
    window.addEventListener('resize', onResize)
    return () => {
      document.removeEventListener('pointerdown', onPointer, true)
      window.removeEventListener('resize', onResize)
    }
  })
</script>

<div class="period" bind:this={root}>
  <button
    bind:this={button}
    type="button"
    class="trigger"
    class:active
    aria-haspopup="dialog"
    aria-expanded={open}
    title={active ? `Período: ${label} (competência)` : 'Filtrar por período de competência'}
    onclick={() => (open ? close(true) : void show())}
  >
    <Icon name="calendar" size={14} />
    {#if active}
      <span class="value">{label}</span>
    {:else}
      <span class="value">Período</span>
    {/if}
    <Icon name="chevronDown" size={12} class="chev" />
  </button>
  {#if active}
    <button type="button" class="clear" aria-label="Limpar o período (Todos)" title="Limpar o período" onclick={() => onchange({ ...ALL_PERIOD })}>
      <Icon name="x" size={12} strokeWidth={2.2} />
    </button>
  {/if}

  {#if open}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      bind:this={popover}
      class="popover"
      role="dialog"
      aria-label="Período por competência"
      tabindex="-1"
      style="top: {position.top}px; left: {position.left}px; width: {WIDTH}px"
      onkeydown={(e) => {
        if (e.key === 'Escape') {
          e.preventDefault()
          e.stopPropagation()
          close(true)
        }
      }}
    >
      <p class="title">Período <span>(mês de competência)</span></p>
      <div class="options" role="radiogroup" aria-label="Período">
        {#each presets as p (p)}
          <label class="option" class:checked={value.preset === p}>
            <input type="radio" name="period-{uid}" checked={value.preset === p} onchange={() => choose(p)} />
            {PERIOD_LABEL[p]}
          </label>
        {/each}
        {#if manual}
          <label class="option" class:checked={value.preset === 'custom'} title="Os meses marcados um a um em “Incluir”">
            <input type="radio" name="period-{uid}" checked={value.preset === 'custom'} onchange={() => choose('custom')} />
            {PERIOD_LABEL.custom}
          </label>
        {/if}
        <label class="option" class:checked={value.preset === 'range'}>
          <input type="radio" name="period-{uid}" checked={value.preset === 'range'} onchange={() => choose('range')} />
          {PERIOD_LABEL.range}
        </label>
      </div>

      <div class="range" class:dim={value.preset !== 'range'}>
        <span class="k">De</span>
        <MonthInput
          value={shown?.from ?? null}
          label="Período: de (MM/AAAA, vazio = sem início)"
          placeholder="início"
          size="sm"
          floatMessage
          onchange={(m) => setRange(m, value.preset === 'range' ? value.to : (shown?.to ?? null))}
        />
        <span class="k">até</span>
        <MonthInput
          value={shown?.to ?? null}
          label="Período: até (MM/AAAA, vazio = sem fim)"
          placeholder="fim"
          size="sm"
          floatMessage
          onchange={(m) => setRange(value.preset === 'range' ? value.from : (shown?.from ?? null), m)}
        />
      </div>

      {#if undated}
        <label class="toggle" class:dim={!active}>
          <input
            type="checkbox"
            checked={undated.include || !active}
            disabled={!active}
            onchange={(e) => onundated?.(e.currentTarget.checked)}
          />
          <span>
            Incluir notas sem competência
            <span class="count">({undated.count})</span>
            {#if active && undated.left}
              <span class="left">{plural(undated.left, 'ficou de fora', 'ficaram de fora')}</span>
            {/if}
          </span>
        </label>
      {/if}

      {#if summary}<p class="summary">{summary}</p>{/if}

      <div class="foot">
        <button type="button" class="link" disabled={!active} onclick={() => onchange({ ...ALL_PERIOD })}>Limpar (Todos)</button>
        <button type="button" class="btn btn-sm" onclick={() => close(true)}>Fechar</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .period {
    display: inline-flex;
    align-items: center;
    flex: none;
  }

  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 34px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .trigger:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .trigger :global(.chev) {
    color: var(--text-3);
  }

  .trigger.active {
    border-color: var(--accent);
    border-radius: var(--radius) 0 0 var(--radius);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .trigger.active :global(.chev) {
    color: var(--accent-text);
  }

  .clear {
    display: grid;
    place-items: center;
    width: 26px;
    height: 34px;
    margin-left: -1px;
    border: 1px solid var(--accent);
    border-radius: 0 var(--radius) var(--radius) 0;
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .clear:hover {
    background: var(--accent-soft-2);
  }

  .popover {
    position: fixed;
    z-index: 65;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    animation: pop 0.12s var(--ease);
  }

  .popover:focus {
    outline: none;
  }

  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text);
  }

  .title span {
    font-weight: 400;
    color: var(--text-3);
  }

  .options {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 4px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text-2);
    font-size: 12.5px;
    cursor: pointer;
    white-space: nowrap;
  }

  .option:hover {
    background: var(--surface-2);
  }

  .option.checked {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-weight: 500;
  }

  .option input {
    margin: 0;
    accent-color: var(--accent);
  }

  .range {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto minmax(0, 1fr);
    align-items: center;
    gap: 6px;
  }

  .range.dim .k {
    color: var(--text-3);
  }

  .k {
    font-size: 12px;
    color: var(--text-2);
  }

  .toggle {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--border);
    font-size: 12.5px;
    color: var(--text-2);
    cursor: pointer;
  }

  .toggle.dim {
    color: var(--text-3);
    cursor: default;
  }

  .toggle input {
    margin: 2px 0 0;
    accent-color: var(--accent);
  }

  .count {
    color: var(--text-3);
  }

  .left {
    display: block;
    margin-top: 1px;
    font-size: 11.5px;
    color: var(--warn);
  }

  .summary {
    font-size: 12px;
    color: var(--text-3);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 2px;
  }

  .foot .link {
    font-size: 12.5px;
  }

  .foot .link:disabled {
    color: var(--text-3);
    cursor: default;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
</style>

<script lang="ts">
  import { tick } from 'svelte'
  import { normalizeText } from '../lib/format'
  import type { CnaeInfo } from '../lib/types'
  import Icon from './Icon.svelte'

  interface Props {
    options: CnaeInfo[]
    /** The chosen CNAE (already resolved to the default when none was chosen). */
    value: CnaeInfo | null
    onchange: (code: string) => void
    id?: string
    labelledby?: string
  }

  let { options, value, onchange, id, labelledby }: Props = $props()

  const PANEL_WIDTH = 380
  const PANEL_MAX_HEIGHT = 340
  /** Only digits: the query is (the start of) a CNAE code. */
  const DIGITS_RE = /^\d+$/

  let open = $state(false)
  let query = $state('')
  let highlighted = $state(0)
  let button: HTMLButtonElement | undefined = $state()
  let panel: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let position = $state({ top: 0, left: 0, width: PANEL_WIDTH })
  let openedAt = 0
  const uid = $props.id()
  const listId = `cnae-${uid}`

  const filtered = $derived.by(() => {
    const q = normalizeText(query).replace(/[^a-z0-9 ]/g, '')
    if (!q) return options
    const digits = q.replaceAll(' ', '')
    const byCode = DIGITS_RE.test(digits)
    return options.filter((o) => {
      const text = normalizeText(`${o.description} ${o.meiOccupation ?? ''}`)
      return (byCode && o.code.replace(/\D/g, '').startsWith(digits)) || text.includes(q)
    })
  })

  /** Groups by annex rule, keeping the catalog order inside each group. */
  const groups = $derived.by(() => {
    const fatorR = filtered.filter((o) => o.activity === 'fatorR')
    const annexIii = filtered.filter((o) => o.activity === 'annexIii')
    return [
      { title: 'Anexo III ou V, pelo Fator R', items: fatorR },
      { title: 'Sempre Anexo III', items: annexIii },
    ].filter((g) => g.items.length)
  })

  /** Options in display order (for the keyboard). */
  const ordered = $derived(groups.flatMap((g) => g.items))

  async function show() {
    if (!button) return
    const r = button.getBoundingClientRect()
    const width = Math.max(r.width, Math.min(PANEL_WIDTH, window.innerWidth - 16))
    const below = r.bottom + 4 + PANEL_MAX_HEIGHT <= window.innerHeight - 8
    position = {
      top: below ? r.bottom + 4 : Math.max(8, r.top - 4 - PANEL_MAX_HEIGHT),
      left: Math.max(8, Math.min(r.left, window.innerWidth - width - 8)),
      width,
    }
    query = ''
    highlighted = Math.max(0, ordered.findIndex((o) => o.code === value?.code))
    open = true
    openedAt = performance.now()
    await tick()
    input?.focus()
    scrollToHighlighted()
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus()
  }

  function choose(option: CnaeInfo) {
    close(true)
    if (option.code !== value?.code) onchange(option.code)
  }

  function scrollToHighlighted() {
    panel?.querySelector(`#${listId}-${highlighted}`)?.scrollIntoView({ block: 'nearest' })
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      highlighted = Math.min(ordered.length - 1, highlighted + 1)
      void tick().then(scrollToHighlighted)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      highlighted = Math.max(0, highlighted - 1)
      void tick().then(scrollToHighlighted)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      const option = ordered[highlighted]
      if (option) choose(option)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      close(true)
    } else if (event.key === 'Tab') {
      close(false)
    }
  }

  $effect(() => {
    if (!open) return
    const onPointer = (e: PointerEvent) => {
      const target = e.target as Node
      if (!panel?.contains(target) && !button?.contains(target)) close(false)
    }
    const onScroll = (e: Event) => {
      if (performance.now() - openedAt < 250) return
      if (!(e.target instanceof Node && panel?.contains(e.target))) close(false)
    }
    const onResize = () => close(false)
    document.addEventListener('pointerdown', onPointer, true)
    document.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onResize)
    return () => {
      document.removeEventListener('pointerdown', onPointer, true)
      document.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onResize)
    }
  })
</script>

<button
  bind:this={button}
  {id}
  type="button"
  class="trigger"
  class:open
  aria-haspopup="listbox"
  aria-expanded={open}
  aria-labelledby={labelledby ? `${labelledby} ${id ?? ''}`.trim() : undefined}
  disabled={!options.length}
  onclick={() => (open ? close(true) : void show())}
>
  {#if value}
    <span class="code num">{value.code}</span>
    <span class="desc">{value.description}</span>
  {:else}
    <span class="desc">Carregando atividades…</span>
  {/if}
  <Icon name="chevronDown" size={14} class="chev" />
</button>

{#if open}
  <div
    bind:this={panel}
    class="panel"
    style="top: {position.top}px; left: {position.left}px; width: {position.width}px; max-height: {PANEL_MAX_HEIGHT}px"
  >
    <div class="search">
      <Icon name="search" size={14} />
      <input
        bind:this={input}
        type="text"
        role="combobox"
        placeholder="Buscar por código ou atividade"
        aria-label="Buscar atividade (CNAE)"
        aria-expanded="true"
        aria-controls={listId}
        aria-autocomplete="list"
        aria-activedescendant={ordered[highlighted] ? `${listId}-${highlighted}` : undefined}
        bind:value={query}
        oninput={() => (highlighted = 0)}
        onkeydown={onKeydown}
      />
    </div>
    <ul id={listId} class="list" role="listbox" aria-label="Atividades (CNAE)">
      {#each groups as group (group.title)}
        <li class="group" role="presentation">{group.title}</li>
        {#each group.items as option (option.code)}
          {@const index = ordered.indexOf(option)}
          <li
            id="{listId}-{index}"
            role="option"
            aria-selected={option.code === value?.code}
            class:highlighted={index === highlighted}
            class:current={option.code === value?.code}
            onmousedown={(e) => {
              e.preventDefault()
              choose(option)
            }}
            onmousemove={() => (highlighted = index)}
          >
            <span class="opt-main">
              <span class="opt-code num">{option.code}</span>
              <span class="opt-desc">{option.description}</span>
            </span>
            <span class="opt-tags">
              {#if option.meiAllowed}<span class="pill ok">MEI</span>{/if}
              {#if option.code === value?.code}<Icon name="check" size={14} />{/if}
            </span>
          </li>
        {/each}
      {/each}
      {#if !ordered.length}
        <li class="empty" role="presentation">Nenhuma atividade encontrada.</li>
      {/if}
    </ul>
  </div>
{/if}

<style>
  .trigger {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 14px;
    grid-template-areas:
      'code chev'
      'desc chev';
    align-items: center;
    column-gap: var(--s-2);
    width: 100%;
    padding: 7px 10px 8px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    text-align: left;
    transition:
      border-color 0.12s var(--ease),
      box-shadow 0.12s var(--ease);
  }

  .trigger:hover:not(:disabled) {
    border-color: var(--text-3);
  }

  .trigger.open {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .trigger:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .code {
    grid-area: code;
    font-size: 13px;
    font-weight: 600;
  }

  .desc {
    grid-area: desc;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--text-2);
  }

  .trigger :global(.chev) {
    grid-area: chev;
    color: var(--text-3);
  }

  .panel {
    position: fixed;
    z-index: 60;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: pop 0.12s var(--ease);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 40px;
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    flex: none;
  }

  .search input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 13px;
    color: var(--text);
  }

  .list {
    margin: 0;
    padding: 4px;
    list-style: none;
    overflow-y: auto;
    min-height: 0;
  }

  .group {
    padding: 8px 8px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  [role='option'] {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s-3);
    padding: 7px 8px;
    border-radius: 5px;
    cursor: pointer;
  }

  [role='option'].highlighted {
    background: var(--surface-2);
  }

  [role='option'].current .opt-code {
    color: var(--accent-text);
  }

  .opt-main {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .opt-code {
    font-size: 12.5px;
    font-weight: 600;
  }

  .opt-desc {
    font-size: 12.5px;
    line-height: 1.35;
    color: var(--text-2);
  }

  .opt-tags {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
    color: var(--accent-text);
  }

  .empty {
    padding: 12px 8px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
</style>

<script lang="ts">
  import { tick } from 'svelte'
  import { normalizeText } from '../lib/format'
  import { store, type Row } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    row: Row
    tabbable: boolean
  }

  let { row, tabbable }: Props = $props()

  let editing = $state(false)
  let draft = $state('')
  let highlighted = $state(-1)
  let input: HTMLInputElement | undefined = $state()
  let position = $state({ top: 0, left: 0, width: 160 })
  let openedAt = 0
  const uid = $props.id()
  const listId = `types-${uid}`

  const suggestions = $derived.by(() => {
    const q = normalizeText(draft)
    if (!q || q === normalizeText(row.docType)) return store.knownTypes
    return store.knownTypes.filter((t) => normalizeText(t).includes(q))
  })

  const isNew = $derived(
    !!draft.trim() && !store.knownTypes.some((t) => normalizeText(t) === normalizeText(draft)),
  )

  async function start(event: MouseEvent) {
    event.stopPropagation()
    draft = row.docType
    highlighted = -1
    editing = true
    openedAt = performance.now()
    await tick()
    if (input) {
      const r = input.getBoundingClientRect()
      position = { top: r.bottom + 4, left: r.left, width: Math.max(r.width, 168) }
      input.focus({ preventScroll: true })
      input.select()
    }
  }

  function finish(value: string | null, refocus: boolean) {
    if (!editing) return
    const tr = input?.closest('tr')
    editing = false
    if (value !== null) store.setType(row, value)
    if (refocus) void tick().then(() => tr?.focus())
  }

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation()
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      highlighted = Math.min(suggestions.length - 1, highlighted + 1)
    } else if (event.key === 'ArrowUp') {
      event.preventDefault()
      highlighted = Math.max(-1, highlighted - 1)
    } else if (event.key === 'Enter') {
      event.preventDefault()
      finish(highlighted >= 0 ? suggestions[highlighted] : draft, true)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      finish(null, true)
    }
  }

  // The suggestion list is position: fixed; close it if anything scrolls.
  $effect(() => {
    if (!editing) return
    const onScroll = (e: Event) => {
      if (performance.now() - openedAt < 250) return
      if (e.target instanceof Node && e.target.contains(input ?? null)) finish(draft, false)
    }
    document.addEventListener('scroll', onScroll, true)
    return () => document.removeEventListener('scroll', onScroll, true)
  })
</script>

{#if editing}
  <input
    bind:this={input}
    class="input type-input"
    type="text"
    role="combobox"
    aria-label="Tipo de {row.name}"
    aria-expanded={suggestions.length > 0}
    aria-controls={listId}
    aria-autocomplete="list"
    aria-activedescendant={highlighted >= 0 ? `${listId}-${highlighted}` : undefined}
    maxlength="40"
    bind:value={draft}
    oninput={() => (highlighted = -1)}
    onkeydown={onKeydown}
    onblur={() => finish(draft, false)}
    onclick={(e) => e.stopPropagation()}
  />
  <ul
    id={listId}
    class="list"
    role="listbox"
    aria-label="Tipos"
    style="top: {position.top}px; left: {position.left}px; min-width: {position.width}px"
  >
    {#each suggestions as type, i (type)}
      <li
        id="{listId}-{i}"
        role="option"
        aria-selected={i === highlighted}
        class:highlighted={i === highlighted}
        class:current={type === row.docType}
        onmousedown={(e) => {
          e.preventDefault()
          finish(type, true)
        }}
      >
        <span>{type}</span>
        {#if type === row.docType}<Icon name="check" size={14} />{/if}
      </li>
    {/each}
    {#if isNew}
      <li class="create" role="presentation">Enter cria o tipo “{draft.trim()}”</li>
    {/if}
  </ul>
{:else}
  <button
    type="button"
    class="type"
    class:edited={row.typeEdited}
    class:placeholder={!row.docType}
    class:expense={row.kind === 'expense'}
    tabindex={tabbable ? 0 : -1}
    title={row.typeEdited ? `Tipo alterado por você (lido: ${row.defaultType || '—'})` : 'Mudar o tipo'}
    aria-label="Tipo: {row.docType || 'sem tipo'}{row.kind === 'expense' ? ', despesa' : ''}. Mudar o tipo"
    onclick={start}
  >
    <span class="lines">
      <span class="text">{row.docType || (row.status === 'pending' ? '…' : '—')}</span>
      {#if row.kind === 'expense'}<span class="kind-line" aria-hidden="true">despesa</span>{/if}
    </span>
    <Icon name="chevronDown" size={12} class="chev" />
  </button>
{/if}

<style>
  .type {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    height: 26px;
    padding: 0 6px 0 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
    font-size: 13px;
    transition:
      background-color 0.12s var(--ease),
      border-color 0.12s var(--ease);
  }

  .lines {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
  }

  .type .text {
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Expenses are marked under the type while the table has no Natureza column. */
  .kind-line {
    font-size: 10.5px;
    font-weight: 600;
    line-height: 13px;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--expense);
  }

  .type.expense {
    height: auto;
    min-height: 26px;
    padding-top: 2px;
    padding-bottom: 2px;
  }

  @container doctable (min-width: 1180px) {
    .kind-line {
      display: none;
    }
  }

  .type :global(.chev) {
    color: var(--text-3);
    opacity: 0;
    transition: opacity 0.12s var(--ease);
  }

  .type:hover,
  .type:focus-visible {
    border-color: var(--border-strong);
    background: var(--surface);
  }

  .type:hover :global(.chev),
  .type:focus-visible :global(.chev) {
    opacity: 1;
  }

  .type.edited .text {
    color: var(--text);
    font-weight: 500;
  }

  .type.placeholder .text {
    color: var(--text-3);
  }

  .type-input {
    width: 100%;
    height: 28px;
    padding: 0 8px;
  }

  .list {
    position: fixed;
    z-index: 60;
    margin: 0;
    padding: 4px;
    list-style: none;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    font-size: 13px;
  }

  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    padding: 6px 8px;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }

  .list li:hover,
  .list li.highlighted {
    background: var(--surface-2);
  }

  .list li.current {
    color: var(--accent-text);
    font-weight: 500;
  }

  .list li.create {
    color: var(--text-3);
    font-size: 12px;
    cursor: default;
  }

  .list li.create:hover {
    background: none;
  }
</style>

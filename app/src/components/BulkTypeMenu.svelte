<script lang="ts">
  import { tick } from 'svelte'
  import { normalizeText, plural } from '../lib/format'
  import { store, type Row } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    /** Rows that get the type (the current selection). */
    rows: Row[]
  }

  let { rows }: Props = $props()

  const PANEL_WIDTH = 248

  let open = $state(false)
  let draft = $state('')
  let highlighted = $state(-1)
  let button: HTMLButtonElement | undefined = $state()
  let panel: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let position = $state({ left: 0, bottom: 0 })
  const uid = $props.id()
  const listId = `bulk-types-${uid}`

  const suggestions = $derived.by(() => {
    const q = normalizeText(draft)
    return q ? store.knownTypes.filter((t) => normalizeText(t).includes(q)) : store.knownTypes
  })
  const isNew = $derived(!!draft.trim() && !store.knownTypes.some((t) => normalizeText(t) === normalizeText(draft)))
  /** Type shared by every selected row, if any (marked in the list). */
  const common = $derived(rows.length && rows.every((r) => r.docType === rows[0].docType) ? rows[0].docType : null)

  async function show() {
    if (!button) return
    const r = button.getBoundingClientRect()
    // The bar sits at the bottom of the table: open upwards.
    position = {
      left: Math.max(8, Math.min(r.left, window.innerWidth - PANEL_WIDTH - 8)),
      bottom: Math.max(8, window.innerHeight - r.top + 6),
    }
    draft = ''
    highlighted = -1
    open = true
    await tick()
    input?.focus()
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus()
  }

  function apply(type: string) {
    const t = type.trim()
    if (!t || !rows.length) return
    close(false)
    store.setTypeMany(rows, t)
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
      apply(highlighted >= 0 ? suggestions[highlighted] : draft)
    } else if (event.key === 'Escape') {
      event.preventDefault()
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
    const onResize = () => close(false)
    document.addEventListener('pointerdown', onPointer, true)
    window.addEventListener('resize', onResize)
    return () => {
      document.removeEventListener('pointerdown', onPointer, true)
      window.removeEventListener('resize', onResize)
    }
  })
</script>

<button
  bind:this={button}
  type="button"
  class="btn btn-sm"
  aria-haspopup="listbox"
  aria-expanded={open}
  onclick={() => (open ? close(true) : void show())}
>
  <Icon name="tag" size={14} />
  Definir tipo…
</button>

{#if open}
  <div
    bind:this={panel}
    class="panel"
    style="left: {position.left}px; bottom: {position.bottom}px; width: {PANEL_WIDTH}px"
  >
    <p class="title">Tipo para {plural(rows.length, 'nota selecionada', 'notas selecionadas')}</p>
    <input
      bind:this={input}
      class="input"
      type="text"
      role="combobox"
      placeholder="Escolha ou digite um tipo"
      aria-label="Tipo para as notas selecionadas"
      aria-expanded="true"
      aria-controls={listId}
      aria-autocomplete="list"
      aria-activedescendant={highlighted >= 0 ? `${listId}-${highlighted}` : undefined}
      maxlength="40"
      bind:value={draft}
      oninput={() => (highlighted = -1)}
      onkeydown={onKeydown}
    />
    <ul id={listId} class="list" role="listbox" aria-label="Tipos">
      {#each suggestions as type, i (type)}
        <li
          id="{listId}-{i}"
          role="option"
          aria-selected={i === highlighted}
          class:highlighted={i === highlighted}
          class:current={type === common}
          onmousedown={(e) => {
            e.preventDefault()
            apply(type)
          }}
        >
          <span>{type}</span>
          {#if type === common}<Icon name="check" size={14} />{/if}
        </li>
      {/each}
      {#if isNew}
        <li class="create" role="presentation">Enter cria o tipo “{draft.trim()}”</li>
      {:else if !suggestions.length}
        <li class="create" role="presentation">Nenhum tipo com esse nome</li>
      {/if}
    </ul>
  </div>
{/if}

<style>
  .panel {
    position: fixed;
    z-index: 60;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    animation: pop 0.12s var(--ease);
  }

  .title {
    padding: 2px 2px 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }

  .input {
    height: 30px;
  }

  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 200px;
    overflow-y: auto;
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
    white-space: normal;
  }

  .list li.create:hover {
    background: none;
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(2px);
    }
  }
</style>

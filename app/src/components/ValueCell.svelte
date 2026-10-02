<script lang="ts">
  import { tick } from 'svelte'
  import { formatAmount, formatBRL, parseMoneyInput } from '../lib/format'
  import { store, type Row } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    row: Row
    tabbable: boolean
  }

  let { row, tabbable }: Props = $props()

  let editing = $state(false)
  let draft = $state('')
  let invalid = $state(false)
  let input: HTMLInputElement | undefined = $state()

  // The table asks for an editor with Enter / double-click on the row.
  $effect(() => {
    if (store.editingPath === row.path && !editing) void start()
  })

  async function start() {
    draft = row.cents != null ? formatAmount(row.cents) : ''
    invalid = false
    editing = true
    await tick()
    input?.focus()
    input?.select()
  }

  function finish(refocus: boolean) {
    const tr = input?.closest('tr')
    editing = false
    if (store.editingPath === row.path) store.editingPath = null
    if (refocus) void tick().then(() => tr?.focus())
  }

  function commit(refocus: boolean): boolean {
    const parsed = parseMoneyInput(draft)
    if (parsed.kind === 'invalid') {
      invalid = true
      return false
    }
    store.setValue(row, parsed.kind === 'empty' ? null : parsed.cents)
    finish(refocus)
    return true
  }

  function onKeydown(event: KeyboardEvent) {
    event.stopPropagation()
    if (event.key === 'Enter') {
      event.preventDefault()
      commit(true)
    } else if (event.key === 'Escape') {
      event.preventDefault()
      finish(true)
    }
  }

  function onBlur() {
    if (editing && !commit(false)) finish(false)
  }

  const hint = $derived(
    (row.cancelled ? 'Nota cancelada: fica fora do total. ' : '') +
      (row.valueEdited
        ? row.origin?.type === 'region'
          ? 'Valor aplicado a partir de uma região marcada. Duplo clique para editar.'
          : 'Valor digitado por você. Deixe vazio para voltar ao valor lido.'
        : 'Duplo clique ou Enter para editar'),
  )
</script>

{#if editing}
  <input
    bind:this={input}
    class="input editor num"
    type="text"
    inputmode="decimal"
    placeholder="0,00"
    aria-label="Valor de {row.name}"
    aria-invalid={invalid}
    title={invalid ? 'Valor inválido. Use o formato 1.234,56' : 'Enter confirma · Esc cancela · vazio volta ao valor lido'}
    bind:value={draft}
    oninput={() => (invalid = false)}
    onkeydown={onKeydown}
    onblur={onBlur}
    onclick={(e) => e.stopPropagation()}
  />
{:else}
  <div class="cell">
    <span class="wrap">
      <!-- Overlaid at the left of the value, so it takes no column width. -->
      <button
        type="button"
        class="edit"
        tabindex={tabbable ? 0 : -1}
        title="Editar valor"
        aria-label="Editar o valor de {row.name}"
        onclick={(e) => {
          e.stopPropagation()
          void start()
        }}
      >
        <Icon name="pencil" size={13} />
      </button>
      {#if row.status === 'pending'}
        <span class="skeleton" aria-label="Lendo"></span>
      {:else if row.cents != null}
        <span class="value num" class:edited={row.valueEdited} class:muted-value={!row.counted} title={hint}>
          {formatBRL(row.cents)}
        </span>
      {:else}
        <span class="none" title={hint}>—</span>
      {/if}
    </span>
  </div>
{/if}

<style>
  .cell {
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }

  .wrap {
    position: relative;
    display: inline-flex;
    align-items: center;
  }

  .value {
    font-weight: 500;
    white-space: nowrap;
  }

  .value.edited {
    color: var(--accent-text);
  }

  .value.muted-value {
    color: var(--text-3);
    text-decoration: line-through;
    text-decoration-color: var(--border-strong);
  }

  .none {
    color: var(--text-3);
  }

  .edit {
    position: absolute;
    top: 50%;
    right: calc(100% + 2px);
    transform: translateY(-50%);
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    opacity: 0;
    transition:
      opacity 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .edit:focus-visible,
  :global(tr:hover) .edit,
  :global(tr:focus-within) .edit {
    opacity: 1;
  }

  .edit:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .editor {
    width: 100%;
    height: 28px;
    padding: 0 8px;
    text-align: right;
  }

  .skeleton {
    display: block;
    width: 72px;
    height: 10px;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--surface-2), var(--surface-3), var(--surface-2));
    background-size: 200% 100%;
    animation: shimmer 1.2s linear infinite;
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

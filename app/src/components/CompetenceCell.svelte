<script lang="ts">
  import { tick } from 'svelte'
  import { formatMonth, formatMonthLong } from '../lib/format'
  import { store, type Row } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import MonthInput from './MonthInput.svelte'

  interface Props {
    row: Row
    tabbable: boolean
  }

  let { row, tabbable }: Props = $props()

  let editing = $state(false)
  let cell: HTMLDivElement | undefined = $state()

  /** Counts towards the taxes but has no month: it is left out of them. */
  const missing = $derived(!row.competence && row.counted && !row.duplicateOf)

  // The table asks for an editor with a double-click on the cell.
  $effect(() => {
    if (store.editingCompetencePath === row.path && !editing) void start()
  })

  async function start() {
    editing = true
    await tick()
  }

  function finish(refocus: boolean) {
    if (!editing) return
    const tr = cell?.closest('tr')
    editing = false
    if (store.editingCompetencePath === row.path) store.editingCompetencePath = null
    if (refocus) void tick().then(() => tr?.focus())
  }

  const hint = $derived.by(() => {
    if (row.competence) {
      const month = formatMonthLong(row.competence)
      if (row.competenceEdited) {
        const read = row.extractedCompetence ? ` (${formatMonth(row.extractedCompetence)})` : ''
        return `Competência ${month}, digitada por você. Deixe vazio para voltar à lida${read}.`
      }
      return `Competência ${month}. Duplo clique para mudar.`
    }
    return missing
      ? 'Sem competência: esta nota fica fora dos impostos. Duplo clique para informar (MM/AAAA).'
      : 'Sem competência. Duplo clique para informar (MM/AAAA).'
  })
</script>

{#if editing}
  <div class="editor" bind:this={cell}>
    <MonthInput
      value={row.competence}
      label="Competência de {row.name} (MM/AAAA)"
      size="sm"
      autofocus
      selectOnFocus
      floatMessage
      onchange={(month, how) => {
        store.setCompetence(row, month)
        if (how !== 'blur') finish(true)
      }}
      oncancel={() => finish(true)}
      onleave={() => finish(false)}
    />
  </div>
{:else}
  <div class="cell" bind:this={cell}>
    {#if row.status === 'pending'}
      <span class="skeleton" aria-label="Lendo"></span>
    {:else if row.competence}
      <span class="month num" class:edited={row.competenceEdited} title={hint}>{formatMonth(row.competence)}</span>
    {:else}
      <span class="none" class:missing title={hint}>
        —{#if missing}<span class="dot" aria-hidden="true"></span><span class="sr-only">sem competência</span>{/if}
      </span>
    {/if}
    <!-- Overlaid right after the month, so it takes no column width. -->
    <button
      type="button"
      class="edit"
      tabindex={tabbable ? 0 : -1}
      title={row.competence ? 'Mudar a competência' : 'Informar a competência'}
      aria-label="{row.competence ? 'Mudar' : 'Informar'} a competência de {row.name}"
      onclick={(e) => {
        e.stopPropagation()
        void start()
      }}
    >
      <Icon name="pencil" size={13} />
    </button>
  </div>
{/if}

<style>
  .cell {
    position: relative;
    display: inline-flex;
    align-items: center;
    max-width: 100%;
  }

  .month {
    white-space: nowrap;
    color: var(--text-2);
  }

  .month.edited {
    color: var(--accent-text);
    font-weight: 500;
  }

  .none {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text-3);
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--viz-warn);
    flex: none;
  }

  .edit {
    position: absolute;
    top: 50%;
    left: calc(100% + 2px);
    transform: translateY(-50%);
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    opacity: 0;
    flex: none;
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
  }

  .skeleton {
    display: block;
    width: 52px;
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

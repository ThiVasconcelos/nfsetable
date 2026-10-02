<script lang="ts">
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  let draft = $state('')

  function add(event: SubmitEvent) {
    event.preventDefault()
    if (store.addExclude(draft) || !draft.trim()) draft = ''
  }
</script>

<div class="chips">
  {#each store.exclude as term (term)}
    <span class="chip">
      <span class="label">{term}</span>
      <button type="button" aria-label="Parar de ignorar “{term}”" title="Parar de ignorar" onclick={() => store.removeExclude(term)}>
        <Icon name="x" size={12} strokeWidth={2} />
      </button>
    </span>
  {/each}
  <form class="add" onsubmit={add}>
    <input
      class="chip-input"
      type="text"
      placeholder={store.exclude.length ? 'Adicionar…' : 'Ex.: cancelada'}
      aria-label="Novo termo para ignorar arquivos"
      bind:value={draft}
      maxlength="60"
    />
  </form>
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 26px;
    padding: 0 3px 0 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-2);
    font-size: 12.5px;
    max-width: 100%;
  }

  .label {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip button {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--text-3);
  }

  .chip button:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .add {
    flex: 1 1 96px;
    min-width: 96px;
  }

  .chip-input {
    width: 100%;
    height: 26px;
    padding: 0 10px;
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    background: transparent;
    font-size: 12.5px;
  }

  .chip-input::placeholder {
    color: var(--text-3);
  }

  .chip-input:focus {
    outline: none;
    border-style: solid;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }
</style>

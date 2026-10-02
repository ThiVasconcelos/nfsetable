<script lang="ts">
  import { baseName, shortenPath } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  const excluded = $derived(store.scan?.excluded ?? [])
  const terms = $derived(store.exclude.map((t) => `“${t}”`).join(', '))

  function dirOf(path: string): string {
    const name = baseName(path)
    return path.slice(0, Math.max(0, path.length - name.length - 1))
  }
</script>

<Modal
  open={store.excludedOpen}
  title="Arquivos ignorados"
  subtitle={terms ? `O nome contém ${terms}.` : undefined}
  size="sm"
  onclose={() => (store.excludedOpen = false)}
>
  {#if excluded.length}
    <ul class="list">
      {#each excluded as path (path)}
        <li title={path}>
          <Icon name="file" size={15} />
          <div class="text">
            <span class="name">{baseName(path)}</span>
            <span class="dir">{shortenPath(dirOf(path), 3)}</span>
          </div>
        </li>
      {/each}
    </ul>
    <p class="note">Para incluir esses arquivos, remova o termo em “Ignorar arquivos que contenham”.</p>
  {:else}
    <p class="note">Nenhum arquivo foi ignorado.</p>
  {/if}

  {#snippet footer()}
    <span class="spacer"></span>
    <button type="button" class="btn" onclick={() => (store.excludedOpen = false)}>Fechar</button>
  {/snippet}
</Modal>

<style>
  .list {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
  }

  .list li:last-child {
    border-bottom: 0;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .dir {
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .note {
    margin-top: var(--s-3);
    font-size: 12.5px;
    color: var(--text-3);
  }

  .spacer {
    flex: 1;
  }
</style>

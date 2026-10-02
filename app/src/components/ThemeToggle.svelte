<script lang="ts">
  import { store, type ThemeChoice } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'

  const OPTIONS: { id: ThemeChoice; label: string; icon: IconName }[] = [
    { id: 'system', label: 'Sistema', icon: 'monitor' },
    { id: 'light', label: 'Claro', icon: 'sun' },
    { id: 'dark', label: 'Escuro', icon: 'moon' },
  ]
</script>

<div class="theme" role="group" aria-label="Tema">
  {#each OPTIONS as option (option.id)}
    <button
      type="button"
      class:active={store.theme === option.id}
      aria-pressed={store.theme === option.id}
      title="Tema: {option.label}"
      aria-label="Tema {option.label.toLowerCase()}"
      onclick={() => store.setTheme(option.id)}
    >
      <Icon name={option.icon} size={15} />
    </button>
  {/each}
</div>

<style>
  .theme {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 26px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    transition:
      background-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  button:hover {
    color: var(--text);
  }

  button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
  }
</style>

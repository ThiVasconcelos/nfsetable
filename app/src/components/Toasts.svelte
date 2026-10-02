<script lang="ts">
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
</script>

<div class="toasts" aria-live="polite" aria-relevant="additions">
  {#each store.toasts as toast (toast.id)}
    <div class="toast {toast.kind}" role={toast.kind === 'error' ? 'alert' : 'status'}>
      <Icon name={toast.kind === 'error' ? 'alertCircle' : toast.kind === 'success' ? 'checkCircle' : 'info'} size={16} />
      <span class="message">{toast.message}</span>
      {#if toast.action}
        {@const action = toast.action}
        <button
          type="button"
          class="action"
          onclick={() => {
            action.run()
            store.dismissToast(toast.id)
          }}>{action.label}</button
        >
      {/if}
      <button type="button" class="close" aria-label="Fechar aviso" onclick={() => store.dismissToast(toast.id)}>
        <Icon name="x" size={14} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 44px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s-2);
    width: max-content;
    max-width: calc(100vw - 32px);
    transform: translateX(-50%);
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: 440px;
    padding: 10px 8px 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    font-size: 13px;
    pointer-events: auto;
    animation: enter 0.18s var(--ease);
  }

  .toast.success :global(svg:first-child) {
    color: var(--ok);
  }

  .toast.error :global(svg:first-child) {
    color: var(--danger);
  }

  .toast.info :global(svg:first-child) {
    color: var(--accent-text);
  }

  .message {
    flex: 1;
    min-width: 0;
  }

  .action {
    flex: none;
    padding: 4px 8px;
    border: 0;
    border-radius: 4px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 12.5px;
    font-weight: 600;
  }

  .action:hover {
    background: var(--accent-soft-2);
  }

  .close {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
  }

  .close:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  @keyframes enter {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
</style>

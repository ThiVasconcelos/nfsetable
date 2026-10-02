<script lang="ts">
  import { store } from '../lib/store.svelte'

  const determinate = $derived(store.phase === 'extracting' && store.progress.total > 0)
  const percent = $derived(determinate ? Math.round((store.progress.done / store.progress.total) * 100) : 0)
</script>

<div
  class="progress"
  class:visible={store.busy}
  role="progressbar"
  aria-label={store.phase === 'scanning' ? 'Procurando PDFs' : 'Lendo notas'}
  aria-hidden={!store.busy}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={determinate ? percent : undefined}
>
  {#if determinate}
    <div class="bar" style="width: {Math.max(4, percent)}%"></div>
  {:else}
    <div class="bar indeterminate"></div>
  {/if}
</div>

<style>
  .progress {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    overflow: hidden;
    z-index: 30;
    opacity: 0;
    transition: opacity 0.25s var(--ease);
    pointer-events: none;
  }

  .progress.visible {
    opacity: 1;
  }

  .bar {
    height: 100%;
    background: var(--accent);
    border-radius: 0 2px 2px 0;
    transition: width 0.2s var(--ease);
  }

  .indeterminate {
    width: 30%;
    animation: slide 1.1s var(--ease) infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
</style>

<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'

  interface Props {
    open: boolean
    title: string
    subtitle?: string
    size?: 'sm' | 'md' | 'lg'
    onclose: () => void
    children: Snippet
    footer?: Snippet
  }

  let { open, title, subtitle, size = 'md', onclose, children, footer }: Props = $props()

  let dialog: HTMLDialogElement | undefined = $state()
  const uid = $props.id()
  const titleId = `modal-${uid}`

  $effect(() => {
    if (!dialog) return
    if (open && !dialog.open) dialog.showModal()
    else if (!open && dialog.open) dialog.close()
  })

  function onBackdrop(event: MouseEvent) {
    // Clicks on the ::backdrop target the <dialog> itself.
    if (event.target === dialog) onclose()
  }
</script>

<!-- Esc is handled by the native "cancel" event; the click handler only detects backdrop clicks. -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={dialog}
  class="modal {size}"
  aria-labelledby={titleId}
  oncancel={(e) => {
    e.preventDefault()
    onclose()
  }}
  onclick={onBackdrop}
>
  {#if open}
    <div class="inner">
      <header class="head">
        <div class="titles">
          <h2 id={titleId}>{title}</h2>
          {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
        </div>
        <button type="button" class="btn btn-ghost btn-icon btn-sm" aria-label="Fechar" onclick={onclose}>
          <Icon name="x" size={16} />
        </button>
      </header>
      <div class="body">
        {@render children()}
      </div>
      {#if footer}
        <footer class="foot">
          {@render footer()}
        </footer>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  .modal {
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-pop);
    width: min(560px, calc(100vw - 32px));
    max-height: min(720px, calc(100vh - 48px));
    overflow: hidden;
  }

  .modal.sm {
    width: min(440px, calc(100vw - 32px));
  }

  .modal.lg {
    width: min(760px, calc(100vw - 32px));
  }

  .modal[open] {
    animation: pop 0.16s var(--ease);
  }

  .modal::backdrop {
    background: var(--overlay);
  }

  .inner {
    display: flex;
    flex-direction: column;
    max-height: min(720px, calc(100vh - 48px));
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s-3);
    padding: var(--s-4) var(--s-4) var(--s-3) var(--s-5);
  }

  h2 {
    font-size: 16px;
    font-weight: 600;
    line-height: 1.35;
  }

  .subtitle {
    margin-top: 2px;
    font-size: 13px;
    color: var(--text-3);
  }

  .body {
    padding: 0 var(--s-5) var(--s-4);
    overflow: auto;
    min-height: 0;
  }

  .foot {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-4) var(--s-3) var(--s-5);
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(4px) scale(0.99);
    }
  }
</style>

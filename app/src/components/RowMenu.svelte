<script lang="ts">
  import { tick } from 'svelte'
  import { dismissOnOutside, menuKeydown } from '../lib/popup'
  import { store, type Row } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    row: Row
    tabbable: boolean
  }

  let { row, tabbable }: Props = $props()

  const MENU_WIDTH = 252
  const MENU_HEIGHT = 226

  let open = $state(false)
  let button: HTMLButtonElement | undefined = $state()
  let menu: HTMLDivElement | undefined = $state()
  let position = $state({ top: 0, left: 0 })

  async function toggle(event: MouseEvent) {
    event.stopPropagation()
    if (open) {
      close(true)
      return
    }
    if (!button) return
    button.scrollIntoView({ block: 'nearest' })
    const r = button.getBoundingClientRect()
    const below = r.bottom + 4 + MENU_HEIGHT <= window.innerHeight
    position = {
      top: below ? r.bottom + 4 : Math.max(8, r.top - 4 - MENU_HEIGHT),
      left: Math.max(8, r.right - MENU_WIDTH),
    }
    open = true
    await tick()
    menu?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus({ preventScroll: true })
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus({ preventScroll: true })
  }

  function run(action: () => void) {
    close(false)
    action()
  }

  function onMenuKeydown(event: KeyboardEvent) {
    event.stopPropagation()
    menuKeydown(event, menu, close)
  }

  $effect(() => {
    if (!open) return
    return dismissOnOutside(() => [menu, button], () => close(false), { scroll: true })
  })
</script>

<button
  bind:this={button}
  type="button"
  class="trigger"
  class:open
  tabindex={tabbable ? 0 : -1}
  aria-haspopup="menu"
  aria-expanded={open}
  aria-label="Ações para {row.name}"
  title="Ações"
  onclick={toggle}
>
  <Icon name="more" size={16} />
</button>

{#if open}
  <div
    bind:this={menu}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label="Ações para {row.name}"
    style="top: {position.top}px; left: {position.left}px; width: {MENU_WIDTH}px"
    onkeydown={onMenuKeydown}
    onclick={(e) => e.stopPropagation()}
  >
    <button type="button" role="menuitem" onclick={() => run(() => store.openPreview(row.path))}>
      <Icon name="fileText" size={15} /> Ver prévia
    </button>
    <button type="button" role="menuitem" onclick={() => run(() => void store.openFile(row))}>
      <Icon name="externalLink" size={15} /> Abrir no visualizador
    </button>
    <button type="button" role="menuitem" onclick={() => run(() => void store.revealFile(row.path))}>
      <Icon name="folderOpen" size={15} /> Mostrar na pasta
    </button>
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" onclick={() => run(() => store.toggleKind(row))}>
      {#if row.kind === 'expense'}
        <Icon name="coins" size={15} /> Marcar como receita
      {:else}
        <Icon name="receipt" size={15} /> Marcar como despesa
      {/if}
    </button>
    <button type="button" role="menuitem" onclick={() => run(() => store.ruleFromRow(row))}>
      <Icon name="wand" size={15} /> Criar regra a partir desta nota
    </button>
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" class="danger" onclick={() => run(() => store.removeRow(row))}>
      <Icon name="trash" size={15} /> Remover da lista
    </button>
  </div>
{/if}

<style>
  .trigger {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
    transition:
      background-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .trigger:hover,
  .trigger.open {
    background: var(--surface-3);
    color: var(--text);
  }

  .menu {
    position: fixed;
    z-index: 60;
    display: flex;
    flex-direction: column;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    animation: pop 0.12s var(--ease);
  }

  .menu:focus {
    outline: none;
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 10px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
    white-space: nowrap;
  }

  .menu button :global(svg) {
    color: var(--text-3);
  }

  .menu button:hover,
  .menu button:focus-visible {
    background: var(--surface-2);
    outline: none;
  }

  .menu button.danger,
  .menu button.danger :global(svg) {
    color: var(--danger);
  }

  .menu button.danger:hover,
  .menu button.danger:focus-visible {
    background: var(--danger-soft);
  }

  .sep {
    height: 1px;
    margin: 4px 2px;
    background: var(--border);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
</style>

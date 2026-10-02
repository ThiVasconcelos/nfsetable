<script lang="ts">
  // The active company in the header, with the list to switch and "Nova empresa", "Renomear" and
  // "Excluir" (see ./CompanyDialogs.svelte).
  import { tick } from 'svelte'
  import { companies, type CompanyDialog } from '../lib/companies.svelte'
  import Icon from './Icon.svelte'

  const MENU_WIDTH = 280

  let open = $state(false)
  let button: HTMLButtonElement | undefined = $state()
  let menu: HTMLDivElement | undefined = $state()
  let position = $state({ top: 0, left: 0 })

  const active = $derived(companies.active)
  const only = $derived(companies.list.length <= 1)

  async function show() {
    if (!button) return
    const r = button.getBoundingClientRect()
    position = { top: r.bottom + 6, left: Math.max(8, Math.min(r.left, window.innerWidth - MENU_WIDTH - 8)) }
    open = true
    await tick()
    menu?.querySelector<HTMLButtonElement>('[aria-checked="true"], [role^="menuitem"]')?.focus()
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus()
  }

  function choose(id: string) {
    close(true)
    void companies.switchTo(id)
  }

  function ask(dialog: CompanyDialog) {
    close(false)
    companies.dialog = dialog
  }

  function onMenuKeydown(event: KeyboardEvent) {
    const items = [...(menu?.querySelectorAll<HTMLButtonElement>('[role^="menuitem"]:not(:disabled)') ?? [])]
    const index = items.indexOf(document.activeElement as HTMLButtonElement)
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      const step = event.key === 'ArrowDown' ? 1 : -1
      items[(index + step + items.length) % items.length]?.focus()
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault()
      items[event.key === 'Home' ? 0 : items.length - 1]?.focus()
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
      if (!menu?.contains(target) && !button?.contains(target)) close(false)
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
  class="switcher"
  aria-haspopup="menu"
  aria-expanded={open}
  aria-label="Empresa: {active?.name ?? ''}. Trocar ou gerenciar empresas"
  title="Empresa: {active?.name ?? ''}{active?.cnpj ? ` (${active.cnpj})` : ''}"
  disabled={!active}
  onclick={() => (open ? close(true) : void show())}
>
  <Icon name="building" size={15} />
  <span class="name">{active?.name ?? '…'}</span>
  {#if companies.busy}
    <span class="spinner small"></span>
  {:else}
    <Icon name="chevronDown" size={13} class="chev" />
  {/if}
</button>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    bind:this={menu}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label="Empresas"
    style="top: {position.top}px; left: {position.left}px; width: {MENU_WIDTH}px"
    onkeydown={onMenuKeydown}
    onclick={(e) => e.stopPropagation()}
  >
    <p class="group">Empresas</p>
    {#each companies.list as c (c.id)}
      {@const current = c.id === companies.activeId}
      <button type="button" role="menuitemradio" aria-checked={current} class:current onclick={() => choose(c.id)}>
        <span class="check">{#if current}<Icon name="check" size={14} strokeWidth={2.2} />{/if}</span>
        <span class="text">
          <span class="n">{c.name}</span>
          {#if c.cnpj}<span class="cnpj num">{c.cnpj}</span>{/if}
        </span>
      </button>
    {/each}
    <div class="sep" role="separator"></div>
    <button type="button" role="menuitem" onclick={() => ask({ mode: 'create' })}>
      <Icon name="plus" size={15} /> Nova empresa…
    </button>
    {#if companies.list.length > 1}
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          close(false)
          void companies.showPicker()
        }}
      >
        <Icon name="layers" size={15} /> Ver todas as empresas
      </button>
    {/if}
    {#if active}
      <button type="button" role="menuitem" onclick={() => ask({ mode: 'rename', id: active.id })}>
        <Icon name="pencil" size={15} /> Renomear ou CNPJ…
      </button>
      <button
        type="button"
        role="menuitem"
        class="danger"
        disabled={only}
        title={only ? 'A única empresa não pode ser excluída' : undefined}
        onclick={() => ask({ mode: 'delete', id: active.id })}
      >
        <Icon name="trash" size={15} /> Excluir “{active.name}”…
      </button>
    {/if}
  </div>
{/if}

<style>
  .switcher {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    max-width: 220px;
    height: 32px;
    padding: 0 8px 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .switcher:hover:not(:disabled),
  .switcher[aria-expanded='true'] {
    border-color: var(--border-strong);
    background: var(--surface-2);
  }

  .switcher :global(svg) {
    flex: none;
    color: var(--accent-text);
  }

  .switcher :global(.chev) {
    color: var(--text-3);
  }

  .name {
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .spinner.small {
    width: 12px;
    height: 12px;
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

  .group {
    padding: 6px 10px 4px;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 32px;
    padding: 4px 10px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
  }

  .menu button :global(svg) {
    flex: none;
    color: var(--text-3);
  }

  .menu button:hover:not(:disabled),
  .menu button:focus-visible {
    background: var(--surface-2);
    outline: none;
  }

  .menu button:disabled {
    color: var(--text-3);
    cursor: not-allowed;
  }

  .check {
    display: grid;
    place-items: center;
    width: 15px;
    flex: none;
  }

  .check :global(svg) {
    color: var(--accent-text) !important;
  }

  .text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .n {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .current .n {
    font-weight: 600;
  }

  .cnpj {
    font-size: 11.5px;
    color: var(--text-3);
  }

  .menu button.danger:not(:disabled),
  .menu button.danger:not(:disabled) :global(svg) {
    color: var(--danger);
  }

  .menu button.danger:hover:not(:disabled) {
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

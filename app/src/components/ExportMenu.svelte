<script lang="ts">
  import { tick } from 'svelte'
  import { formatInt, plural } from '../lib/format'
  import { EXPORT_FORMATS, EXPORT_FORMAT_ORDER } from '../lib/labels'
  import { dismissOnOutside, menuKeydown } from '../lib/popup'
  import { store } from '../lib/store.svelte'
  import type { ExportFormat } from '../lib/types'
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'

  const ICONS: Record<ExportFormat, IconName> = {
    csv: 'fileGrid',
    xlsx: 'sheet',
    pdf: 'fileText',
    postgresql: 'database',
    mysql: 'database',
  }
  const MENU_WIDTH = 320

  let open = $state(false)
  let button: HTMLButtonElement | undefined = $state()
  let menu: HTMLDivElement | undefined = $state()
  let position = $state({ top: 0, left: 0 })

  const count = $derived(store.visibleRows.length)
  const filtered = $derived(count < store.rows.length)
  const disabled = $derived(!count || store.busy)
  const title = $derived(
    store.busy
      ? 'Aguarde a leitura terminar'
      : !count
        ? 'Nenhuma nota para exportar'
        : filtered
          ? `Exporta as ${formatInt(count)} notas visíveis (filtro ativo), na ordem da tabela`
          : 'Salvar a tabela em CSV, Excel, PDF ou SQL',
  )

  // Reading started (or the table emptied) while the menu was open.
  $effect(() => {
    if (disabled && open) close(false)
  })

  async function show(focusLast = false) {
    if (!button || disabled) return
    const r = button.getBoundingClientRect()
    position = { top: r.bottom + 6, left: Math.max(8, Math.min(r.right - MENU_WIDTH, window.innerWidth - MENU_WIDTH - 8)) }
    open = true
    await tick()
    const items = menu?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')
    items?.[focusLast ? items.length - 1 : 0]?.focus()
  }

  function close(refocus: boolean) {
    open = false
    if (refocus) button?.focus()
  }

  function choose(format: ExportFormat) {
    close(true)
    void store.exportTable(format)
  }

  function onButtonKeydown(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      void show(event.key === 'ArrowUp')
    }
  }

  function onMenuKeydown(event: KeyboardEvent) {
    menuKeydown(event, menu, close)
  }

  $effect(() => {
    if (!open) return
    return dismissOnOutside(() => [menu, button], () => close(false))
  })
</script>

<button
  bind:this={button}
  type="button"
  class="btn btn-primary trigger"
  class:open
  aria-haspopup="menu"
  aria-expanded={open}
  {disabled}
  {title}
  onclick={() => (open ? close(true) : void show())}
  onkeydown={onButtonKeydown}
>
  <Icon name="download" size={15} />
  Exportar
  <Icon name="chevronDown" size={14} class="chev" />
</button>

{#if open}
  <div
    bind:this={menu}
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label="Exportar as notas visíveis"
    style="top: {position.top}px; left: {position.left}px; width: {MENU_WIDTH}px"
    onkeydown={onMenuKeydown}
  >
    <div class="head" aria-hidden="true">
      {plural(count, 'nota', 'notas')}{filtered ? ' visíveis (filtro ativo)' : ''} · na ordem da tabela
    </div>
    {#each EXPORT_FORMAT_ORDER as format (format)}
      {@const info = EXPORT_FORMATS[format]}
      <button type="button" role="menuitem" data-format={format} onclick={() => choose(format)}>
        <span class="icon"><Icon name={ICONS[format]} size={16} /></span>
        <span class="text">
          <span class="label">{info.label}</span>
          <span class="hint">{info.hint}</span>
        </span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .trigger :global(.chev) {
    margin-left: -2px;
    margin-right: -4px;
    opacity: 0.85;
    transition: transform 0.15s var(--ease);
  }

  .trigger.open :global(.chev) {
    transform: rotate(180deg);
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

  .head {
    padding: 6px 10px 6px;
    font-size: 12px;
    color: var(--text-3);
  }

  .menu button {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--text);
    text-align: left;
  }

  .menu button:hover,
  .menu button:focus-visible {
    background: var(--surface-2);
    outline: none;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: var(--surface-2);
    color: var(--text-2);
    flex: none;
  }

  .menu button:hover .icon,
  .menu button:focus-visible .icon {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .label {
    font-size: 13px;
    font-weight: 500;
  }

  .hint {
    font-size: 12px;
    color: var(--text-3);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
</style>

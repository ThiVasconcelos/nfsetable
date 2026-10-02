<script lang="ts">
  import { formatBRL, formatMonth } from '../lib/format'
  import { originLabel } from '../lib/labels'
  import { KIND_LABEL, store, type Row } from '../lib/store.svelte'
  import type { Rect, RenderedPage } from '../lib/types'
  import Icon from './Icon.svelte'
  import PageView from './PageView.svelte'
  import RegionResult from './RegionResult.svelte'
  import StatusBadge from './StatusBadge.svelte'

  interface Props {
    row: Row
    zoom: number
  }

  let { row, zoom = $bindable() }: Props = $props()

  const ZOOMS = [1, 1.5, 2, 3]

  // Initialized once per row (the parent re-creates this component when the row changes).
  // svelte-ignore state_referenced_locally
  let page = $state(row.highlight?.page ?? 0)
  // svelte-ignore state_referenced_locally
  let pageCount = $state(Math.max(1, row.pageCount))
  let drawMode = $state(false)

  const canMark = $derived(row.extractionStatus !== 'error' && row.extractionStatus !== 'noText')
  const region = $derived(store.region && store.region.path === row.path ? store.region : null)
  const highlight = $derived(row.highlight && row.highlight.page === page ? row.highlight.bbox : null)
  const drawn = $derived(region && region.page === page ? region.rect : null)

  const hint = $derived.by((): { kind: 'warn' | 'info' | 'error'; text: string } | null => {
    if (row.status === 'duplicate')
      return { kind: 'info', text: `Cópia idêntica de “${row.duplicateName}”. Fica fora dos totais.` }
    if (row.extractionStatus === 'error')
      return { kind: 'error', text: row.message ?? 'Não foi possível abrir o arquivo.' }
    if (row.cents == null && row.extractionStatus === 'noText')
      return {
        kind: 'warn',
        text: 'Este PDF não tem texto (parece uma imagem digitalizada). Digite o valor direto na tabela.',
      }
    if (row.cents == null && row.extractionStatus === 'notFound')
      return {
        kind: 'warn',
        text: 'Valor não encontrado. Clique em “Marcar valor na página” e desenhe um retângulo em volta dele.',
      }
    if (row.origin?.type === 'manual') return { kind: 'info', text: 'Valor digitado por você.' }
    return null
  })

  function onRendered(p: RenderedPage) {
    pageCount = Math.max(1, p.pageCount)
    if (page >= pageCount) page = pageCount - 1
  }

  function onRegion(rect: Rect) {
    drawMode = false
    void store.readRegion(row.path, page, rect)
  }

  function zoomIn() {
    zoom = ZOOMS.find((z) => z > zoom) ?? zoom
  }

  function zoomOut() {
    zoom = [...ZOOMS].reverse().find((z) => z < zoom) ?? zoom
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return
    event.stopPropagation()
    if (drawMode) drawMode = false
    else store.closePreview()
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<aside class="preview" aria-label="Prévia de {row.name}" onkeydown={onKeydown}>
  <header class="head">
    <div class="titles">
      <span class="kicker">Prévia</span>
      <h2 class="name" title={row.path}>{row.name}</h2>
    </div>
    <div class="head-actions">
      <button
        type="button"
        class="btn btn-ghost btn-icon btn-sm"
        title="Abrir no visualizador"
        aria-label="Abrir no visualizador de PDF"
        onclick={() => store.openFile(row)}
      >
        <Icon name="externalLink" size={15} />
      </button>
      <button type="button" class="btn btn-ghost btn-icon btn-sm" aria-label="Fechar prévia" title="Fechar (Esc)" onclick={() => store.closePreview()}>
        <Icon name="x" size={16} />
      </button>
    </div>
  </header>

  <div class="summary">
    <StatusBadge status={row.status} />
    <span class="value num" class:none={row.cents == null}>{row.cents != null ? formatBRL(row.cents) : 'Sem valor'}</span>
    {#if row.origin}<span class="origin">· {originLabel(row.origin)}</span>{/if}
    <button
      type="button"
      class="kind-pill"
      class:expense={row.kind === 'expense'}
      title="Clique para marcar como {row.kind === 'expense' ? 'receita' : 'despesa'}"
      aria-label="Natureza: {KIND_LABEL[row.kind]}. Marcar como {row.kind === 'expense' ? 'receita' : 'despesa'}"
      onclick={() => store.toggleKind(row)}
    >
      {KIND_LABEL[row.kind]}
    </button>
  </div>

  {#if row.status !== 'pending' && row.extractionStatus !== 'error'}
    <p class="facts">
      <span title="Valor bruto da nota, antes das retenções: é o faturamento usado nos impostos">
        <span class="k">Valor do serviço</span>
        <span class="v num" class:none={row.serviceCents == null}>
          {row.serviceCents != null ? formatBRL(row.serviceCents) : 'não encontrado'}
        </span>
      </span>
      <span class="sep" aria-hidden="true">·</span>
      <span
        title={row.competence
          ? 'Mês de competência usado nos impostos'
          : 'Sem competência a nota fica fora dos impostos. Informe na coluna Competência.'}
      >
        <span class="k">Competência</span>
        <span class="v num" class:none={!row.competence} class:missing={!row.competence && row.counted && !row.duplicateOf}>
          {row.competence ? formatMonth(row.competence) : 'não encontrada'}
        </span>
        {#if row.competenceEdited}<span class="tag">editada</span>{/if}
      </span>
    </p>
  {/if}

  {#if hint}
    <div class="hint {hint.kind}">
      <Icon name={hint.kind === 'error' ? 'alertCircle' : hint.kind === 'warn' ? 'alert' : 'info'} size={15} />
      <span>{hint.text}</span>
    </div>
  {/if}

  <div class="toolbar">
    <div class="group" role="group" aria-label="Páginas">
      <button type="button" class="btn btn-ghost btn-icon btn-sm" aria-label="Página anterior" disabled={page <= 0} onclick={() => (page -= 1)}>
        <Icon name="chevronLeft" size={16} />
      </button>
      <span class="pages num" aria-live="polite">{page + 1} / {pageCount}</span>
      <button
        type="button"
        class="btn btn-ghost btn-icon btn-sm"
        aria-label="Próxima página"
        disabled={page >= pageCount - 1}
        onclick={() => (page += 1)}
      >
        <Icon name="chevronRight" size={16} />
      </button>
    </div>

    <div class="group" role="group" aria-label="Zoom">
      <button type="button" class="btn btn-ghost btn-icon btn-sm" aria-label="Diminuir zoom" disabled={zoom <= ZOOMS[0]} onclick={zoomOut}>
        <Icon name="zoomOut" size={16} />
      </button>
      <button
        type="button"
        class="btn btn-ghost btn-sm fit"
        class:active={zoom === 1}
        title="Ajustar à largura"
        aria-label="Ajustar à largura"
        onclick={() => (zoom = 1)}
      >
        {#if zoom === 1}<Icon name="fitWidth" size={15} /> Largura{:else}<span class="num">{Math.round(zoom * 100)}%</span>{/if}
      </button>
      <button
        type="button"
        class="btn btn-ghost btn-icon btn-sm"
        aria-label="Aumentar zoom"
        disabled={zoom >= ZOOMS[ZOOMS.length - 1]}
        onclick={zoomIn}
      >
        <Icon name="zoomIn" size={16} />
      </button>
    </div>

    <button
      type="button"
      class="btn btn-sm mark"
      class:btn-primary={drawMode}
      aria-pressed={drawMode}
      disabled={!canMark}
      title={canMark ? 'Desenhe um retângulo em volta do valor na página' : 'Indisponível: o PDF não tem texto legível'}
      onclick={() => (drawMode = !drawMode)}
    >
      <Icon name="marquee" size={15} />
      {drawMode ? 'Arraste na página…' : 'Marcar valor na página'}
    </button>
  </div>

  <PageView
    path={row.path}
    {page}
    {zoom}
    {drawMode}
    {highlight}
    {drawn}
    onrendered={onRendered}
    onregion={onRegion}
  />

  {#if drawMode}
    <div class="draw-tip">
      <Icon name="marquee" size={14} />
      Arraste um retângulo em volta do valor. <kbd>Esc</kbd> cancela.
    </div>
  {:else if region}
    <RegionResult {region} onredraw={() => (drawMode = true)} />
  {/if}
</aside>

<style>
  .preview {
    display: flex;
    flex-direction: column;
    width: var(--preview-w);
    min-height: 0;
    background: var(--surface);
    border-left: 1px solid var(--border);
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-2) 0 var(--s-4);
  }

  .titles {
    min-width: 0;
  }

  .kicker {
    font-size: 11.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .name {
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .head-actions {
    display: flex;
    gap: 2px;
    flex: none;
  }

  .summary {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-2) var(--s-4) 0;
    font-size: 13px;
    min-width: 0;
  }

  .summary .value {
    font-size: 15px;
    font-weight: 600;
  }

  .summary .value.none {
    color: var(--text-3);
    font-weight: 500;
  }

  .origin {
    min-width: 0;
    color: var(--text-3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .kind-pill {
    flex: none;
    margin-left: auto;
    height: 22px;
    padding: 0 9px;
    border: 1px solid transparent;
    border-radius: 999px;
    background: var(--muted-soft);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }

  .kind-pill.expense {
    background: var(--expense-soft);
    color: var(--expense);
  }

  .kind-pill:hover,
  .kind-pill:focus-visible {
    border-color: var(--border-strong);
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    column-gap: 6px;
    margin: 4px 0 0;
    padding: 0 var(--s-4);
    font-size: 12.5px;
    line-height: 20px;
    color: var(--text-3);
  }

  .facts > span {
    white-space: nowrap;
  }

  .facts .v {
    margin-left: 2px;
    font-weight: 500;
    color: var(--text);
  }

  .facts .v.none {
    font-weight: 400;
    color: var(--text-3);
  }

  .facts .v.missing {
    color: var(--warn);
  }

  .facts .tag {
    margin-left: 4px;
  }

  .tag {
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 11px;
    font-weight: 500;
    line-height: 17px;
  }

  .hint {
    display: flex;
    gap: var(--s-2);
    margin: var(--s-3) var(--s-4) 0;
    padding: var(--s-2) var(--s-3);
    border-radius: var(--radius);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .hint :global(svg) {
    margin-top: 1px;
  }

  .hint.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .hint.error {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .hint.info {
    background: var(--surface-2);
    color: var(--text-2);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: var(--s-3) var(--s-3) var(--s-2) var(--s-3);
    border-bottom: 1px solid var(--border);
    flex-wrap: wrap;
  }

  .group {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }

  .pages {
    min-width: 44px;
    text-align: center;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .fit {
    min-width: 76px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .mark {
    margin-left: auto;
  }

  .draw-tip {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 10px var(--s-4);
    border-top: 1px solid var(--border);
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 12.5px;
  }

  kbd {
    padding: 1px 5px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: var(--surface);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--text-2);
  }
</style>

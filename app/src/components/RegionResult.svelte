<script lang="ts">
  import { formatBRL, formatInt } from '../lib/format'
  import { store, type RegionSelection } from '../lib/store.svelte'
  import Icon from './Icon.svelte'

  interface Props {
    region: RegionSelection
    onredraw: () => void
  }

  let { region, onredraw }: Props = $props()

  const read = $derived(region.read)
  const value = $derived(read?.value ?? null)
  const anchorText = $derived(
    read && read.suggestedRule.type === 'region' && read.suggestedRule.anchor ? read.suggestedRule.anchor.text : null,
  )
  const fixable = $derived(store.fixableRows.length)
  const all = $derived(store.testableRows.length)
  const isCurrentValue = $derived(
    value?.cents != null && store.activeRow?.cents === value.cents && store.activeRow?.path === region.path,
  )
</script>

<section class="result" class:ok={!!value} class:none={!!read && !value} aria-live="polite" aria-label="Resultado da área marcada">
  <button type="button" class="close btn btn-ghost btn-icon btn-sm" aria-label="Descartar a área marcada" onclick={() => store.clearRegion()}>
    <Icon name="x" size={14} />
  </button>

  {#if region.loading}
    <div class="line"><span class="spinner"></span> Lendo a área marcada…</div>
  {:else if region.error}
    <div class="line error"><Icon name="alertCircle" size={16} /> {region.error}</div>
    <div class="actions"><button type="button" class="btn btn-sm" onclick={onredraw}>Marcar de novo</button></div>
  {:else if read}
    {#if value && value.cents != null}
      <div class="line strong">
        <Icon name="checkCircle" size={16} />
        <span>Lido: <span class="amount num">{formatBRL(value.cents)}</span></span>
      </div>
      <p class="meta">
        {#if anchorText}
          Posição relativa ao texto “{anchorText}”, para achar o valor mesmo se o bloco mudar de lugar.
        {:else}
          Posição fixa na página (nenhum texto de referência por perto).
        {/if}
      </p>
    {:else}
      <div class="line strong warn">
        <Icon name="alert" size={16} />
        <span>Nenhum valor nesta área</span>
      </div>
      <p class="meta">
        {#if read.text}Texto encontrado: “{read.text}”.{:else}Não há texto dentro do retângulo.{/if}
        Desenhe em volta de um valor em R$.
      </p>
    {/if}

    <div class="actions">
      {#if !value}
        <button type="button" class="btn btn-sm btn-primary" onclick={onredraw}>Marcar de novo</button>
      {/if}
      <button
        type="button"
        class="btn btn-sm"
        class:btn-primary={!!value}
        disabled={fixable === 0}
        title={fixable ? 'Testar esta área nas notas em que o valor não foi encontrado' : 'Nenhuma nota com valor não encontrado'}
        onclick={() => store.startRuleTest('attention')}
      >
        Testar nas com erro ({formatInt(fixable)})
      </button>
      <button type="button" class="btn btn-sm" onclick={() => store.startRuleTest('all')}>
        Testar em todas ({formatInt(all)})
      </button>
      {#if value && value.cents != null && !isCurrentValue}
        <button type="button" class="btn btn-sm btn-ghost" onclick={() => store.applyRegionHere()}>Usar nesta nota</button>
      {/if}
      {#if value}
        <button type="button" class="btn btn-sm btn-ghost" onclick={onredraw}>Marcar de novo</button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .result {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 40px 12px var(--s-4);
    border-top: 1px solid var(--border);
    background: var(--surface);
    font-size: 13px;
  }

  .close {
    position: absolute;
    top: 8px;
    right: 8px;
    color: var(--text-3);
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--text-2);
  }

  .line.strong {
    font-size: 14px;
    font-weight: 500;
    color: var(--text);
  }

  .ok .line.strong :global(svg) {
    color: var(--ok);
  }

  .line.warn,
  .line.warn :global(svg) {
    color: var(--warn);
  }

  .line.error {
    color: var(--danger);
  }

  .amount {
    font-size: 15px;
    font-weight: 650;
  }

  .meta {
    color: var(--text-3);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
</style>

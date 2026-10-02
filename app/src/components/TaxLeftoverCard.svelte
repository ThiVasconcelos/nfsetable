<script lang="ts">
  import { formatBRL, formatMonthShort, formatRateInput, plural } from '../lib/format'
  import { activeIn, costValidity, isDated, monthlyCost, type CostItem, type LeftoverBase } from '../lib/tax.svelte'
  import type { LeftoverReport } from '../lib/types'
  import Icon from './Icon.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    leftover: LeftoverReport
    regimeLabel: string
    /** Base chosen by the user (the report says whether a custom revenue was used). */
    base: LeftoverBase
    costs: CostItem[]
    /** The last month considered: the typical month uses its costs (fixed in effect, variable of it). */
    referenceMonth: string | null
    reserveRate: number
    /** MEI: no pró-labore (no INSS and IRRF lines). */
    mei?: boolean
    /** Opens "Custos e reserva". */
    onedit: () => void
  }

  let { leftover: l, regimeLabel, base, costs, referenceMonth, reserveRate, mei = false, onedit }: Props = $props()

  const BASE_LABEL: Record<LeftoverBase, string> = {
    average: 'média dos meses',
    noBonus: 'média sem bônus',
    fixed: 'valor fixo',
  }

  /** The base actually used: without a custom revenue the report used the average. */
  const baseLabel = $derived(l.customRevenue ? BASE_LABEL[base] : BASE_LABEL.average)
  const items = $derived(costs.filter((c) => c.cents != null && c.cents > 0))
  /** Items with a start or end month: the card counts the ones in effect in the reference month. */
  const dated = $derived(items.some(isDated))
  const refLabel = $derived(referenceMonth ? formatMonthShort(referenceMonth, 'full') : null)
  const taxes = $derived(l.companyTaxesCents + l.proLaboreInssCents + l.proLaboreIrrfCents)
  const variable = $derived(l.variableCostsCents ?? 0)
  const parts = $derived.by(() => {
    const total = Math.max(1, l.revenueCents)
    return [
      { key: 'taxes', label: 'Impostos', cents: taxes },
      { key: 'costs', label: 'Custos', cents: l.costsCents + variable },
      { key: 'reserve', label: 'Reserva', cents: l.reserveCents },
      { key: 'left', label: 'Sobra', cents: Math.max(0, l.leftoverCents) },
    ]
      .filter((p) => p.cents > 0)
      .map((p) => ({ ...p, share: p.cents / total }))
  })
</script>

<TaxCard title="Sobra do mês" subtitle="{regimeLabel} · base: {baseLabel}" icon="coins">
  {#snippet aside()}
    <button type="button" class="link edit" onclick={onedit}>Ajustar</button>
  {/snippet}

  <div class="stack" role="img" aria-label="Faturamento dividido em impostos, custos, reserva e sobra">
    {#each parts as p (p.key)}
      <span class="seg {p.key}" style="flex-grow: {p.share}" title="{p.label}: {formatBRL(p.cents)}"></span>
    {/each}
  </div>
  <ul class="legend" aria-hidden="true">
    {#each parts as p (p.key)}
      <li><span class="key {p.key}"></span>{p.label}</li>
    {/each}
  </ul>

  <dl class="flow">
    <div>
      <dt>Faturamento ({baseLabel})</dt>
      <dd class="num">{formatBRL(l.revenueCents)}</dd>
    </div>
    <div class="minus" class:zero={!l.companyTaxesCents}>
      <dt>Impostos da empresa</dt>
      <dd class="num">− {formatBRL(l.companyTaxesCents)}</dd>
    </div>
    {#if !mei}
      <div class="minus" class:zero={!l.proLaboreInssCents}>
        <dt>INSS do pró-labore</dt>
        <dd class="num">− {formatBRL(l.proLaboreInssCents)}</dd>
      </div>
      <div class="minus" class:zero={!l.proLaboreIrrfCents}>
        <dt>IRRF do pró-labore</dt>
        <dd class="num">− {formatBRL(l.proLaboreIrrfCents)}</dd>
      </div>
    {/if}
    {#if items.length}
      <details class="minus costs">
        <summary>
          <span class="dt">
            Custos fixos
            <span class="count">
              ({dated && refLabel ? `vigentes em ${refLabel}` : plural(items.length, 'item', 'itens')})
            </span>
            <Icon name="chevronDown" size={12} class="chev" />
          </span>
          <span class="dd num">− {formatBRL(l.costsCents)}</span>
        </summary>
        <ul>
          {#each items as c (c.id)}
            {@const off = !!referenceMonth && isDated(c) && !activeIn(c, referenceMonth)}
            <li class:off title={off ? `Não vale em ${refLabel}: não entra na sobra` : undefined}>
              <span>
                {c.name || 'Custo'}{c.frequency === 'yearly' ? ' (anual ÷ 12)' : ''}{isDated(c) ? ` · ${costValidity(c)}` : ''}
              </span>
              <span class="num">{formatBRL(monthlyCost(c))}</span>
            </li>
          {/each}
        </ul>
      </details>
    {:else}
      <div class="minus" class:zero={!l.costsCents}>
        <dt>Custos fixos</dt>
        <dd class="num">− {formatBRL(l.costsCents)}</dd>
      </div>
    {/if}
    <div class="minus" class:zero={!variable}>
      <dt title="Os gastos com nota ou lançados no mês de referência. Os dos outros meses aparecem no Fluxo mês a mês.">
        Gastos variáveis {#if refLabel}<span class="count">(de {refLabel})</span>{/if}
      </dt>
      <dd class="num">− {formatBRL(variable)}</dd>
    </div>
    <div class="minus" class:zero={!l.reserveCents}>
      <dt>Reserva ({formatRateInput(reserveRate)}%)</dt>
      <dd class="num">− {formatBRL(l.reserveCents)}</dd>
    </div>
    <div class="total" class:negative={l.leftoverCents < 0}>
      <dt>Sobra</dt>
      <dd class="num">{formatBRL(l.leftoverCents)}</dd>
    </div>
  </dl>

  <p class="note">
    {mei ? 'No MEI, a sobra é o que fica para você depois do DAS, dos custos e da reserva.' : 'A sobra inclui o pró-labore líquido e o lucro.'}
    Sem contabilidade completa, dá para distribuir até
    <strong class="num">{formatBRL(l.taxFreeDistributionLimitCents)}</strong> por mês de lucro isento.
  </p>
</TaxCard>

<style>
  .edit {
    font-size: 12.5px;
  }

  .costs li.off {
    opacity: 0.55;
  }

  .costs li.off .num {
    text-decoration: line-through;
  }

  .stack {
    display: flex;
    gap: 2px;
    height: 12px;
  }

  .seg {
    flex-basis: 0;
    min-width: 4px;
  }

  .seg:first-child {
    border-radius: 4px 0 0 4px;
  }

  .seg:last-child {
    border-radius: 0 4px 4px 0;
  }

  .seg:only-child {
    border-radius: 4px;
  }

  .taxes {
    background: var(--viz-2);
  }

  .costs.key,
  .seg.costs {
    background: var(--viz-muted);
  }

  .reserve {
    background: repeating-linear-gradient(45deg, var(--viz-1) 0 1.6px, var(--viz-1-soft) 1.6px 4px);
  }

  .left {
    background: var(--viz-1);
  }

  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: -4px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--text-2);
  }

  .legend li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .key {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }

  .flow {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
    font-size: 13px;
  }

  .flow > div,
  .costs summary {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-3);
  }

  dt,
  .dt {
    color: var(--text-2);
  }

  dd,
  .dd {
    margin: 0;
    font-weight: 500;
    white-space: nowrap;
  }

  .minus dt,
  .minus dd,
  .minus .dt,
  .minus .dd {
    color: var(--text-2);
    font-weight: 400;
  }

  /* Zero lines stay in the formula, one step quieter. */
  .minus.zero dt,
  .minus.zero dd {
    color: var(--text-3);
  }

  .costs summary {
    cursor: pointer;
    list-style: none;
  }

  .costs summary::-webkit-details-marker {
    display: none;
  }

  .dt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .count {
    color: var(--text-3);
  }

  .costs :global(.chev) {
    color: var(--text-3);
    transition: transform 0.15s var(--ease);
  }

  .costs[open] :global(.chev) {
    transform: rotate(180deg);
  }

  .costs ul {
    margin: 4px 0 2px;
    padding: 6px 0 6px 12px;
    border-left: 2px solid var(--border);
    list-style: none;
    font-size: 12px;
    color: var(--text-3);
  }

  .costs li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }

  .total {
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .total dt {
    font-weight: 600;
    color: var(--text);
  }

  .total dd {
    font-size: 17px;
    font-weight: 650;
  }

  .total.negative dd {
    color: var(--danger);
  }

  .note {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  .note strong {
    font-weight: 600;
    color: var(--text-2);
  }
</style>

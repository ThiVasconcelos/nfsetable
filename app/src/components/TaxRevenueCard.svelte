<script lang="ts">
  import { formatBRL, formatInt, formatMonthRange, plural } from '../lib/format'
  import { tax } from '../lib/tax.svelte'
  import type { RevenueSummary } from '../lib/types'
  import RevenueChart from './RevenueChart.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    revenue: RevenueSummary
  }

  let { revenue }: Props = $props()

  /** Live average of the months considered (the report catches up after the debounce). */
  const liveAverage = $derived(tax.consideredMonths.length ? tax.averageCents : null)
  /** Counted over the report months, like the numbers of this card. */
  const forecastMonths = $derived(tax.forecastAmong(revenue.months))
  const range = $derived(
    revenue.months.length ? formatMonthRange(revenue.months[0].month, revenue.months[revenue.months.length - 1].month) : '',
  )
</script>

<TaxCard
  title="Faturamento"
  subtitle="{plural(revenue.monthsCount, 'mês', 'meses')} · {range}{forecastMonths
    ? ` · ${plural(forecastMonths, 'previsto', 'previstos')}`
    : ''}"
  icon="barChart"
>
  <div class="hero">
    <span class="value">{formatBRL(revenue.averageMonthlyCents)}</span>
    <span class="caption">por mês, em média: o <strong>mês típico</strong> usado nas estimativas</span>
  </div>

  <dl class="stats">
    <div>
      <dt>Total</dt>
      <dd class="num">{formatBRL(revenue.totalCents)}</dd>
    </div>
    <div>
      <dt>Meses</dt>
      <dd class="num">{formatInt(revenue.monthsCount)}</dd>
    </div>
    <div>
      <dt>
        RBT12
        {#if revenue.rbt12Annualized}
          <span class="pill accent" title="Menos de 12 meses: média × 12, a regra das empresas novas">anualizado</span>
        {/if}
      </dt>
      <dd class="num">{formatBRL(revenue.rbt12Cents)}</dd>
    </div>
  </dl>

  <RevenueChart months={tax.planMonths} average={liveAverage} ontoggle={(m) => tax.toggleMonth(m)} />
</TaxCard>

<style>
  .hero {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .value {
    font-size: 28px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }

  .caption {
    font-size: 12.5px;
    color: var(--text-3);
  }

  .caption strong {
    font-weight: 600;
    color: var(--text-2);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, auto));
    justify-content: start;
    gap: var(--s-2) var(--s-5);
    margin: 0;
  }

  .stats div {
    min-width: 0;
  }

  dt {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    font-size: 12px;
    color: var(--text-3);
  }

  dt .pill {
    height: 18px;
    padding: 0 6px;
    font-size: 11px;
  }

  dd {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }
</style>

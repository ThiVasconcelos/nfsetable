<script lang="ts">
  // "Receita, gastos e lucro": a small table of the months considered (report.cashFlow) on the
  // Resumo, for a sense of the company's health: revenue, taxes, costs (fixed + variable), the
  // operating profit and the gross pró-labore (not in the MEI) and what the company earned. The
  // full detail (reserve, leftover) is in "Fluxo mês a mês".
  import { formatAmount, formatBRL, formatMonthLong, formatMonthShort, formatPercent, plural } from '../lib/format'
  import type { CashFlowMonth, TaxRegime } from '../lib/types'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    flow: CashFlowMonth[]
    regime: TaxRegime
    /** Opens "Fluxo mês a mês". */
    onflow: () => void
  }

  let { flow, regime, onflow }: Props = $props()

  const mei = $derived(regime === 'mei')

  /** Lucro operacional: revenue − taxes − costs, what the company made before the pró-labore. */
  const operating = (m: CashFlowMonth) => m.profitCents + m.proLaboreCents

  const totals = $derived.by(() => {
    const t = { revenue: 0, taxes: 0, costs: 0, operating: 0, proLabore: 0, profit: 0 }
    for (const m of flow) {
      t.revenue += m.revenueCents
      t.taxes += m.companyTaxesCents
      t.costs += m.fixedCostsCents + m.variableCostsCents
      t.operating += operating(m)
      t.proLabore += m.proLaboreCents
      t.profit += m.profitCents
    }
    return t
  })
  const losses = $derived(flow.filter((m) => m.profitCents < 0).length)
  const annexV = $derived(flow.filter((m) => m.annex === 'V').length)
  /** Share of the revenue: the profit in the MEI, the operating profit in an ME. */
  const margin = $derived(totals.revenue > 0 ? (mei ? totals.profit : totals.operating) / totals.revenue : null)

</script>

<TaxCard
  id="tax-monthly-card"
  title="Receita, gastos e lucro"
  subtitle="Cada mês considerado: o que entrou, o que saiu e o que a empresa ganhou"
  icon="table"
  wide
>
  <p class="summary">
    {#if mei}
      Lucro de <strong class="num" class:negative={totals.profit < 0}>{formatBRL(totals.profit)}</strong> em
      {plural(flow.length, 'mês', 'meses')}{#if margin != null}, <span class="num">{formatPercent(margin)}</span> da receita{/if}.
    {:else}
      Lucro operacional de
      <strong class="num" class:negative={totals.operating < 0}>{formatBRL(totals.operating)}</strong> em
      {plural(flow.length, 'mês', 'meses')}{#if margin != null}, <span class="num">{formatPercent(margin)}</span> da receita{/if};
      depois do pró-labore, lucro de
      <strong class="num" class:negative={totals.profit < 0}>{formatBRL(totals.profit)}</strong>.
    {/if}
    {#if losses}
      <span class="loss">{plural(losses, 'mês no prejuízo', 'meses no prejuízo')}.</span>
    {/if}
    {#if annexV}
      <span class="warn">{plural(annexV, 'mês no Anexo V', 'meses no Anexo V')}.</span>
    {/if}
  </p>

  <div class="wrap">
    <table>
      <caption class="sr-only">Receita, gastos e lucro de cada mês, valores em reais</caption>
      <thead>
        <tr>
          <th scope="col" class="c-month">Mês</th>
          <th scope="col" class="c-num">Receita</th>
          <th scope="col" class="c-num" title="Impostos da empresa sobre a receita do mês">Impostos</th>
          <th scope="col" class="c-num" title="Custos fixos e gastos variáveis do mês">Gastos</th>
          {#if !mei}
            <th scope="col" class="c-num" title="Receita − impostos − gastos: o que a empresa gerou antes do pró-labore (lucro + pró-labore)">
              Lucro operacional
            </th>
            <th scope="col" class="c-num" title="Pró-labore bruto do mês">Pró-labore</th>
          {/if}
          <th scope="col" class="c-num" title="O que a empresa ganhou no mês">Lucro</th>
        </tr>
      </thead>
      <tbody>
        {#each flow as m (m.month)}
          <tr data-month={m.month} class:loss={m.profitCents < 0}>
            <th scope="row" class="c-month" title={formatMonthLong(m.month)}>
              {formatMonthShort(m.month, 'short')}
              {#if m.annex === 'V'}
                <span class="annex-v" title="Fator R abaixo de 28% neste mês: o DAS vai pelo Anexo V">
                  Anexo V<span class="sr-only">: Fator R abaixo de 28% neste mês, o DAS vai pelo Anexo V</span>
                </span>
              {/if}
            </th>
            <td class="c-num num">{formatAmount(m.revenueCents)}</td>
            <td class="c-num num">{formatAmount(m.companyTaxesCents)}</td>
            <td class="c-num num">{formatAmount(m.fixedCostsCents + m.variableCostsCents)}</td>
            {#if !mei}
              <td class="c-num num operating" class:negative={operating(m) < 0}>{formatAmount(operating(m))}</td>
              <td class="c-num num">{formatAmount(m.proLaboreCents)}</td>
            {/if}
            <td class="c-num num profit" class:negative={m.profitCents < 0}>{formatAmount(m.profitCents)}</td>
          </tr>
        {/each}
      </tbody>
      <tfoot>
        <tr>
          <th scope="row" class="c-month">Total</th>
          <td class="c-num num">{formatAmount(totals.revenue)}</td>
          <td class="c-num num">{formatAmount(totals.taxes)}</td>
          <td class="c-num num">{formatAmount(totals.costs)}</td>
          {#if !mei}
            <td class="c-num num operating" class:negative={totals.operating < 0}>{formatAmount(totals.operating)}</td>
            <td class="c-num num">{formatAmount(totals.proLabore)}</td>
          {/if}
          <td class="c-num num profit" class:negative={totals.profit < 0}>{formatAmount(totals.profit)}</td>
        </tr>
      </tfoot>
    </table>
  </div>

  <p class="foot">
    <span class="formula">
      {#if mei}
        Lucro = receita − impostos − gastos. Valores em R$.
      {:else}
        Lucro operacional = receita − impostos − gastos; lucro = lucro operacional − pró-labore. Valores em R$.
      {/if}
    </span>
    <button type="button" class="link" onclick={onflow}>Ver o fluxo completo (reserva e sobra)</button>
  </p>
</TaxCard>

<style>
  .summary {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .summary strong {
    font-weight: 650;
    color: var(--text);
  }

  .summary .negative,
  .summary .loss {
    color: var(--danger);
  }

  .summary .warn {
    color: var(--warn);
  }

  .wrap {
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 600px;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 12.5px;
  }

  th,
  td {
    padding: 7px 8px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
  }

  thead th {
    height: 30px;
    padding-top: 0;
    padding-bottom: 0;
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  th:first-child,
  td:first-child {
    padding-left: var(--s-4);
  }

  th:last-child,
  td:last-child {
    padding-right: var(--s-4);
  }

  .c-month {
    text-align: left;
    font-weight: 500;
  }

  tbody .c-month {
    color: var(--text);
  }

  .c-num,
  thead .c-num {
    text-align: right;
  }

  td.c-num {
    color: var(--text-2);
  }

  td.profit {
    color: var(--text);
    font-weight: 600;
  }

  td.profit.negative {
    color: var(--danger);
  }

  td.operating {
    color: var(--text);
    font-weight: 550;
  }

  td.operating.negative,
  tfoot td.operating.negative {
    color: var(--danger);
  }

  .annex-v {
    display: inline-block;
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 10.5px;
    font-weight: 600;
    line-height: 16px;
    vertical-align: 1px;
    cursor: help;
  }

  tfoot th,
  tfoot td {
    border-bottom: 0;
    padding-top: 9px;
    font-weight: 600;
    color: var(--text);
  }

  tfoot td.c-num {
    color: var(--text);
  }

  tfoot td.profit.negative {
    color: var(--danger);
  }

  .foot {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 4px var(--s-3);
    font-size: 12px;
    color: var(--text-3);
  }

  .foot .link {
    font-size: 12.5px;
  }
</style>

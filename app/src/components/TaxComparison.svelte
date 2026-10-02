<script lang="ts">
  import { formatBRL, formatPercent } from '../lib/format'
  import type { RegimeCost } from '../lib/types'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    comparison: RegimeCost[]
    /** `regime` key of the row for the current regime ("mei", "simplesIii", "simplesV"...). */
    current: string
  }

  let { comparison, current }: Props = $props()

  const cheapest = $derived.by(() => {
    const available = comparison.filter((c) => c.available)
    if (!available.length) return null
    return available.reduce((best, c) => (c.totalTaxesCents < best.totalTaxesCents ? c : best)).regime
  })
  const maxRate = $derived(Math.max(0.0001, ...comparison.map((c) => c.totalRate)))
</script>

<TaxCard
  title="Comparativo de regimes"
  subtitle="O mesmo mês típico em cada regime: impostos da empresa, do pró-labore e o que sobra"
  icon="scale"
  wide
>
  <div class="wrap">
    <table>
      <thead>
        <tr>
          <th class="c-regime">Regime</th>
          <th class="c-num">Impostos da empresa</th>
          <th class="c-num c-pl">Pró-labore</th>
          <th class="c-num c-owner">INSS + IRRF</th>
          <th class="c-num">Total de impostos</th>
          <th class="c-rate">% do faturamento</th>
          <th class="c-num">Sobra</th>
        </tr>
      </thead>
      <tbody>
        {#each comparison as c (c.regime)}
          {@const best = c.regime === cheapest}
          <tr class:off={!c.available} class:best>
            <td class="c-regime">
              <div class="regime">
                <span class="name">{c.label}</span>
                <span class="tags">
                  {#if c.regime === current}<span class="pill">Atual</span>{/if}
                  {#if best}<span class="pill accent">Mais barato</span>{/if}
                  {#if !c.available}<span class="pill">Indisponível</span>{/if}
                </span>
              </div>
              {#if c.note}<p class="note">{c.note}</p>{/if}
            </td>
            <td class="c-num num">{formatBRL(c.companyTaxesCents)}</td>
            <td class="c-num c-pl num">{c.proLaboreCents ? formatBRL(c.proLaboreCents) : '—'}</td>
            <td class="c-num c-owner num">{c.ownerTaxesCents ? formatBRL(c.ownerTaxesCents) : '—'}</td>
            <td class="c-num num strong">{formatBRL(c.totalTaxesCents)}</td>
            <td class="c-rate">
              <span class="rate">
                <span class="bar" aria-hidden="true"><span style="width: {(c.totalRate / maxRate) * 100}%"></span></span>
                <span class="num">{formatPercent(c.totalRate)}</span>
              </span>
            </td>
            <td class="c-num num strong">
              {formatBRL(c.ownerNetCents)}
              {#if c.reserveCents > 0}
                <span class="reserve" title="Já descontada a reserva de {formatBRL(c.reserveCents)} por mês">
                  reserva − {formatBRL(c.reserveCents)}
                </span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  <p class="foot">
    Sobra = faturamento − todos os impostos − custos do mês de referência (fixos e variáveis) − reserva (inclui o pró-labore líquido). Regimes
    indisponíveis aparecem em cinza, com o motivo.
  </p>
</TaxCard>

<style>
  .wrap {
    container: comparison / inline-size;
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 13px;
  }

  th {
    height: 32px;
    padding: 0 10px;
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    padding: 10px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  th:first-child,
  td:first-child {
    padding-left: var(--s-4);
  }

  th:last-child,
  td:last-child {
    padding-right: var(--s-4);
  }

  .c-num {
    text-align: right;
    white-space: nowrap;
  }

  td.c-num {
    padding-top: 12px;
  }

  .strong {
    font-weight: 600;
  }

  .reserve {
    display: block;
    margin-top: 2px;
    font-size: 11.5px;
    font-weight: 400;
    color: var(--text-3);
  }

  .c-regime {
    min-width: 180px;
  }

  .regime {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px 8px;
  }

  .name {
    font-weight: 600;
  }

  .tags {
    display: inline-flex;
    gap: 4px;
  }

  .tags .pill {
    height: 19px;
    padding: 0 7px;
    font-size: 11px;
  }

  .note {
    margin-top: 3px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-3);
  }

  tr.best td {
    background: var(--accent-soft);
  }

  tr.off td {
    color: var(--text-3);
  }

  tr.off .name,
  tr.off .strong {
    color: var(--text-3);
    font-weight: 500;
  }

  .c-rate {
    width: 150px;
  }

  td.c-rate {
    padding-top: 12px;
  }

  .rate {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .bar {
    flex: 1;
    height: 6px;
    border-radius: 999px;
    background: var(--viz-track);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    border-radius: 999px;
    background: var(--viz-1);
  }

  tr.off .bar span {
    background: var(--viz-muted);
  }

  .rate .num {
    min-width: 50px;
    text-align: right;
    white-space: nowrap;
  }

  .foot {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  @container comparison (max-width: 720px) {
    .c-pl,
    .c-owner {
      display: none;
    }

    .c-rate {
      width: 120px;
    }
  }

  @container comparison (max-width: 520px) {
    .bar {
      display: none;
    }

    .c-rate {
      width: auto;
    }
  }
</style>

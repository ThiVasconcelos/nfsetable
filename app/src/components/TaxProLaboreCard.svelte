<script lang="ts">
  // "Pró-labore" on the Resumo (not in the MEI): the amount per month, how it was set, INSS + IRRF
  // and the net, and, in the Simples with the Fator R, the annex it leads to. The breakdown and the
  // explanations are in the "Pró-labore" tab.
  import { formatBRL, formatPercent, plural } from '../lib/format'
  import type { CashFlowMonth, ProLaboreReport, SimplesActivity, SimplesReport, TaxRegime } from '../lib/types'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    proLabore: ProLaboreReport
    regime: TaxRegime
    activity: SimplesActivity
    simples: SimplesReport
    flow: CashFlowMonth[]
    /** Opens the "Pró-labore" tab. */
    ondetails: () => void
  }

  let { proLabore: pl, regime, activity, simples, flow, ondetails }: Props = $props()

  /**
   * The pró-labore in use: the report's, except in the Lucro Presumido, where the months show the one
   * it uses (one minimum wage when automatic: there is no Fator R).
   */
  const inUse = $derived.by(() => {
    const first = flow[0]
    if (regime === 'presumido' && first) {
      return { gross: first.proLaboreCents, owner: first.inssCents + first.irrfCents, net: first.proLaboreCents - first.inssCents - first.irrfCents }
    }
    return { gross: pl.grossCents, owner: pl.inssCents + pl.irrfCents, net: pl.netCents }
  })
  const fatorR = $derived(regime === 'simples' && activity === 'fatorR')
  const monthsV = $derived(flow.filter((m) => m.annex === 'V').length)
</script>

<TaxCard title="Pró-labore" subtitle="A retirada mensal do sócio" icon="wallet" highlight id="tax-prolabore-card">
  {#snippet aside()}
    <span class="pill accent">{pl.automatic ? 'Automático' : 'Valor fixo'}</span>
  {/snippet}

  <div class="hero">
    <span class="value num">{formatBRL(inUse.gross)}</span>
    <span class="caption">por mês, bruto · {pl.automatic ? 'automático' : 'valor fixo'}</span>
  </div>

  <dl class="lines">
    <div>
      <dt>INSS + IRRF</dt>
      <dd class="num">− {formatBRL(inUse.owner)}</dd>
    </div>
    <div class="total">
      <dt>Líquido para você</dt>
      <dd class="num">{formatBRL(inUse.net)}</dd>
    </div>
  </dl>

  {#if fatorR}
    <p class="fator" class:low={simples.annex === 'V'}>
      Fator R <strong class="num">{formatPercent(simples.fatorR)}</strong> → <strong>Anexo {simples.annex}</strong>
      {#if monthsV}
        <span class="months-v">· {plural(monthsV, 'mês no Anexo V', 'meses no Anexo V')}</span>
      {/if}
    </p>
  {/if}

  <button type="button" class="link details" onclick={ondetails}>Ver detalhes</button>
</TaxCard>

<style>
  .hero {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 2px 8px;
  }

  .value {
    font-size: 28px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.15;
    color: var(--text);
  }

  .caption {
    font-size: 12.5px;
    color: var(--text-3);
  }

  .lines {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    font-size: 13px;
  }

  .lines div {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s-3);
  }

  dt {
    color: var(--text-2);
  }

  dd {
    margin: 0;
    font-weight: 500;
    white-space: nowrap;
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
    font-size: 15px;
    font-weight: 650;
  }

  .fator {
    font-size: 12.5px;
    color: var(--text-2);
  }

  .fator strong {
    font-weight: 600;
    color: var(--text);
  }

  .fator.low strong,
  .months-v {
    color: var(--warn);
  }

  .details {
    align-self: flex-start;
    margin-top: auto;
    font-size: 12.5px;
  }
</style>

<script lang="ts">
  // "Pró-labore" tab (not in the MEI): what the pró-labore is, the one in use broken down (gross,
  // INSS, IRRF and its reduction, net), how the automatic amount was chosen and, month by month,
  // what it means: in the Simples each month's RBT12, Fator R, annex and the least pró-labore that
  // keeps it in Anexo III; in the Lucro Presumido the INSS and IRRF of each month.
  import { formatAmount, formatBRL, formatMonthLong, formatMonthShort, formatPercent, plural } from '../lib/format'
  import type { CashFlowMonth, ProLaboreReport, SimplesActivity, TaxRegime } from '../lib/types'
  import Icon from './Icon.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    proLabore: ProLaboreReport
    regime: TaxRegime
    activity: SimplesActivity
    flow: CashFlowMonth[]
    /** "Outras despesas com folha" (the automatic amount discounts it). */
    payrollCents: number
  }

  let { proLabore: pl, regime, activity, flow, payrollCents }: Props = $props()

  const simples = $derived(regime === 'simples')
  const fatorR = $derived(simples && activity === 'fatorR')

  /**
   * The pró-labore in use. In the Lucro Presumido with "Automático" it is one minimum wage (no
   * Fator R): the months show it; elsewhere the report's breakdown.
   */
  const inUse = $derived.by(() => {
    const first = flow[0]
    if (regime === 'presumido' && pl.automatic && first) {
      const net = first.proLaboreCents - first.inssCents - first.irrfCents
      return { gross: first.proLaboreCents, inss: first.inssCents, irrf: first.irrfCents, reduction: 0, net }
    }
    return { gross: pl.grossCents, inss: pl.inssCents, irrf: pl.irrfCents, reduction: pl.irrfReductionCents, net: pl.netCents }
  })
  const discounts = $derived(inUse.inss + inUse.irrf)
  const netShare = $derived(inUse.gross > 0 ? Math.max(0, inUse.net) / inUse.gross : 1)

  /** 28% of the basis month (RBT12 ÷ 12), minus the other payroll. */
  const plain = $derived(Math.round(pl.basisAverageCents * 0.28) - payrollCents)
  /** The automatic amount hit the floor (one minimum wage). */
  const floored = $derived(pl.automatic && !pl.basisMonth && pl.grossCents > plain + 1)
  const monthsV = $derived(flow.filter((m) => m.annex === 'V').length)
  const basisText = $derived(
    `RBT12 ÷ 12 = ${formatBRL(pl.basisAverageCents)}, ${plural(pl.basisMonths, 'mês', 'meses')}`,
  )

  const totals = $derived.by(() => {
    const t = { revenue: 0, proLabore: 0, inss: 0, irrf: 0 }
    for (const m of flow) {
      t.revenue += m.revenueCents
      t.proLabore += m.proLaboreCents
      t.inss += m.inssCents
      t.irrf += m.irrfCents
    }
    return t
  })

  /** "Ir para as configurações": the Pró-labore section of the sidebar (scrolled there only). */
  function toSettings() {
    const heading = document.getElementById('tax-owner')
    const side = heading?.closest<HTMLElement>('.settings')
    if (heading && side) {
      side.scrollTo({ top: side.scrollTop + heading.getBoundingClientRect().top - side.getBoundingClientRect().top - 12, behavior: 'smooth' })
    }
    document.querySelector<HTMLInputElement>('input[name="tax-prolabore"]:checked')?.focus({ preventScroll: true })
  }
</script>

<div class="tab" id="tax-prolabore-tab">
  <TaxCard title="Como funciona" subtitle="O pró-labore em poucas palavras" icon="info" wide>
    <ul class="explain">
      <li><strong>Pró-labore</strong> é a retirada mensal do sócio, como um salário.</li>
      <li><strong>INSS:</strong> 11% do pró-labore, até o teto do INSS, descontados do sócio.</li>
      <li><strong>IRRF:</strong> pela tabela mensal, já com a redução de 2026 (Lei 15.270/2025).</li>
      {#if fatorR}
        <li>
          <strong>Fator R:</strong> folha de 12 meses (pró-labore e salários) ÷ receita de 12 meses. Com 28% ou mais, o DAS
          vai pelo <strong>Anexo III</strong> (desde 6%); abaixo disso, pelo <strong>Anexo V</strong> (desde 15,5%).
        </li>
      {:else if simples}
        <li>Esta atividade é sempre do <strong>Anexo III</strong>: o pró-labore não muda o anexo.</li>
      {:else}
        <li>
          No <strong>Lucro Presumido</strong> não há Fator R; a empresa paga ainda 20% de INSS patronal (CPP) sobre o
          pró-labore, já nos impostos.
        </li>
      {/if}
    </ul>
    <p class="pointer">
      <Icon name="settings" size={14} />
      <span>
        O valor (automático ou fixo), as outras despesas com folha e os dependentes ficam nas configurações, ao lado.
        <button type="button" class="link" onclick={toSettings}>Ir para as configurações</button>
      </span>
    </p>
  </TaxCard>

  <div class="pair">
    <TaxCard title="O pró-labore em uso" subtitle="Bruto, descontos e o que sobra para você" icon="wallet" highlight id="tax-prolabore-breakdown">
      {#snippet aside()}
        <span class="pill accent">{pl.automatic ? 'Automático' : 'Valor fixo'}</span>
      {/snippet}
      <div class="hero">
        <span class="value num">{formatBRL(inUse.gross)}</span>
        <span class="caption">por mês, bruto</span>
      </div>
      <div class="split" role="img" aria-label="Líquido {formatBRL(inUse.net)} e descontos {formatBRL(discounts)}">
        <span class="seg net" style="flex-grow: {netShare}"></span>
        {#if discounts > 0}<span class="seg cut" style="flex-grow: {1 - netShare}"></span>{/if}
      </div>
      <dl class="lines">
        <div>
          <dt><span class="key cut" aria-hidden="true"></span>INSS (11%)</dt>
          <dd class="num">− {formatBRL(inUse.inss)}</dd>
        </div>
        <div>
          <dt><span class="key cut" aria-hidden="true"></span>IRRF</dt>
          <dd class="num">− {formatBRL(inUse.irrf)}</dd>
        </div>
        {#if inUse.reduction > 0}
          <div class="sub">
            <dt>{inUse.irrf === 0 ? 'Isento pela' : 'Já com a'} redução da Lei 15.270/2025</dt>
            <dd class="num">{formatBRL(inUse.reduction)} a menos</dd>
          </div>
        {/if}
        <div class="total">
          <dt><span class="key net" aria-hidden="true"></span>Líquido para você</dt>
          <dd class="num">{formatBRL(inUse.net)}</dd>
        </div>
      </dl>
    </TaxCard>

    <TaxCard title="Como o valor foi escolhido" subtitle={pl.automatic ? 'Automático' : 'Valor fixo'} icon="target" id="tax-prolabore-basis">
      <p class="basis">
        {#if !pl.automatic}
          Valor fixo informado por você.
          {#if fatorR}
            {#if monthsV}
              <span class="warn">
                Em {plural(monthsV, 'mês', 'meses')} ele fica abaixo do mínimo para o Anexo III: o DAS desses meses vai pelo Anexo V
                (veja a tabela).
              </span>
            {:else}
              Com ele, todos os meses considerados ficam no Anexo III.
            {/if}
          {/if}
        {:else if regime === 'presumido'}
          No Lucro Presumido o automático é um salário mínimo: não há Fator R a atingir.
        {:else if pl.basisMonth}
          Automático: o menor valor que deixa todos os meses considerados no Anexo III. O mês mais exigente é
          <strong>{formatMonthShort(pl.basisMonth, 'short')}</strong> ({basisText}): 28% disso{payrollCents > 0
            ? `, menos ${formatBRL(payrollCents)} de outras despesas com folha`
            : ''}.
        {:else if floored}
          Um salário mínimo: 28% do mês típico ({basisText}) daria {formatBRL(Math.max(0, plain))}.
        {:else}
          Automático: 28% do mês típico ({basisText}){payrollCents > 0
            ? `, menos ${formatBRL(payrollCents)} de outras despesas com folha`
            : ''}. Com ele, todos os meses considerados ficam no Anexo III.
        {/if}
      </p>
      {#if pl.automatic && regime !== 'presumido'}
        <p class="basis-note">
          O RBT12 de cada mês é a receita dos 12 meses antes dele que o app conhece (proporcional quando são menos de 12).
        </p>
      {/if}
    </TaxCard>
  </div>

  <TaxCard
    title="Mês a mês"
    subtitle={simples ? 'RBT12, Fator R e anexo de cada mês, com o pró-labore em uso' : 'O pró-labore de cada mês, com INSS e IRRF'}
    icon="calendar"
    wide
    id="tax-prolabore-months"
  >
    {#if fatorR && monthsV}
      <p class="alert">
        <Icon name="alert" size={14} />
        <span>
          {plural(monthsV, 'mês ficou', 'meses ficaram')} no <strong>Anexo V</strong>: o pró-labore está abaixo do mínimo
          para o Anexo III nesses meses.
        </span>
      </p>
    {/if}
    <div class="wrap">
      <table class:simples>
        <caption class="sr-only">Pró-labore mês a mês, valores em reais</caption>
        <thead>
          <tr>
            <th scope="col" class="c-month">Mês</th>
            <th scope="col" class="c-num">Receita</th>
            {#if simples}
              <th scope="col" class="c-num" title="Receita dos 12 meses anteriores">RBT12</th>
              <th scope="col" class="c-num">Fator R</th>
              <th scope="col" class="c-annex">Anexo</th>
              {#if fatorR}<th scope="col" class="c-num c-min">Mínimo p/ Anexo III</th>{/if}
              <th scope="col" class="c-num">Pró-labore</th>
              <th scope="col" class="c-num">INSS + IRRF</th>
            {:else}
              <th scope="col" class="c-num">Pró-labore</th>
              <th scope="col" class="c-num">INSS</th>
              <th scope="col" class="c-num">IRRF</th>
              <th scope="col" class="c-num">Líquido</th>
            {/if}
          </tr>
        </thead>
        <tbody>
          {#each flow as m (m.month)}
            {@const missing = (m.proLaboreForAnnexIiiCents ?? 0) - m.proLaboreCents}
            <tr data-month={m.month} class:v={m.annex === 'V'}>
              <th scope="row" class="c-month" title={formatMonthLong(m.month)}>{formatMonthShort(m.month, 'short')}</th>
              <td class="c-num num">{formatAmount(m.revenueCents)}</td>
              {#if simples}
                <td class="c-num num">{formatAmount(m.rbt12Cents ?? 0)}</td>
                <td class="c-num num" class:low={m.annex === 'V'}>{formatPercent(m.fatorR ?? 0)}</td>
                <td class="c-annex"><span class="annex" class:v={m.annex === 'V'}>{m.annex ?? '—'}</span></td>
                {#if fatorR}<td class="c-num num">{formatAmount(m.proLaboreForAnnexIiiCents ?? 0)}</td>{/if}
                <td class="c-num num">
                  {formatAmount(m.proLaboreCents)}
                  {#if fatorR && missing > 0}<span class="missing">faltam {formatBRL(missing)}</span>{/if}
                </td>
                <td class="c-num num">{formatAmount(m.inssCents + m.irrfCents)}</td>
              {:else}
                <td class="c-num num">{formatAmount(m.proLaboreCents)}</td>
                <td class="c-num num">{formatAmount(m.inssCents)}</td>
                <td class="c-num num">{formatAmount(m.irrfCents)}</td>
                <td class="c-num num">{formatAmount(m.proLaboreCents - m.inssCents - m.irrfCents)}</td>
              {/if}
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row" class="c-month">Total</th>
            <td class="c-num num">{formatAmount(totals.revenue)}</td>
            {#if simples}
              <td></td>
              <td></td>
              <td></td>
              {#if fatorR}<td></td>{/if}
              <td class="c-num num">{formatAmount(totals.proLabore)}</td>
              <td class="c-num num">{formatAmount(totals.inss + totals.irrf)}</td>
            {:else}
              <td class="c-num num">{formatAmount(totals.proLabore)}</td>
              <td class="c-num num">{formatAmount(totals.inss)}</td>
              <td class="c-num num">{formatAmount(totals.irrf)}</td>
              <td class="c-num num">{formatAmount(totals.proLabore - totals.inss - totals.irrf)}</td>
            {/if}
          </tr>
        </tfoot>
      </table>
    </div>
    <p class="note">
      Valores em R$.
      {#if fatorR}
        “Mínimo p/ Anexo III”: o menor pró-labore que deixa o mês no Anexo III (28% do RBT12 ÷ 12, menos as outras
        despesas com folha).
      {/if}
    </p>
  </TaxCard>
</div>

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    min-width: 0;
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s-3);
  }

  @container results (max-width: 640px) {
    .pair {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  .explain {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .explain strong {
    font-weight: 600;
    color: var(--text);
  }

  .pointer {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-3);
  }

  .pointer :global(svg) {
    flex: none;
    margin-top: 3px;
    color: var(--text-3);
  }

  .pointer .link {
    font-size: inherit;
  }

  /* ------------------------------------------------------------ breakdown */

  .hero {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 2px 8px;
  }

  .value {
    font-size: 26px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.15;
    color: var(--text);
  }

  .caption {
    font-size: 12.5px;
    color: var(--text-3);
  }

  .split {
    display: flex;
    gap: 2px;
    height: 10px;
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

  .net {
    background: var(--viz-1);
  }

  .cut {
    background: var(--viz-2);
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

  .lines dt {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--text-2);
  }

  .lines dd {
    margin: 0;
    font-weight: 500;
    white-space: nowrap;
  }

  .key {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex: none;
  }

  .sub {
    margin-top: -4px;
    padding-left: 18px;
    font-size: 12px;
  }

  .sub dt,
  .sub dd {
    color: var(--ok) !important;
    font-weight: 400 !important;
  }

  .lines .total {
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }

  .lines .total dt {
    font-weight: 600;
    color: var(--text);
  }

  .lines .total dd {
    font-size: 15px;
    font-weight: 650;
  }

  .basis {
    font-size: 13px;
    line-height: 1.55;
    color: var(--text-2);
  }

  .basis strong {
    font-weight: 600;
    color: var(--text);
  }

  .basis .warn {
    display: block;
    margin-top: 4px;
    color: var(--warn);
  }

  .basis-note {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ month by month */

  .alert {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .alert :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .alert strong {
    font-weight: 600;
  }

  .wrap {
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  table {
    width: 100%;
    min-width: 520px;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 12.5px;
  }

  table.simples {
    min-width: 640px;
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

  thead .c-min {
    white-space: normal;
    line-height: 1.2;
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

  td.low {
    color: var(--warn);
  }

  .c-annex {
    text-align: center;
  }

  thead .c-annex {
    text-align: center;
  }

  .annex {
    display: inline-block;
    min-width: 30px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent-soft-2);
    color: var(--accent-text);
    font-size: 11px;
    font-weight: 650;
    line-height: 18px;
    text-align: center;
  }

  .annex.v {
    background: var(--warn-soft);
    color: var(--warn);
  }

  tr.v th,
  tr.v td {
    background: var(--warn-soft);
  }

  .missing {
    display: block;
    font-size: 11px;
    color: var(--warn);
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

  .note {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }
</style>

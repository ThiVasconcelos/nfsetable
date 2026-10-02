<script lang="ts">
  import { formatBRL, formatPercent, formatBRLCompact } from '../lib/format'
  import { taxHint, taxName } from '../lib/labels'
  import type { SimplesActivity, SimplesReport } from '../lib/types'
  import Icon from './Icon.svelte'
  import Meter from './Meter.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    simples: SimplesReport
    activity: SimplesActivity
    /** Year of the reference month (for the ME limit). */
    year: string
    /** The pró-labore is automatic (then it already is the one for the Annex III). */
    automatic: boolean
  }

  let { simples: s, activity, year, automatic }: Props = $props()

  const ME_LIMIT = 36_000_000
  const tier = $derived(s.meLimitRemainingCents > 0 ? 'ME' : 'EPP')
  const annexIii = $derived(s.annex === 'III')
  const fatorMax = $derived(Math.min(1, Math.max(0.4, s.fatorR * 1.15)))
  const meUsed = $derived(Math.max(0, ME_LIMIT - s.meLimitRemainingCents))
  const meRatio = $derived(meUsed / ME_LIMIT)
</script>

<TaxCard
  title="Simples Nacional ({tier})"
  subtitle={tier === 'ME' ? 'Microempresa: faturamento de até R$ 360 mil por ano' : 'Empresa de pequeno porte: acima de R$ 360 mil por ano'}
  icon="building"
>
  {#snippet aside()}
    <span class="pill {annexIii ? 'ok' : 'warn'}">
      <Icon name={annexIii ? 'check' : 'alert'} size={12} strokeWidth={2.2} />
      Anexo {s.annex}
    </span>
  {/snippet}

  <div class="fator">
    <div class="fator-head">
      <span class="label">Fator R</span>
      <span class="fator-value num">{formatPercent(s.fatorR)}</span>
      <span class="goal">mínimo para o Anexo III: 28%</span>
    </div>
    <Meter
      value={s.fatorR}
      max={fatorMax}
      tone={s.fatorR >= 0.28 || activity === 'annexIii' ? 'accent' : 'warn'}
      marks={[{ at: 0.28, label: '28%: Anexo III' }]}
      label="Fator R"
      valueText="{formatPercent(s.fatorR)}, mínimo de 28% para o Anexo III"
    />
    <p class="caption">
      {#if activity === 'annexIii'}
        Esta atividade é sempre do Anexo III: o Fator R não muda o anexo.
      {:else if s.fatorR >= 0.28}
        Folha (pró-labore e salários) de 28% ou mais do faturamento: Anexo III.
      {:else}
        Abaixo de 28%: Anexo V. Com pró-labore de {formatBRL(s.proLaboreForAnnexIiiCents)} a empresa vai para o Anexo III.
      {/if}
    </p>
  </div>

  <dl class="grid">
    <div class="das">
      <dt>DAS por mês</dt>
      <dd class="num">{formatBRL(s.dasCents)}</dd>
    </div>
    <div>
      <dt>Alíquota efetiva</dt>
      <dd class="num">{formatPercent(s.effectiveRate)} <span class="sub">nominal {formatPercent(s.nominalRate)}</span></dd>
    </div>
    <div>
      <dt>Faixa</dt>
      <dd class="num">{s.bracket}ª faixa</dd>
    </div>
    {#if activity === 'fatorR' && !automatic}
      <div>
        <dt>Pró-labore do Anexo III</dt>
        <dd class="num">{formatBRL(s.proLaboreForAnnexIiiCents)}</dd>
      </div>
    {:else}
      <div>
        <dt>DAS em 12 meses</dt>
        <dd class="num">{formatBRL(s.dasCents * 12)}</dd>
      </div>
    {/if}
  </dl>

  {#if s.split.length}
    <div class="split">
      <span class="label">Repartição do DAS</span>
      <ul>
        {#each s.split as share (share.tax)}
          <li title={taxHint(share.tax)}>
            <span>{taxName(share.tax)}</span>
            <span class="num">{formatBRL(share.cents)}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="limit">
    <span class="label">Limite da ME: R$ 360 mil por ano</span>
    <Meter
      value={meRatio}
      max={1}
      tone={meRatio >= 1 ? 'danger' : meRatio >= 0.8 ? 'warn' : 'accent'}
      label="Limite da microempresa"
      valueText="{formatBRLCompact(meUsed)} de R$ 360 mil faturados em {year}"
    />
    <p class="limit-foot num">
      <span>Faturado em {year}: {formatBRL(meUsed)}</span>
      <span class:over={s.meLimitRemainingCents === 0}>
        {s.meLimitRemainingCents > 0 ? `restam ${formatBRL(s.meLimitRemainingCents)}` : 'limite atingido: EPP'}
      </span>
    </p>
  </div>
</TaxCard>

<style>
  .label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }

  .fator {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .fator-head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 4px 10px;
  }

  .fator-value {
    font-size: 22px;
    font-weight: 650;
    letter-spacing: -0.015em;
    line-height: 1.2;
    color: var(--text);
  }

  .goal {
    margin-left: auto;
    font-size: 12px;
    color: var(--text-3);
  }

  .caption {
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-2);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px var(--s-4);
    margin: 0;
    padding: 10px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  dt {
    font-size: 12px;
    color: var(--text-3);
  }

  dd {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    white-space: nowrap;
  }

  .das dd {
    font-size: 18px;
    font-weight: 650;
  }

  .sub {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-3);
  }

  .split ul {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 2px var(--s-4);
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
  }

  .split li {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    color: var(--text-2);
  }

  .split li .num {
    color: var(--text);
  }

  .limit {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .limit-foot {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 2px 12px;
    font-size: 12px;
    color: var(--text-3);
  }

  .limit-foot span:last-child {
    font-weight: 500;
    color: var(--text-2);
  }

  .limit-foot .over {
    color: var(--danger);
  }
</style>

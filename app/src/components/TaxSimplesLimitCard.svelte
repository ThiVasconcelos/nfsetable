<script lang="ts">
  import { formatBRL, formatBRLCompact, formatPercent } from '../lib/format'
  import type { RevenueSummary, SimplesReport } from '../lib/types'
  import Icon from './Icon.svelte'
  import Meter from './Meter.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    revenue: RevenueSummary
    simples: SimplesReport
    year: string
  }

  let { revenue, simples: s, year }: Props = $props()

  type Outlook = 'me' | 'toEpp' | 'epp' | 'aboveCeiling'

  const ytd = $derived(revenue.yearToDateCents)
  const projection = $derived(revenue.yearProjectionCents)
  // Up to the ME limit the meters measure against it; once the year passed it, the company is an
  // EPP and they measure against the ceiling of the Simples.
  const epp = $derived(ytd > s.meLimitCents)
  const limit = $derived(epp ? s.eppLimitCents : s.meLimitCents)
  const limitName = $derived(epp ? 'do teto' : 'do limite da ME')
  const usedRatio = $derived(limit ? ytd / limit : 0)
  const projectionRatio = $derived(limit ? projection / limit : 0)
  /** Both meters share one scale, with room past the limit. */
  const scale = $derived(Math.min(3, Math.max(1.3, usedRatio, projectionRatio) * 1.05))
  const marks = $derived([{ at: 1, label: epp ? 'Teto do Simples' : 'Limite da ME' }])

  const outlook: Outlook = $derived(
    projection > s.eppLimitCents ? 'aboveCeiling' : epp ? 'epp' : projection > s.meLimitCents ? 'toEpp' : 'me',
  )
  const PILL: Record<Outlook, 'ok' | 'warn' | 'danger'> = { me: 'ok', toEpp: 'warn', epp: 'ok', aboveCeiling: 'danger' }
  const TONE: Record<Outlook, 'accent' | 'warn' | 'danger'> = { me: 'accent', toEpp: 'warn', epp: 'accent', aboveCeiling: 'danger' }
  const pillLabel = $derived(
    {
      me: 'Dentro do limite da ME',
      toEpp: 'Projeção acima do limite da ME',
      epp: 'EPP, dentro do teto',
      aboveCeiling: ytd > s.eppLimitCents ? 'Acima do teto do Simples' : 'Projeção acima do teto do Simples',
    }[outlook],
  )
  const caption = $derived(
    {
      me: 'No ritmo atual, o ano fecha dentro do limite da ME.',
      toEpp: 'Se o ritmo continuar, passa do limite da ME e a empresa passa a ser EPP. O DAS segue as mesmas tabelas do Simples.',
      epp: `A receita do ano passou do limite da ME: a empresa passa a ser EPP, e o Simples Nacional vale até ${formatBRLCompact(s.eppLimitCents)} por ano.`,
      aboveCeiling:
        ytd > s.eppLimitCents
          ? 'A receita do ano passou do teto do Simples Nacional: a empresa não pode continuar no Simples.'
          : 'Se o ritmo continuar, a receita do ano passa do teto do Simples Nacional, e a empresa não poderia continuar no Simples.',
    }[outlook],
  )
</script>

<TaxCard
  title="Limite do Simples"
  subtitle="ME até {formatBRLCompact(s.meLimitCents)} e EPP até {formatBRLCompact(s.eppLimitCents)} por ano"
  icon="target"
>
  {#snippet aside()}
    <span class="pill {PILL[outlook]}">
      <Icon name={PILL[outlook] === 'ok' ? 'check' : 'alert'} size={12} strokeWidth={2.2} />
      {pillLabel}
    </span>
  {/snippet}

  <div class="meter-block">
    <div class="row">
      <span class="label">Faturado em {year}</span>
      <span class="num"><strong>{formatBRL(ytd)}</strong> · {formatPercent(usedRatio)} {limitName}</span>
    </div>
    <Meter
      value={usedRatio}
      max={scale}
      tone={ytd > limit ? 'danger' : 'accent'}
      {marks}
      label="Faturado no ano em relação ao limite {limitName}"
      valueText="{formatPercent(usedRatio)} {limitName}"
    />
  </div>

  <div class="meter-block">
    <div class="row">
      <span class="label">Projeção do ano, no ritmo atual</span>
      <span class="num"><strong>{formatBRL(projection)}</strong> · {formatPercent(projectionRatio)}</span>
    </div>
    <Meter
      value={projectionRatio}
      max={scale}
      tone={TONE[outlook]}
      {marks}
      label="Projeção do ano em relação ao limite {limitName}"
      valueText="{formatPercent(projectionRatio)} {limitName}, {pillLabel}"
    />
    <p class="caption" class:warn={outlook !== 'me' && outlook !== 'epp'}>
      {#if outlook !== 'me' && outlook !== 'epp'}<Icon name="alert" size={13} />{/if}
      {caption}
    </p>
  </div>

  <dl class="grid">
    <div>
      <dt>Limite da ME</dt>
      <dd class="num">{formatBRL(s.meLimitCents)}</dd>
    </div>
    <div>
      <dt>Teto do Simples (EPP)</dt>
      <dd class="num">{formatBRL(s.eppLimitCents)}</dd>
    </div>
  </dl>
</TaxCard>

<style>
  .meter-block {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 2px 8px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .row strong {
    font-weight: 600;
    color: var(--text);
  }

  .label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-3);
  }

  .caption {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  .caption.warn {
    color: var(--warn);
  }

  .caption :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px var(--s-4);
    margin: 0;
    padding-top: 10px;
    border-top: 1px solid var(--border);
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
</style>

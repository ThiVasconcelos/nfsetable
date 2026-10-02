<script lang="ts">
  import { formatBRL, formatPercent } from '../lib/format'
  import { MEI_PROJECTED_HINT, MEI_PROJECTED_LABEL, MEI_STATUS_HINT, MEI_STATUS_LABEL } from '../lib/labels'
  import type { CnaeInfo, MeiReport, MeiStatus } from '../lib/types'
  import Icon from './Icon.svelte'
  import Meter from './Meter.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    mei: MeiReport
    cnae: CnaeInfo | null
    year: string
    /** The opening month falls in the reference year (proportional limit). */
    openingYear: boolean
  }

  let { mei, cnae, year, openingYear }: Props = $props()

  const TONE: Record<MeiStatus, 'accent' | 'warn' | 'danger'> = {
    within: 'accent',
    upToTolerance: 'warn',
    aboveTolerance: 'danger',
  }
  const PILL: Record<MeiStatus, string> = { within: 'ok', upToTolerance: 'warn', aboveTolerance: 'danger' }

  /** Both meters share one scale, with room past the limit and the 20% tolerance. */
  const scale = $derived(Math.min(3, Math.max(1.3, mei.usedRatio, mei.projectionRatio) * 1.05))
  const marks = $derived([
    { at: 1, label: 'Limite' },
    { at: 1.2, label: 'Limite + 20% de tolerância' },
  ])
  const allowed = $derived(cnae ? cnae.meiAllowed : mei.itServicesAllowed)

  // The pill tells where the year is heading, not only where it stands: the projection is never
  // below the year to date, so a projection in a worse band wins, worded as a projection.
  const projectedOnly = $derived(mei.projectedStatus !== mei.status)
  const pillLabel = $derived((projectedOnly ? MEI_PROJECTED_LABEL : MEI_STATUS_LABEL)[mei.projectedStatus])
  const outlookHint = $derived((projectedOnly ? MEI_PROJECTED_HINT : MEI_STATUS_HINT)[mei.projectedStatus])
</script>

<TaxCard title="MEI" subtitle="Microempreendedor individual" icon="user">
  {#snippet aside()}
    <span class="pill {PILL[mei.projectedStatus]}">
      <Icon name={mei.projectedStatus === 'within' ? 'check' : 'alert'} size={12} strokeWidth={2.2} />
      {pillLabel}
    </span>
  {/snippet}

  {#if !allowed}
    <div class="notice warn">
      <Icon name="alert" size={15} />
      <span>
        <strong>TI não é permitido ao MEI.</strong>
        {cnae
          ? `A atividade ${cnae.code} não está entre as ocupações do MEI.`
          : 'Desenvolvimento de software e serviços de TI ficam de fora.'}
      </span>
    </div>
  {:else if cnae?.meiOccupation}
    <div class="notice ok">
      <Icon name="checkCircle" size={15} />
      <span>Permitido ao MEI como {cnae.meiOccupation}.</span>
    </div>
  {/if}

  <div class="meter-block">
    <div class="row">
      <span class="label">Faturado em {year}</span>
      <span class="num"><strong>{formatBRL(mei.yearToDateCents)}</strong> · {formatPercent(mei.usedRatio)} do limite</span>
    </div>
    <Meter
      value={mei.usedRatio}
      max={scale}
      tone={TONE[mei.status]}
      {marks}
      label="Faturado no ano em relação ao limite do MEI"
      valueText="{formatPercent(mei.usedRatio)} do limite"
    />
  </div>

  <div class="meter-block">
    <div class="row">
      <span class="label">Projeção do ano, no ritmo atual</span>
      <span class="num"><strong>{formatBRL(mei.yearProjectionCents)}</strong> · {formatPercent(mei.projectionRatio)}</span>
    </div>
    <Meter
      value={mei.projectionRatio}
      max={scale}
      tone={TONE[mei.projectedStatus]}
      {marks}
      label="Projeção do ano em relação ao limite do MEI"
      valueText="{formatPercent(mei.projectionRatio)} do limite, {MEI_STATUS_LABEL[mei.projectedStatus]}"
    />
    <p class="caption" class:warn={mei.projectedStatus !== 'within'}>
      {#if mei.projectedStatus !== 'within'}<Icon name="alert" size={13} />{/if}
      {outlookHint}
    </p>
  </div>

  <dl class="grid">
    <div>
      <dt>Limite em {year}</dt>
      <dd class="num">{formatBRL(mei.limitCents)}{#if openingYear}<span class="sub"> proporcional</span>{/if}</dd>
    </div>
    <div>
      <dt>DAS-MEI</dt>
      <dd class="num">{formatBRL(mei.dasCents)} <span class="sub">por mês</span></dd>
    </div>
  </dl>
</TaxCard>

<style>
  .notice {
    display: flex;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .notice :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .notice.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .notice.ok {
    background: var(--ok-soft);
    color: var(--ok);
  }

  .notice strong {
    font-weight: 600;
  }

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

  .sub {
    font-size: 12px;
    font-weight: 400;
    color: var(--text-3);
  }
</style>

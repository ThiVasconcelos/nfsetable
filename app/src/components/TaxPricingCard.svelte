<script lang="ts">
  import { formatBRL } from '../lib/format'
  import type { PricingReport } from '../lib/types'
  import Icon from './Icon.svelte'
  import TaxCard from './TaxCard.svelte'

  interface Props {
    pricing: PricingReport | null
    /** Average monthly revenue today (the typical month). */
    averageCents: number
  }

  let { pricing, averageCents }: Props = $props()

  const gap = $derived(pricing ? pricing.requiredRevenueCents - averageCents : 0)

  function focusGoal() {
    const input = document.getElementById('tax-desired')
    input?.scrollIntoView({ block: 'center', behavior: 'smooth' })
    input?.focus({ preventScroll: true })
  }
</script>

<TaxCard title="Quanto cobrar" subtitle="O faturamento para sobrar o que você quer, no regime atual" icon="target">
  {#if pricing}
    <p class="lead">Para tirar <strong class="num">{formatBRL(pricing.desiredNetCents)}</strong> por mês, fature cerca de</p>
    <div class="hero">
      <span class="value">{formatBRL(pricing.requiredRevenueCents)}</span>
      <span class="caption">por mês</span>
    </div>
    <p class="compare" class:ok={gap <= 0}>
      <Icon name={gap <= 0 ? 'checkCircle' : 'info'} size={14} />
      {#if gap <= 0}
        O seu mês típico ({formatBRL(averageCents)}) já chega lá.
      {:else}
        Faltam {formatBRL(gap)} por mês em relação ao seu mês típico ({formatBRL(averageCents)}).
      {/if}
    </p>
    <p class="note">{pricing.note}</p>
    <button type="button" class="link change" onclick={focusGoal}>Mudar o valor desejado</button>
  {:else}
    <p class="empty">Diga quanto quer tirar por mês (depois dos impostos e dos custos) e veja quanto precisa faturar.</p>
    <button type="button" class="btn btn-sm" onclick={focusGoal}>
      <Icon name="target" size={14} />
      Informar quanto quero tirar
    </button>
  {/if}
</TaxCard>

<style>
  .lead {
    font-size: 13px;
    color: var(--text-2);
  }

  .lead strong {
    font-weight: 600;
    color: var(--text);
  }

  .hero {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: -6px;
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

  .compare {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-2);
  }

  .compare :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--accent-text);
  }

  .compare.ok :global(svg) {
    color: var(--ok);
  }

  .note {
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  .change {
    align-self: flex-start;
    font-size: 12.5px;
  }

  .empty {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .btn {
    align-self: flex-start;
  }
</style>

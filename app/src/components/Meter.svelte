<script lang="ts">
  // A ratio against a limit: the fill carries the severity (accent -> warn -> danger), the track is a
  // lighter step of the accent, and optional marks show thresholds (e.g. 100% of a limit, 28%).

  interface Mark {
    at: number
    label: string
  }

  interface Props {
    /** Current value on the same scale as `max` (e.g. 0.58 for 58%). */
    value: number
    max: number
    tone?: 'accent' | 'warn' | 'danger' | 'ok'
    marks?: Mark[]
    /** Accessible name and value text (the visible texts live next to the meter). */
    label: string
    valueText: string
    /** A lighter segment after the fill, e.g. a projection (same scale). */
    ghost?: number | null
  }

  let { value, max, tone = 'accent', marks = [], label, valueText, ghost = null }: Props = $props()

  const pct = (v: number) => `${Math.max(0, Math.min(100, (v / max) * 100))}%`
</script>

<div
  class="meter"
  role="meter"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={max}
  aria-valuenow={Math.min(value, max)}
  aria-valuetext={valueText}
>
  <div class="track">
    {#if ghost != null && ghost > value}
      <div class="ghost {tone}" style="width: {pct(ghost)}"></div>
    {/if}
    <div class="fill {tone}" style="width: {pct(value)}"></div>
  </div>
  {#each marks as mark (mark.at)}
    <span class="mark" style="left: {pct(mark.at)}" title={mark.label}></span>
  {/each}
</div>

<style>
  .meter {
    position: relative;
    padding: 3px 0;
  }

  .track {
    position: relative;
    height: 8px;
    border-radius: 999px;
    background: var(--viz-track);
    overflow: hidden;
  }

  .fill,
  .ghost {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 999px;
    transition: width 0.35s var(--ease);
  }

  .fill.accent {
    background: var(--viz-1);
  }

  .fill.ok {
    background: var(--ok);
  }

  .fill.warn {
    background: var(--viz-warn);
  }

  .fill.danger {
    background: var(--viz-danger);
  }

  .ghost {
    opacity: 0.35;
  }

  .ghost.accent {
    background: var(--viz-1);
  }

  .ghost.ok {
    background: var(--ok);
  }

  .ghost.warn {
    background: var(--viz-warn);
  }

  .ghost.danger {
    background: var(--viz-danger);
  }

  .mark {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    margin-left: -1px;
    border-radius: 1px;
    background: var(--text-2);
  }
</style>

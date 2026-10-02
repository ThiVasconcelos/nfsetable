<script lang="ts">
  import { STATUS_LABEL, STATUS_SHORT, type RowStatus } from '../lib/labels'

  interface Props {
    status: RowStatus
    title?: string
  }

  let { status, title }: Props = $props()
</script>

<span class="badge {status}" class:has-short={STATUS_SHORT[status] !== STATUS_LABEL[status]} {title}>
  {#if status === 'pending'}
    <span class="spinner small" aria-hidden="true"></span>
  {:else}
    <span class="dot" aria-hidden="true"></span>
  {/if}
  <span class="full">{STATUS_LABEL[status]}</span>
  {#if STATUS_SHORT[status] !== STATUS_LABEL[status]}
    <span class="short" aria-hidden="true">{STATUS_SHORT[status]}</span>
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    line-height: 1;
  }

  /* The table swaps to the short label when it is narrow (see DocTable). */
  .short {
    display: none;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
    flex: none;
  }

  .small {
    width: 10px;
    height: 10px;
    border-width: 1.5px;
  }

  .ok {
    color: var(--ok);
    background: var(--ok-soft);
  }

  .notFound,
  .noText {
    color: var(--warn);
    background: var(--warn-soft);
  }

  .error {
    color: var(--danger);
    background: var(--danger-soft);
  }

  .duplicate {
    color: var(--muted-badge);
    background: var(--muted-soft);
  }

  .pending {
    color: var(--accent-text);
    background: var(--accent-soft);
  }
</style>

<script lang="ts">
  import type { Snippet } from 'svelte'
  import Icon from './Icon.svelte'
  import type { IconName } from './icons'

  interface Props {
    title: string
    subtitle?: string
    icon?: IconName
    /** Stands out (accent border and header). */
    highlight?: boolean
    /** Spans the whole row of the grid. */
    wide?: boolean
    /** Right side of the header (a pill, a link...). */
    aside?: Snippet
    children: Snippet
    id?: string
  }

  let { title, subtitle, icon, highlight = false, wide = false, aside, children, id }: Props = $props()

  const uid = $props.id()
  const titleId = `tax-card-${uid}`
</script>

<section class="card" class:highlight class:wide {id} aria-labelledby={titleId}>
  <header class="head">
    {#if icon}
      <span class="icon" aria-hidden="true"><Icon name={icon} size={16} /></span>
    {/if}
    <div class="titles">
      <h3 id={titleId}>{title}</h3>
      {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
    </div>
    {#if aside}<div class="aside">{@render aside()}</div>{/if}
  </header>
  <div class="body">
    {@render children()}
  </div>
</section>

<style>
  .card {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .card.wide {
    grid-column: 1 / -1;
  }

  .card.highlight {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 14px var(--s-4) 0;
  }

  .icon {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-top: -2px;
    border-radius: 8px;
    background: var(--surface-2);
    color: var(--text-2);
    flex: none;
  }

  .highlight .icon {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .titles {
    flex: 1;
    min-width: 0;
  }

  h3 {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }

  .subtitle {
    margin-top: 1px;
    font-size: 12px;
    color: var(--text-3);
  }

  .aside {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4) var(--s-4);
    min-width: 0;
  }
</style>

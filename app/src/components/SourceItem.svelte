<script lang="ts">
  import { isPdfPath, plural, shortenPath } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import type { Source, SourceInfo } from '../lib/types'
  import Icon from './Icon.svelte'

  interface Props {
    source: Source
    info: SourceInfo | null
  }

  let { source, info }: Props = $props()

  // Before the first scan answers, guess from the extension.
  const isDir = $derived(info ? info.isDir : !isPdfPath(source.path))
  const missing = $derived(!!info && !info.exists)
  const uid = $props.id()
  const switchId = `sub-${uid}`

  const meta = $derived.by(() => {
    if (!info) return store.phase === 'idle' ? '' : 'Procurando…'
    if (!info.exists) return isDir ? 'Pasta não encontrada' : 'Arquivo não encontrado'
    return plural(info.fileCount, 'PDF', 'PDFs')
  })
  const unreadable = $derived(info?.unreadable ?? [])
</script>

<li class="source" class:missing>
  <span class="icon" aria-hidden="true">
    <Icon name={isDir ? 'folder' : 'file'} size={16} />
  </span>
  <div class="main">
    <span class="path" title={source.path}>{shortenPath(source.path)}</span>
    {#if meta}
      <span class="meta num">
        {#if missing}<Icon name="alert" size={12} />{/if}
        {meta}
      </span>
    {/if}
    {#if unreadable.length}
      <span class="meta unreadable" title={unreadable.join('\n')}>
        <Icon name="alert" size={12} />
        {plural(unreadable.length, 'pasta não pôde ser lida', 'pastas não puderam ser lidas')} (sem permissão?)
      </span>
    {/if}
  </div>
  {#if isDir && !missing}
    <div class="sub">
      <label class="toggle" for={switchId}>
        <input
          id={switchId}
          type="checkbox"
          role="switch"
          checked={source.recursive}
          onchange={(e) => store.setRecursive(source.path, e.currentTarget.checked)}
        />
        <span class="track" aria-hidden="true"><span class="thumb"></span></span>
        Incluir subpastas
      </label>
      {#if info && info.exists && info.fileCount === 0 && !source.recursive}
        <span class="hint">Nenhum PDF aqui. As notas estão em subpastas?</span>
      {/if}
    </div>
  {/if}
  <button
    type="button"
    class="btn btn-ghost btn-icon btn-sm remove"
    title="Remover fonte"
    aria-label="Remover a fonte {source.path}"
    onclick={() => store.removeSource(source.path)}
  >
    <Icon name="x" size={14} />
  </button>
</li>

<style>
  .meta.unreadable {
    color: var(--warn);
  }

  .source {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) 28px;
    grid-template-areas:
      'icon main remove'
      '.    sub  sub';
    column-gap: var(--s-2);
    align-items: start;
    padding: 10px 6px 10px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }

  .icon {
    grid-area: icon;
  }

  .main {
    grid-area: main;
  }

  .sub {
    grid-area: sub;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .remove {
    grid-area: remove;
  }

  .source.missing {
    border-style: dashed;
  }

  .icon {
    padding-top: 1px;
    color: var(--text-3);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .path {
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-3);
  }

  .missing .meta {
    color: var(--warn);
  }

  .toggle {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    margin-top: 6px;
    font-size: 12.5px;
    color: var(--text-2);
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
  }

  .toggle input {
    position: absolute;
    z-index: 1;
    opacity: 0;
    width: 28px;
    height: 16px;
    margin: 0;
    cursor: pointer;
  }

  .track {
    position: relative;
    flex: none;
    width: 28px;
    height: 16px;
    border-radius: 999px;
    background: var(--surface-3);
    box-shadow: inset 0 0 0 1px var(--border-strong);
    transition: background-color 0.15s var(--ease);
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transition: transform 0.15s var(--ease);
  }

  .toggle input:checked + .track {
    background: var(--accent);
    box-shadow: none;
  }

  .toggle input:checked + .track .thumb {
    transform: translateX(12px);
  }

  .toggle input:focus-visible + .track {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }

  .hint {
    margin-top: 4px;
    font-size: 12px;
    color: var(--warn);
  }

  .remove {
    color: var(--text-3);
  }

  .remove:hover {
    color: var(--text);
  }
</style>

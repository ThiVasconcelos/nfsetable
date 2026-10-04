<script lang="ts">
  import { untrack } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { baseName, formatBRL, formatInt, plural } from '../lib/format'
  import { describeRule } from '../lib/labels'
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  interface Item {
    path: string
    name: string
    current: number | null
    next: number | null
    error: string | null
    applicable: boolean
    same: boolean
  }

  /** Rows shown at a time: a test over thousands of files lists the new values first. */
  const PAGE = 200

  const test = $derived(store.ruleTest)
  const chosen = new SvelteSet<string>()
  let shown = $state(PAGE)
  let saveProfile = $state(false)
  let profileName = $state('')
  let initializedSeq = -1
  let nameInput: HTMLInputElement | undefined = $state()

  const items = $derived.by((): Item[] => {
    if (!test) return []
    const results = new Map(test.results.map((r) => [r.path, r]))
    const rows = new Map(store.rows.map((r) => [r.path, r]))
    const list = test.paths.map((path) => {
      const row = rows.get(path)
      const result = results.get(path)
      const next = result?.value?.cents ?? null
      const current = row?.cents ?? null
      return {
        path,
        name: row?.name ?? baseName(path),
        current,
        next,
        error: result?.error ?? null,
        applicable: next != null && next !== current,
        same: next != null && next === current,
      }
    })
    // New values first, then unchanged ones, then files without value (stable within groups).
    const rank = (i: Item) => (i.applicable ? 0 : i.same ? 1 : i.error ? 3 : 2)
    return list.map((item, index) => ({ item, index }))
      .sort((a, b) => rank(a.item) - rank(b.item) || a.index - b.index)
      .map((x) => x.item)
  })

  const found = $derived(items.filter((i) => i.next != null).length)
  const applicable = $derived(items.filter((i) => i.applicable))
  const allChosen = $derived(applicable.length > 0 && applicable.every((i) => chosen.has(i.path)))
  const someChosen = $derived(!allChosen && applicable.some((i) => chosen.has(i.path)))
  const nameMissing = $derived(saveProfile && !profileName.trim())
  const canApply = $derived(!!test && !test.running && !test.saving && !nameMissing && (chosen.size > 0 || saveProfile))

  // When the results of a new test arrive: preselect every file that gets a new value. A closed
  // test keeps nothing.
  $effect(() => {
    if (!test) {
      untrack(() => chosen.clear())
      return
    }
    if (test.running || test.seq === initializedSeq) return
    initializedSeq = test.seq
    untrack(() => {
      shown = PAGE
      chosen.clear()
      for (const i of items) if (i.applicable) chosen.add(i.path)
      saveProfile = false
      profileName = store.suggestProfileName(test.sourcePath)
    })
  })

  function toggleAll(on: boolean) {
    for (const i of applicable) {
      if (on) chosen.add(i.path)
      else chosen.delete(i.path)
    }
  }

  async function apply() {
    if (!canApply) return
    await store.applyRuleTest([...chosen], saveProfile ? profileName.trim() : null)
  }

  const scopeText = $derived(
    test
      ? test.scope === 'attention'
        ? `nas ${plural(test.paths.length, 'nota', 'notas')} com erro`
        : `em todas as ${plural(test.paths.length, 'nota', 'notas')}`
      : '',
  )
</script>

<Modal open={!!test} title="Testar a área marcada" subtitle={test ? `${describeRule(test.rule)} · ${scopeText}` : ''} size="lg" onclose={() => store.closeRuleTest()}>
  {#if test}
    {#if test.running}
      <div class="running">
        <div class="bar" role="progressbar" aria-label="Testando" aria-valuemin={0} aria-valuemax={test.progress.total} aria-valuenow={test.progress.done}>
          <div style="width: {test.progress.total ? (test.progress.done / test.progress.total) * 100 : 0}%"></div>
        </div>
        <span class="num">Testando {formatInt(test.progress.done)} de {formatInt(test.progress.total)}…</span>
      </div>
    {:else}
      <div class="summary">
        <span class="big num"><strong>{formatInt(found)} de {formatInt(items.length)}</strong> encontrados</span>
        {#if applicable.length}
          <span class="pill new">{plural(applicable.length, 'valor novo', 'valores novos')}</span>
        {/if}
        {#if found < items.length}
          <span class="pill miss">{plural(items.length - found, 'sem valor', 'sem valor')}</span>
        {/if}
      </div>

      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th class="c-check">
                <input
                  type="checkbox"
                  aria-label="Aplicar em todos os valores novos"
                  checked={allChosen}
                  indeterminate={someChosen}
                  disabled={!applicable.length}
                  onchange={(e) => toggleAll(e.currentTarget.checked)}
                />
              </th>
              <th>Arquivo</th>
              <th class="c-num">Atual</th>
              <th class="c-num">Novo</th>
            </tr>
          </thead>
          <tbody>
            {#each items.slice(0, shown) as item (item.path)}
              <tr class:off={!item.applicable}>
                <td class="c-check">
                  <input
                    type="checkbox"
                    aria-label="Aplicar em {item.name}"
                    disabled={!item.applicable}
                    checked={chosen.has(item.path)}
                    onchange={() => (chosen.has(item.path) ? chosen.delete(item.path) : chosen.add(item.path))}
                  />
                </td>
                <td class="file" title={item.path}>{item.name}</td>
                <td class="c-num num" class:dim={item.current == null}>
                  {item.current != null ? formatBRL(item.current) : '—'}
                </td>
                <td class="c-num num">
                  {#if item.next != null}
                    <span class:new={item.applicable} class:same={item.same}>{formatBRL(item.next)}</span>
                    {#if item.same}<span class="tag">igual</span>{/if}
                  {:else if item.error}
                    <span class="err" title={item.error}><Icon name="alertCircle" size={13} /> Erro</span>
                  {:else}
                    <span class="dim">Não encontrado</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if items.length > shown}
          <button type="button" class="btn btn-sm more" onclick={() => (shown += PAGE)}>
            Mostrar mais {formatInt(Math.min(PAGE, items.length - shown))} de {formatInt(items.length - shown)}
          </button>
        {/if}
      </div>
    {/if}

    {#if test.error}
      <p class="error" role="alert"><Icon name="alertCircle" size={15} /> {test.error}</p>
    {/if}
  {/if}

  {#snippet footer()}
    <label class="save">
      <input
        type="checkbox"
        bind:checked={saveProfile}
        onchange={() => {
          if (saveProfile) queueMicrotask(() => nameInput?.focus())
        }}
      />
      Salvar como perfil
    </label>
    <input
      bind:this={nameInput}
      class="input name"
      type="text"
      placeholder="Nome do perfil"
      aria-label="Nome do perfil"
      maxlength="80"
      disabled={!saveProfile}
      aria-invalid={nameMissing}
      bind:value={profileName}
    />
    <span class="spacer"></span>
    <button type="button" class="btn" onclick={() => store.closeRuleTest()}>Cancelar</button>
    <button type="button" class="btn btn-primary" disabled={!canApply} onclick={apply}>
      {#if test?.saving}<span class="spinner"></span>{/if}
      Aplicar{chosen.size ? ` (${formatInt(chosen.size)})` : ''}
    </button>
  {/snippet}
</Modal>

<style>
  .more {
    display: block;
    margin: var(--s-2) auto;
  }

  .running {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    padding: var(--s-6) 0;
    font-size: 13px;
    color: var(--text-2);
  }

  .bar {
    height: 6px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar div {
    height: 100%;
    background: var(--accent);
    border-radius: 999px;
    transition: width 0.2s var(--ease);
  }

  .summary {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    flex-wrap: wrap;
    margin-bottom: var(--s-3);
  }

  .big {
    font-size: 15px;
    color: var(--text-2);
    margin-right: var(--s-1);
  }

  .big strong {
    color: var(--text);
    font-weight: 650;
  }

  .pill {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 500;
  }

  .pill.new {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .pill.miss {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .table-wrap {
    max-height: 340px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 13px;
  }

  th {
    position: sticky;
    top: 0;
    z-index: 1;
    height: 34px;
    padding: 0 var(--s-3);
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
  }

  td {
    height: 38px;
    padding: 0 var(--s-3);
    border-bottom: 1px solid var(--border);
  }

  tbody tr:last-child td {
    border-bottom: 0;
  }

  .c-check {
    width: 40px;
    text-align: center;
    padding-right: 0;
  }

  .c-num {
    width: 150px;
    text-align: right;
    white-space: nowrap;
  }

  .file {
    max-width: 0;
    width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 500;
  }

  tr.off .file {
    color: var(--text-2);
    font-weight: 400;
  }

  .dim {
    color: var(--text-3);
  }

  .new {
    color: var(--accent-text);
    font-weight: 600;
  }

  .same {
    color: var(--text-2);
  }

  .tag {
    margin-left: 6px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 11px;
    color: var(--text-3);
  }

  .err {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--danger);
  }

  .error {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    margin-top: var(--s-3);
    padding: var(--s-2) var(--s-3);
    border-radius: var(--radius);
    background: var(--danger-soft);
    color: var(--danger);
    font-size: 13px;
  }

  .save {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    font-size: 13px;
    white-space: nowrap;
    cursor: pointer;
  }

  .name {
    width: 220px;
    min-width: 120px;
    flex: 0 1 220px;
  }

  .spacer {
    flex: 1;
  }
</style>

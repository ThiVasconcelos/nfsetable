<script lang="ts">
  import { tick } from 'svelte'
  import { formatBRL, formatMonthLong, formatMonthShort, plural } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import { periodLabel } from '../lib/period'
  import { NOTE_MAX, tax, type DraftRow, type MonthPreset, type PlanMonth } from '../lib/tax.svelte'
  import Icon from './Icon.svelte'
  import MoneyField from './MoneyField.svelte'
  import MonthInput from './MonthInput.svelte'
  import TaxCard from './TaxCard.svelte'

  const PRESETS: { id: Exclude<MonthPreset, 'custom' | 'range'>; label: string; hint: string }[] = [
    { id: 'all', label: 'Todos', hint: 'Todos os meses da lista' },
    { id: 'thisYear', label: 'Ano atual', hint: `Os meses de ${tax.currentYear}` },
    { id: 'last12', label: 'Últimos 12 meses', hint: 'Os 12 meses até o mês atual' },
    { id: 'next', label: 'Próximos meses', hint: 'Do mês atual em diante' },
  ]

  /** A period chosen elsewhere (Resumo) or by ticking months: shown next to the quick presets. */
  const otherPeriod = $derived(PRESETS.some((p) => p.id === tax.plan.preset) ? null : periodLabel(tax.period))

  /** Month row flashed after "Ir para o mês" (a duplicate typed in a new row). */
  let flashed = $state<string | null>(null)
  let flashTimer: ReturnType<typeof setTimeout> | undefined

  const months = $derived(tax.planMonths)
  const drafts = $derived(tax.plan.drafts)
  const considered = $derived(tax.consideredMonths)
  const total = $derived(considered.reduce((sum, m) => sum + (m.cents ?? 0), 0))
  const emptyForecasts = $derived(
    tax.plan.projections.filter((p) => p.cents == null && !tax.revenue.byMonth.has(p.month)).length,
  )
  const missing = $derived(tax.revenue.missing)


  async function focusForecast(month: string) {
    await tick()
    document.getElementById(`forecast-${month}`)?.focus()
  }

  /** "Adicionar linha": an empty row at the end, with the focus in its month. */
  async function addRow() {
    const id = tax.addDraft()
    await tick()
    document.getElementById(`draft-month-${id}`)?.focus()
  }

  /** A month typed in a new row: a valid new month turns it into a projection (focus on its value). */
  function onDraftMonth(draft: DraftRow, month: string | null, how: 'enter' | 'blur' | 'pick') {
    const promoted = tax.updateDraft(draft.id, { month })
    if (promoted && how !== 'blur') void focusForecast(promoted)
  }

  function draftHint(d: DraftRow): 'month' | 'duplicate' | null {
    if (d.month == null) return 'month'
    return tax.monthKeySet.has(d.month) ? 'duplicate' : null
  }

  /** Scrolls to a month already in the list and flashes it. */
  async function goToMonth(month: string) {
    const row = document.querySelector<HTMLElement>(`#tax-panel tr[data-month="${month}"]`)
    row?.scrollIntoView({ block: 'center', behavior: 'smooth' })
    flashed = month
    clearTimeout(flashTimer)
    flashTimer = setTimeout(() => (flashed = null), 1800)
    await tick()
    document.getElementById(`forecast-${month}`)?.focus({ preventScroll: true })
  }

  function addNext(count: number) {
    const added = tax.addNextMonths(count)
    if (!added.length) store.toast('Esses meses já estão na lista.')
    else void focusForecast(added[0])
  }

  function fillAverage() {
    const filled = tax.fillEmptyWithAverage()
    if (filled) store.toast(`${plural(filled, 'previsto preenchido', 'previstos preenchidos')} com a média das notas.`, 'success')
  }

  function rowHint(m: PlanMonth): string | null {
    if (m.cents != null || !m.included) return null
    return m.source === 'forecast' ? 'Preencha o previsto para este mês contar.' : 'Sem notas neste mês.'
  }
</script>

{#if missing.length}
  <div class="notice warn">
    <Icon name="calendar" size={16} />
    <span>
      <strong>{plural(missing.length, 'nota sem competência', 'notas sem competência')}</strong>
      não {missing.length === 1 ? 'entra' : 'entram'} em nenhum mês.
    </span>
    <button type="button" class="btn btn-sm" onclick={() => store.reviewCompetence(missing.map((r) => r.path))}>
      Preencher competência
    </button>
  </div>
{/if}

<TaxCard
  title="Meses do cálculo"
  subtitle="O faturamento de cada mês vem das notas ou do valor previsto. Os meses incluídos formam o mês típico."
  icon="calendar"
  wide
>
  <div class="toolbar">
    <div class="presets" role="group" aria-label="Seleção rápida de meses">
      {#each PRESETS as p (p.id)}
        {@const count = tax.presetMonths(p.id).length}
        <button
          type="button"
          class:active={tax.plan.preset === p.id}
          aria-pressed={tax.plan.preset === p.id}
          disabled={!count}
          title={count ? `${p.hint} (${plural(count, 'mês', 'meses')})` : `${p.hint}: nenhum na lista`}
          onclick={() => tax.setPreset(p.id)}
        >
          {p.label}
        </button>
      {/each}
      {#if otherPeriod}
        <span
          class="current"
          title={tax.plan.preset === 'custom' ? 'Meses marcados um a um em “Incluir”' : 'Período escolhido no Resumo'}
        >
          <Icon name={tax.plan.preset === 'custom' ? 'check' : 'calendar'} size={12} />
          {otherPeriod}
        </span>
      {/if}
    </div>
    <div class="add">
      <button type="button" class="btn btn-sm" onclick={addRow}>
        <Icon name="plus" size={14} />
        Adicionar linha
      </button>
      <div class="next" role="group" aria-label="Adicionar os próximos meses">
        <span class="next-label">Próximos</span>
        {#each [3, 6, 12] as n (n)}
          <button
            type="button"
            title="Adicionar os próximos {n} meses"
            aria-label="Adicionar os próximos {n} meses"
            onclick={() => addNext(n)}>{n}</button
          >
        {/each}
        <span class="next-label">meses</span>
      </div>
    </div>
  </div>

  {#if months.length || drafts.length}
    <div class="wrap">
      <table>
        <thead>
          <tr>
            <th class="c-month">Mês</th>
            <th class="c-notes">Notas</th>
            <th class="c-forecast">Previsto</th>
            <th class="c-use">Usar</th>
            <th class="c-inc">Incluir</th>
            <th class="c-del"><span class="sr-only">Remover</span></th>
          </tr>
        </thead>
        <tbody>
          {#each months as m (m.month)}
            {@const hint = rowHint(m)}
            <tr class:off={!m.included} class:empty={!!hint} class:flash={flashed === m.month} data-month={m.month}>
              <th class="c-month" scope="row">
                <span class="month">{formatMonthShort(m.month, 'full')}</span>
                {#if m.future}<span class="tag">futuro</span>{/if}
              </th>
              <td class="c-notes num">
                {#if m.notesCents != null}
                  <span class="value" class:chosen={m.source === 'notes'}>{formatBRL(m.notesCents)}</span>
                  <span class="sub">
                    {plural(m.notesCount, 'nota', 'notas')}{#if m.bonusCount}<span
                        title="Notas do tipo Bônus: {formatBRL(m.bonusCents)}">{` · ${plural(m.bonusCount, 'bônus', 'bônus')}`}</span
                      >{/if}
                  </span>
                {:else}
                  <span class="none">—</span>
                {/if}
              </td>
              <td class="c-forecast">
                <div class="forecast">
                  <div class="amount" class:chosen={m.source === 'forecast'}>
                    <MoneyField
                      id="forecast-{m.month}"
                      label="Previsto para {formatMonthLong(m.month)}"
                      placeholder="—"
                      allowEmpty
                      value={m.projection?.cents ?? null}
                      onchange={(v) => tax.setProjection(m.month, { cents: v })}
                    />
                  </div>
                  <input
                    class="input note"
                    type="text"
                    placeholder="Nota (opcional)"
                    aria-label="Nota do previsto de {formatMonthLong(m.month)}"
                    maxlength={NOTE_MAX}
                    value={m.projection?.note ?? ''}
                    onchange={(e) => tax.setProjection(m.month, { note: e.currentTarget.value.trim() })}
                  />
                </div>
                {#if hint}<span class="hint"><Icon name="alert" size={12} /> {hint}</span>{/if}
              </td>
              <td class="c-use">
                <div class="use" role="group" aria-label="Usar em {formatMonthLong(m.month)}">
                  <button
                    type="button"
                    class:active={m.source === 'notes'}
                    aria-pressed={m.source === 'notes'}
                    disabled={m.notesCents == null && m.source !== 'notes'}
                    title={m.notesCents == null ? 'Sem notas neste mês' : 'Usar a soma das notas'}
                    onclick={() => tax.setSource(m.month, 'notes')}>Notas</button
                  >
                  <button
                    type="button"
                    class:active={m.source === 'forecast'}
                    aria-pressed={m.source === 'forecast'}
                    title="Usar o valor previsto"
                    onclick={() => tax.setSource(m.month, 'forecast')}>Previsto</button
                  >
                </div>
              </td>
              <td class="c-inc">
                <input
                  type="checkbox"
                  aria-label="Incluir {formatMonthLong(m.month)} no cálculo"
                  checked={m.included}
                  onchange={() => tax.toggleMonth(m.month)}
                />
              </td>
              <td class="c-del">
                {#if m.projection}
                  <button
                    type="button"
                    class="del"
                    title={m.notesCents == null ? 'Remover o mês previsto' : 'Limpar o previsto'}
                    aria-label="Remover o previsto de {formatMonthLong(m.month)}"
                    onclick={() => tax.removeProjection(m.month)}
                  >
                    <Icon name="x" size={14} />
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
          {#each drafts as d (d.id)}
            {@const problem = draftHint(d)}
            <tr class="draft">
              <th class="c-month" scope="row">
                <MonthInput
                  id="draft-month-{d.id}"
                  size="sm"
                  label="Mês da nova linha (MM/AAAA)"
                  value={d.month}
                  taken={tax.monthKeySet}
                  error={problem === 'duplicate' ? 'Esse mês já está na lista' : null}
                  errorText={false}
                  onchange={(month, how) => onDraftMonth(d, month, how)}
                />
              </th>
              <td class="c-notes num"><span class="none">—</span></td>
              <td class="c-forecast">
                <div class="forecast">
                  <div class="amount">
                    <MoneyField
                      id="draft-value-{d.id}"
                      label="Previsto da nova linha"
                      placeholder="—"
                      allowEmpty
                      value={d.cents}
                      onchange={(v) => tax.updateDraft(d.id, { cents: v })}
                    />
                  </div>
                  <input
                    class="input note"
                    type="text"
                    placeholder="Nota (opcional)"
                    aria-label="Nota da nova linha"
                    maxlength={NOTE_MAX}
                    value={d.note}
                    onchange={(e) => tax.updateDraft(d.id, { note: e.currentTarget.value.trim() })}
                  />
                </div>
                {#if problem === 'duplicate' && d.month}
                  {@const month = d.month}
                  <span class="hint">
                    <Icon name="alert" size={12} /> Esse mês já está na lista.
                    <button type="button" class="link" onclick={() => goToMonth(month)}>Ir para o mês</button>
                  </span>
                {:else}
                  <span class="hint"><Icon name="alert" size={12} /> Informe o mês (MM/AAAA) para a linha contar.</span>
                {/if}
              </td>
              <td class="c-use"></td>
              <td class="c-inc"></td>
              <td class="c-del">
                <button
                  type="button"
                  class="del"
                  title="Remover a linha"
                  aria-label="Remover a nova linha"
                  onclick={() => tax.removeDraft(d.id)}
                >
                  <Icon name="x" size={14} />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row" colspan="3">
              Total dos meses incluídos
              <span class="sub">{plural(considered.length, 'mês', 'meses')} com valor</span>
            </th>
            <td class="num total" colspan="3">
              {formatBRL(total)}
              <span class="sub">média {formatBRL(tax.averageCents)} por mês</span>
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
  {:else}
    <div class="blank">
      <p>Nenhum mês ainda. Use “Adicionar linha” ou os próximos meses com o faturamento que você espera, ou carregue notas.</p>
    </div>
  {/if}

  <div class="foot">
    <button
      type="button"
      class="btn btn-sm"
      disabled={tax.notesAverage == null || !emptyForecasts}
      title={tax.notesAverage == null
        ? 'Sem notas para calcular a média'
        : emptyForecasts
          ? 'Preenche os meses previstos que estão vazios'
          : 'Nenhum previsto vazio'}
      onclick={fillAverage}
    >
      <Icon name="barChart" size={14} />
      Preencher vazios com a média{tax.notesAverage != null ? ` (${formatBRL(tax.notesAverage)})` : ''}
    </button>
    <span class="foot-note">
      Previsto vale só para o cálculo: não muda as notas.
      {tax.plan.projections.length
        ? plural(tax.plan.projections.length, 'mês previsto', 'meses previstos')
        : 'Nenhum mês previsto'}{#if drafts.length}{`, ${plural(drafts.length, 'linha incompleta', 'linhas incompletas')}`}{/if}.
    </span>
  </div>
</TaxCard>

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 12px;
    border-radius: var(--radius);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 13px;
  }

  .notice > span {
    flex: 1;
    min-width: 0;
  }

  .notice strong {
    font-weight: 600;
  }

  .notice :global(svg),
  .notice .btn {
    flex: none;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-4);
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .presets button,
  .next button {
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    transition:
      background-color 0.12s var(--ease),
      border-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .presets button:hover:not(:disabled),
  .next button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .presets button.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .presets button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .current {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border: 1px solid var(--accent);
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .add {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-3);
  }


  .next {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .next button {
    min-width: 32px;
    padding: 0 8px;
  }

  .next-label {
    font-size: 12px;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ table */

  .wrap {
    container: months / inline-size;
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 13px;
  }

  thead th {
    height: 32px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  tbody th,
  tbody td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }

  th:first-child,
  td:first-child {
    padding-left: var(--s-4);
  }

  th:last-child,
  td:last-child {
    padding-right: var(--s-4);
  }

  tbody th {
    font-weight: 500;
    text-align: left;
  }

  .c-month {
    width: 124px;
    white-space: nowrap;
  }

  tbody .c-month {
    padding-top: 13px;
  }

  .month {
    display: block;
    color: var(--text);
  }

  .tag {
    display: inline-block;
    margin-top: 2px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 10.5px;
    font-weight: 500;
    line-height: 16px;
    color: var(--text-3);
  }

  .c-notes {
    width: 120px;
    text-align: right;
  }

  thead .c-notes {
    text-align: right;
  }

  tbody .c-notes {
    padding-top: 13px;
  }

  .c-notes .value {
    display: block;
    color: var(--text-2);
  }

  .value.chosen {
    font-weight: 600;
    color: var(--text);
  }

  .sub {
    display: block;
    margin-top: 1px;
    font-size: 11.5px;
    font-weight: 400;
    color: var(--text-3);
  }

  .none {
    color: var(--text-3);
  }

  .forecast {
    display: flex;
    gap: 6px;
    min-width: 0;
  }

  .amount {
    flex: 0 0 124px;
  }

  /* The chosen source stands out: its field gets the accent border. */
  .amount.chosen :global(.money) {
    border-color: var(--accent);
  }

  .note {
    flex: 1;
    min-width: 72px;
    height: 32px;
    font-size: 12.5px;
  }

  .hint {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
    font-size: 11.5px;
    color: var(--warn);
  }

  .hint .link {
    font-size: 11.5px;
  }

  .c-use {
    width: 146px;
  }

  .use {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .use button {
    height: 26px;
    padding: 0 8px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }

  .use button:hover:not(:disabled) {
    color: var(--text);
  }

  .use button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
  }

  .use button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .c-inc {
    width: 64px;
    text-align: center;
  }

  thead .c-inc {
    text-align: center;
  }

  tbody .c-inc {
    padding-top: 15px;
  }

  .c-del {
    width: 44px;
    text-align: right;
  }

  tbody .c-del {
    padding-top: 10px;
  }

  .del {
    display: inline-grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
  }

  .del:hover {
    background: var(--danger-soft);
    color: var(--danger);
  }

  tr.off .c-month,
  tr.off .c-notes,
  tr.off .forecast,
  tr.off .use {
    opacity: 0.5;
  }

  tr.empty .c-month .month {
    color: var(--warn);
  }

  tr.draft th,
  tr.draft td {
    background: var(--surface-2);
  }

  tbody tr.draft .c-month {
    padding-top: 10px;
  }

  tr.flash th,
  tr.flash td {
    animation: flash 1.8s var(--ease);
  }

  @keyframes flash {
    0%,
    40% {
      background: var(--accent-soft-2);
    }
  }

  tfoot th,
  tfoot td {
    padding: 10px 8px 4px;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    vertical-align: top;
  }

  tfoot th:first-child {
    padding-left: var(--s-4);
  }

  tfoot .total {
    text-align: right;
    font-size: 14px;
  }

  .blank {
    padding: var(--s-5) var(--s-2);
    text-align: center;
    font-size: 13px;
    color: var(--text-3);
  }

  .foot {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-3);
    padding-top: var(--s-2);
    border-top: 1px solid var(--border);
  }

  .foot-note {
    font-size: 12px;
    color: var(--text-3);
  }

  @container months (max-width: 720px) {
    .forecast {
      flex-direction: column;
    }

    .amount {
      flex-basis: auto;
    }

    /* In a column, flex: 1 would squeeze the field to its minimum height. */
    .note {
      flex: none;
      height: 28px;
    }
  }
</style>

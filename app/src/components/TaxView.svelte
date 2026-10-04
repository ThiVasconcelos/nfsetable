<script lang="ts">
  import { onMount, tick, untrack } from 'svelte'
  import { formatMonthLong, formatMonthRange, plural } from '../lib/format'
  import { TAX_PRESETS } from '../lib/period'
  import { store } from '../lib/store.svelte'
  import { tax, type TaxTab } from '../lib/tax.svelte'
  import Icon from './Icon.svelte'
  import PeriodPicker from './PeriodPicker.svelte'
  import ProgressBar from './ProgressBar.svelte'
  import TaxCard from './TaxCard.svelte'
  import TaxCashFlowTab from './TaxCashFlowTab.svelte'
  import TaxComparison from './TaxComparison.svelte'
  import TaxCostsTab from './TaxCostsTab.svelte'
  import TaxLeftoverCard from './TaxLeftoverCard.svelte'
  import TaxMeiCard from './TaxMeiCard.svelte'
  import TaxMonthlyCard from './TaxMonthlyCard.svelte'
  import TaxPricingCard from './TaxPricingCard.svelte'
  import TaxProLaboreCard from './TaxProLaboreCard.svelte'
  import TaxProLaboreTab from './TaxProLaboreTab.svelte'
  import TaxRevenueCard from './TaxRevenueCard.svelte'
  import TaxRevenueTab from './TaxRevenueTab.svelte'
  import TaxSettings from './TaxSettings.svelte'
  import TaxSimplesCard from './TaxSimplesCard.svelte'
  import TaxSimplesLimitCard from './TaxSimplesLimitCard.svelte'

  const ALL_TABS: { id: TaxTab; label: string }[] = [
    { id: 'summary', label: 'Resumo' },
    { id: 'revenue', label: 'Receitas e projeção' },
    { id: 'costs', label: 'Custos e reserva' },
    { id: 'prolabore', label: 'Pró-labore' },
    { id: 'cashflow', label: 'Fluxo mês a mês' },
  ]

  onMount(() => {
    void tax.loadCatalog()
  })

  // Ask for a new report whenever the input changes (debounced in the store).
  $effect(() => {
    const input = tax.input
    untrack(() => tax.request(input))
  })

  const report = $derived(tax.report)
  const missing = $derived(tax.revenue.missing)
  const expenseMissing = $derived(tax.expenses.missing)
  /** Months in the period, considered (with a value) or still empty. */
  const periodNote = $derived.by(() => {
    const considered = tax.consideredMonths.length
    const empty = tax.emptyMonths.length
    if (!considered && !empty) return 'Nenhum mês da lista no período'
    const parts = [plural(considered, 'mês considerado', 'meses considerados')]
    if (empty) parts.push(plural(empty, 'sem valor', 'sem valor'))
    return parts.join(' · ')
  })
  /** Items of "Custos e reserva": fixed costs and typed variable costs. */
  const costCount = $derived(tax.plan.costs.length + tax.plan.variableCosts.length)
  /** MEI: no pró-labore anywhere (no card, no tab, no INSS + IRRF). */
  const mei = $derived(tax.settings.regime === 'mei')
  const TABS = $derived(mei ? ALL_TABS.filter((t) => t.id !== 'prolabore') : ALL_TABS)
  /** The open tab ("Pró-labore" saved while in the MEI shows the Resumo). */
  const tab = $derived<TaxTab>(mei && tax.plan.tab === 'prolabore' ? 'summary' : tax.plan.tab)
  const year = $derived(report?.referenceMonth.slice(0, 4) ?? tax.currentYear)
  const stale = $derived(tax.loading && !!report)
  /** Nothing to calculate from: no notes and no projection. */
  const empty = $derived(!store.rows.length && !tax.plan.projections.length && !store.busy)

  /** Row of the comparison that matches the current regime. */
  const currentRegime = $derived.by(() => {
    const regime = tax.settings.regime
    if (regime !== 'simples') return regime
    return report?.simples.annex === 'V' ? 'simplesV' : 'simplesIii'
  })
  const regimeLabel = $derived.by(() => {
    const regime = tax.settings.regime
    if (regime === 'mei') return 'MEI'
    if (regime === 'presumido') return 'Lucro Presumido'
    return `Simples Nacional, Anexo ${report?.simples.annex ?? 'III'}`
  })

  const subtitle = $derived.by(() => {
    if (!report || !report.revenue.monthsCount || !tax.input) return 'Estimativas a partir do faturamento das notas e do previsto'
    const months = report.revenue.months
    const range = formatMonthRange(months[0].month, months[months.length - 1].month)
    const count = plural(report.revenue.monthsCount, 'mês', 'meses')
    const forecastCount = tax.forecastAmong(months)
    const forecast = forecastCount ? `, ${plural(forecastCount, 'previsto', 'previstos')}` : ''
    return `Um mês típico de ${count} (${range}${forecast}) · tabelas de ${formatMonthLong(report.referenceMonth)}`
  })

  function reviewMissing() {
    store.reviewCompetence(missing.map((r) => r.path))
  }

  /** From the empty state: the next 3 months, ready to be filled in. */
  async function startProjection() {
    tax.setTab('revenue')
    const added = tax.addNextMonths(3)
    await tick()
    document.getElementById(`forecast-${added[0] ?? tax.monthKeys[0]}`)?.focus()
  }

  async function selectTab(id: TaxTab, focus = false) {
    tax.setTab(id)
    if (!focus) return
    await tick()
    document.getElementById(`tax-tab-${id}`)?.focus()
  }

  /** Tabs pattern: arrows move between tabs, Home/End jump to the ends. */
  function onTabKeydown(event: KeyboardEvent, index: number) {
    const last = TABS.length - 1
    const to =
      event.key === 'ArrowRight' ? (index === last ? 0 : index + 1)
      : event.key === 'ArrowLeft' ? (index === 0 ? last : index - 1)
      : event.key === 'Home' ? 0
      : event.key === 'End' ? last
      : -1
    if (to < 0) return
    event.preventDefault()
    void selectTab(TABS[to].id, true)
  }
</script>

<div class="tax-view">
  <ProgressBar />
  <TaxSettings />

  <main class="results" aria-label="Impostos" aria-busy={tax.loading}>
    <header class="head">
      <div class="titles">
        <h1>Impostos</h1>
        <p class="subtitle">{subtitle}</p>
      </div>
      <div class="status" aria-live="polite">
        {#if tax.loading && tax.input}<span class="calc"><span class="spinner"></span> Calculando…</span>{/if}
        {#if tax.saveState === 'pending'}
          <span class="save">Salvando…</span>
        {:else if tax.saveState === 'saved'}
          <span class="save ok"><Icon name="check" size={13} strokeWidth={2.2} /> Salvo</span>
        {:else if tax.saveState === 'error'}
          <button
            type="button"
            class="save error"
            title="{tax.saveError ?? 'Não foi possível salvar.'} {tax.readBlocked ? 'Clique para ver as opções.' : 'Clique para tentar de novo.'}"
            onclick={() => tax.retrySave()}
          >
            <Icon name="alertCircle" size={13} /> Não salvo
          </button>
        {/if}
      </div>
    </header>

    <p class="disclaimer"><Icon name="info" size={14} /> Estimativa para planejamento. Não substitui um contador.</p>

    {#if empty}
      <div class="empty">
        <span class="empty-icon"><Icon name="calculator" size={22} /></span>
        <h2>Nenhuma nota nem projeção ainda</h2>
        <p>
          O faturamento de cada mês vem das notas da tabela ou do valor que você espera receber. Uma empresa nova pode
          começar só com a projeção.
        </p>
        <div class="empty-actions">
          <button type="button" class="btn btn-primary" onclick={startProjection}>
            <Icon name="plus" size={15} />
            Adicionar projeção
          </button>
          <button type="button" class="btn" onclick={() => store.setView('notes')}>
            <Icon name="table" size={15} />
            Ir para Notas
          </button>
        </div>
      </div>
    {:else}
      <div class="tabs" role="tablist" aria-label="Seções dos impostos">
        {#each TABS as t, i (t.id)}
          {@const selected = tab === t.id}
          <button
            type="button"
            role="tab"
            id="tax-tab-{t.id}"
            aria-selected={selected}
            aria-controls="tax-panel"
            tabindex={selected ? 0 : -1}
            class:selected
            onclick={() => selectTab(t.id)}
            onkeydown={(e) => onTabKeydown(e, i)}
          >
            {t.label}
            {#if t.id === 'revenue' && tax.plan.projections.length}
              <span class="n num" title="Meses com previsto">{tax.plan.projections.length}</span>
            {:else if t.id === 'costs' && costCount}
              <span class="n num" title="Custos fixos e gastos lançados">{costCount}</span>
            {/if}
          </button>
        {/each}
      </div>

      <div class="panel" id="tax-panel" role="tabpanel" aria-labelledby="tax-tab-{tab}">
        {#if tab === 'summary' || tab === 'cashflow' || tab === 'prolabore'}
          <div class="period-bar">
            <PeriodPicker
              value={tax.period}
              presets={TAX_PRESETS}
              manual
              summary="Inclui os meses das notas e os previstos de “Receitas e projeção”."
              onchange={(p) => tax.setPeriod(p)}
            />
            <span class="period-note">{periodNote}</span>
          </div>
        {/if}
        {#if tab === 'revenue'}
          <div class="stack"><TaxRevenueTab /></div>
        {:else if tab === 'costs'}
          <div class="grid"><TaxCostsTab /></div>
        {:else if (tab === 'cashflow' || tab === 'prolabore') && tax.consideredMonths.length}
          {#if report}
            <div class="stack" class:stale>
              {#if tax.error}
                <div class="notice error" role="alert">
                  <Icon name="alertCircle" size={16} />
                  <span>Não foi possível recalcular: {tax.error}. Os valores abaixo são do cálculo anterior.</span>
                  <button type="button" class="btn btn-sm" onclick={() => tax.retry()}>Tentar de novo</button>
                </div>
              {/if}
              {#if expenseMissing.length}
                <div class="notice warn">
                  <Icon name="calendar" size={16} />
                  <span>
                    <strong>{plural(expenseMissing.length, 'despesa sem competência', 'despesas sem competência')}</strong>
                    {expenseMissing.length === 1 ? 'ficou' : 'ficaram'} de fora dos gastos variáveis.
                  </span>
                  <button type="button" class="btn btn-sm" onclick={() => store.reviewCompetence(expenseMissing.map((r) => r.path))}>
                    Preencher competência
                  </button>
                </div>
              {/if}
              {#if tab === 'cashflow'}
                <TaxCashFlowTab flow={report.cashFlow ?? []} {regimeLabel} />
              {:else}
                <TaxProLaboreTab
                  proLabore={report.proLabore}
                  regime={tax.settings.regime}
                  activity={tax.activity}
                  flow={report.cashFlow ?? []}
                  payrollCents={tax.settings.payrollCents}
                />
              {/if}
            </div>
          {:else if tax.error}
            <div class="notice error" role="alert">
              <Icon name="alertCircle" size={16} />
              <span>Não foi possível calcular: {tax.error}</span>
              <button type="button" class="btn btn-sm" onclick={() => tax.retry()}>Tentar de novo</button>
            </div>
          {:else}
            <div class="empty"><span class="spinner"></span><p>Calculando…</p></div>
          {/if}
        {:else if !tax.consideredMonths.length}
          <div class="empty">
            {#if store.busy}
              <span class="spinner"></span>
              <h2>Lendo as notas…</h2>
              <p>O faturamento aparece aqui assim que a leitura terminar.</p>
            {:else if !tax.monthKeys.length && missing.length}
              <span class="empty-icon warn"><Icon name="calendar" size={22} /></span>
              <h2>Nenhuma nota tem competência</h2>
              <p>
                {plural(missing.length, 'nota com valor está', 'notas com valor estão')} sem o mês de competência. Informe
                o mês na tabela ou use uma projeção.
              </p>
              <div class="empty-actions">
                <button type="button" class="btn btn-primary" onclick={reviewMissing}>
                  <Icon name="calendar" size={15} />
                  Preencher competências
                </button>
                <button type="button" class="btn" onclick={startProjection}>Adicionar projeção</button>
              </div>
            {:else if !tax.monthKeys.length}
              <span class="empty-icon"><Icon name="fileText" size={22} /></span>
              <h2>Nenhuma nota com valor para somar</h2>
              <p>Os impostos usam as notas que entram no total (sem duplicadas nem canceladas) e o previsto.</p>
              <div class="empty-actions">
                <button type="button" class="btn btn-primary" onclick={startProjection}>Adicionar projeção</button>
                <button type="button" class="btn" onclick={() => store.setView('notes')}>Ir para Notas</button>
              </div>
            {:else if tax.emptyMonths.length}
              <span class="empty-icon"><Icon name="calendar" size={22} /></span>
              <h2>Preencha o previsto dos meses</h2>
              <p>
                {plural(tax.emptyMonths.length, 'mês incluído ainda não tem', 'meses incluídos ainda não têm')} valor.
                Informe quanto espera faturar em cada um.
              </p>
              <button type="button" class="btn btn-primary" onclick={() => selectTab('revenue')}>
                Abrir receitas e projeção
              </button>
            {:else}
              <span class="empty-icon"><Icon name="calendar" size={22} /></span>
              <h2>Escolha pelo menos um mês</h2>
              <p>Em “Receitas e projeção”, marque os meses que representam o seu faturamento.</p>
              <button type="button" class="btn" onclick={() => tax.setPreset('all')}>Considerar todos os meses</button>
            {/if}
          </div>
        {:else if !report}
          {#if tax.error}
            <div class="notice error" role="alert">
              <Icon name="alertCircle" size={16} />
              <span>Não foi possível calcular: {tax.error}</span>
              <button type="button" class="btn btn-sm" onclick={() => tax.retry()}>Tentar de novo</button>
            </div>
          {:else}
            <div class="empty"><span class="spinner"></span><p>Calculando…</p></div>
          {/if}
        {:else}
          <div class="stack">
            {#if tax.error}
              <div class="notice error" role="alert">
                <Icon name="alertCircle" size={16} />
                <span>Não foi possível recalcular: {tax.error}. Os valores abaixo são do cálculo anterior.</span>
                <button type="button" class="btn btn-sm" onclick={() => tax.retry()}>Tentar de novo</button>
              </div>
            {/if}
            {#if missing.length}
              <div class="notice warn">
                <Icon name="calendar" size={16} />
                <span>
                  <strong>{plural(missing.length, 'nota sem competência', 'notas sem competência')}</strong>
                  {missing.length === 1 ? 'ficou' : 'ficaram'} de fora do faturamento.
                </span>
                <button type="button" class="btn btn-sm" onclick={reviewMissing}>Preencher competência</button>
              </div>
            {/if}
            {#if expenseMissing.length}
              <div class="notice warn">
                <Icon name="calendar" size={16} />
                <span>
                  <strong>{plural(expenseMissing.length, 'despesa sem competência', 'despesas sem competência')}</strong>
                  {expenseMissing.length === 1 ? 'ficou' : 'ficaram'} de fora dos gastos variáveis.
                </span>
                <button type="button" class="btn btn-sm" onclick={() => store.reviewCompetence(expenseMissing.map((r) => r.path))}>
                  Preencher competência
                </button>
              </div>
            {/if}
            {#if tax.emptyMonths.length}
              <div class="notice warn">
                <Icon name="alert" size={16} />
                <span>
                  <strong>{plural(tax.emptyMonths.length, 'mês incluído', 'meses incluídos')} sem valor</strong>
                  {tax.emptyMonths.length === 1 ? 'ficou' : 'ficaram'} de fora do cálculo.
                </span>
                <button type="button" class="btn btn-sm" onclick={() => selectTab('revenue')}>Preencher</button>
              </div>
            {/if}
            {#if store.busy}
              <div class="notice">
                <span class="spinner"></span>
                <span>Lendo notas: os valores vão mudar quando a leitura terminar.</span>
              </div>
            {/if}

            <div class="grid" class:stale>
              <TaxRevenueCard revenue={report.revenue} />
              <TaxMonthlyCard flow={report.cashFlow ?? []} regime={tax.settings.regime} onflow={() => selectTab('cashflow')} />
              {#if !mei}
                <TaxProLaboreCard
                  proLabore={report.proLabore}
                  regime={tax.settings.regime}
                  activity={tax.activity}
                  simples={report.simples}
                  flow={report.cashFlow ?? []}
                  ondetails={() => selectTab('prolabore')}
                />
                <TaxSimplesCard simples={report.simples} activity={tax.activity} automatic={report.proLabore.automatic} />
              {/if}
              <!-- The limit of the current regime (the comparativo still shows the MEI to everyone). -->
              {#if mei}
                <TaxMeiCard
                  mei={report.mei}
                  cnae={tax.cnaeInfo}
                  {year}
                  openingYear={tax.settings.openingMonth?.slice(0, 4) === year}
                />
              {:else if tax.settings.regime === 'simples'}
                <TaxSimplesLimitCard revenue={report.revenue} simples={report.simples} {year} />
              {/if}
              <TaxComparison comparison={report.comparison} current={currentRegime} />
              <TaxLeftoverCard
                leftover={report.leftover}
                {regimeLabel}
                base={tax.plan.leftoverBase}
                costs={tax.plan.costs}
                referenceMonth={tax.referenceMonth}
                reserveRate={tax.plan.reserveRate}
                {mei}
                onedit={() => selectTab('costs')}
              />
              <TaxPricingCard pricing={report.pricing} averageCents={report.revenue.averageMonthlyCents} />

              <TaxCard title="Avisos e fontes" subtitle="O que a estimativa considera e de onde vêm as tabelas" icon="book" wide>
                {#if report.warnings.length}
                  <ul class="warnings">
                    {#each report.warnings as warning, i (i)}
                      <li><Icon name="info" size={14} /> <span>{warning}</span></li>
                    {/each}
                  </ul>
                {/if}
                {#if report.sources.length}
                  <details class="sources">
                    <summary>Fontes e base legal ({report.sources.length})</summary>
                    <ul>
                      {#each report.sources as source, i (i)}
                        <li>{source}</li>
                      {/each}
                    </ul>
                  </details>
                {/if}
              </TaxCard>
            </div>

            <p class="disclaimer bottom">
              <Icon name="info" size={14} />
              Estimativa para planejamento. Não substitui um contador: confirme os valores antes de decidir ou pagar.
            </p>
          </div>
        {/if}
      </div>
    {/if}
  </main>
</div>

<style>
  .tax-view {
    --tax-settings-w: 300px;
    position: relative;
    display: grid;
    grid-template-columns: var(--tax-settings-w) minmax(0, 1fr);
    min-height: 0;
  }

  .results {
    container: results / inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    min-width: 0;
    min-height: 0;
    padding: 20px var(--s-5) var(--s-6);
    overflow-y: auto;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s-4);
  }

  .titles {
    min-width: 0;
  }

  h1 {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.01em;
    line-height: 1.25;
  }

  .subtitle {
    margin-top: 2px;
    font-size: 13px;
    color: var(--text-3);
  }

  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--s-3);
    flex: none;
    min-height: 28px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  .calc,
  .save {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    white-space: nowrap;
  }

  .save.ok :global(svg) {
    color: var(--ok);
  }

  button.save {
    height: 24px;
    padding: 0 6px;
    margin: 0 -6px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    font-size: inherit;
  }

  button.save:hover {
    background: var(--danger-soft);
  }

  .save.error {
    color: var(--danger);
  }

  .disclaimer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: -6px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  .disclaimer :global(svg) {
    flex: none;
    color: var(--accent-text);
  }

  .disclaimer.bottom {
    margin-top: var(--s-2);
    justify-content: center;
    text-align: center;
  }

  /* ------------------------------------------------------------ tabs */

  .tabs {
    display: flex;
    gap: var(--s-4);
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 38px;
    padding: 0 2px;
    border: 0;
    background: transparent;
    color: var(--text-3);
    font-size: 13.5px;
    font-weight: 500;
    white-space: nowrap;
    transition: color 0.12s var(--ease);
  }

  .tabs button:hover {
    color: var(--text);
  }

  .tabs button.selected {
    color: var(--text);
  }

  .tabs button.selected::after {
    content: '';
    position: absolute;
    left: 0;
    right: 0;
    bottom: -1px;
    height: 2px;
    border-radius: 2px 2px 0 0;
    background: var(--accent);
  }

  .n {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 11px;
    line-height: 17px;
    text-align: center;
    color: var(--text-3);
  }

  .selected .n {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    min-width: 0;
  }

  .period-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-3);
  }

  .period-note {
    font-size: 12.5px;
    color: var(--text-3);
  }

  .panel:focus {
    outline: none;
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    min-width: 0;
  }

  /* ------------------------------------------------------------ notices, grid, empty */

  .notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    font-size: 13px;
    color: var(--text-2);
  }

  .notice > span:not(.spinner) {
    flex: 1;
    min-width: 0;
  }

  .notice :global(svg) {
    flex: none;
  }

  .notice.warn {
    border-color: transparent;
    background: var(--warn-soft);
    color: var(--warn);
  }

  .notice.warn strong {
    font-weight: 600;
  }

  .notice.error {
    border-color: transparent;
    background: var(--danger-soft);
    color: var(--danger);
  }

  .notice .btn {
    flex: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-flow: row dense;
    gap: var(--s-3);
    transition: opacity 0.2s var(--ease);
  }

  .grid.stale,
  .stack.stale {
    opacity: 0.6;
  }

  .warnings {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-2);
  }

  .warnings li {
    display: flex;
    gap: 8px;
  }

  .warnings :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--text-3);
  }

  .sources {
    font-size: 12.5px;
    color: var(--text-2);
  }

  .sources summary {
    width: fit-content;
    cursor: pointer;
    color: var(--accent-text);
    font-weight: 500;
  }

  .sources ul {
    margin: 8px 0 0;
    padding-left: 18px;
    line-height: 1.5;
    color: var(--text-3);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s-2);
    flex: 1;
    min-height: 260px;
    padding: var(--s-6) var(--s-4);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: center;
    color: var(--text-3);
  }

  .empty h2 {
    font-size: 16px;
    font-weight: 600;
    color: var(--text);
  }

  .empty p {
    max-width: 440px;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .empty .btn,
  .empty-actions {
    margin-top: var(--s-2);
  }

  .empty-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--s-2);
  }

  .empty-actions .btn {
    margin-top: 0;
  }

  .empty-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-bottom: var(--s-1);
    border-radius: 12px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .empty-icon.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  @container results (max-width: 640px) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }

    .tabs {
      gap: var(--s-3);
    }
  }

  @container results (min-width: 1180px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }

  @media (max-width: 1199px) {
    .tax-view {
      --tax-settings-w: 272px;
    }

    .results {
      padding: var(--s-4);
    }
  }

  @media (max-height: 700px) {
    .results {
      padding-top: var(--s-4);
    }
  }
</style>

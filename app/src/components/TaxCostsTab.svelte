<script lang="ts">
  import { tick } from 'svelte'
  import { formatBRL, formatMonthLong, formatMonthShort, formatRateInput, normalizeText, parseRateInput, plural, sameText } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import {
    COST_NAME_MAX,
    COST_SUGGESTIONS,
    RESERVE_MAX,
    activeIn,
    costValidity,
    datesInvalid,
    isDated,
    monthlyCost,
    tax,
    type CostFrequency,
    type CostItem,
    type LeftoverBase,
  } from '../lib/tax.svelte'
  import Icon from './Icon.svelte'
  import MoneyField from './MoneyField.svelte'
  import MonthInput from './MonthInput.svelte'
  import TaxCard from './TaxCard.svelte'

  const FREQUENCIES: { id: CostFrequency; label: string }[] = [
    { id: 'monthly', label: 'Mensal' },
    { id: 'yearly', label: 'Anual' },
  ]
  const RESERVE_PRESETS = [0, 0.05, 0.1, 0.15, 0.2]

  let reserveDraft = $state<string | null>(null)
  let reserveInvalid = $state(false)
  /** Fixed cost whose "tipo de nota" list is open. */
  let linking = $state<string | null>(null)
  /** Fixed cost whose "Início / Fim" fields are open. */
  let dating = $state<string | null>(null)
  const anyDated = $derived(tax.plan.costs.some(isDated))

  const variable = $derived(tax.variableByMonth)
  const manual = $derived(tax.plan.variableCosts)
  const expenseMissing = $derived(tax.expenses.missing)
  const consideredSet = $derived(new Set(tax.consideredMonths.map((m) => m.month)))


  /** Types offered to a fixed cost: the expense types (the one already linked included). */
  function typeOptions(item: CostItem): string[] {
    const list = [...tax.expenseTypes]
    if (item.linkedType && !list.some((t) => sameText(t, item.linkedType))) list.push(item.linkedType)
    return list
  }

  /** The type is linked to another fixed cost. */
  function linkedElsewhere(type: string, id: string): boolean {
    const other = tax.costLinkedTo(type)
    return !!other && other.id !== id
  }

  async function openLink(id: string) {
    linking = id
    await tick()
    document.getElementById(`cost-link-${id}`)?.focus()
  }

  function linkType(id: string, type: string) {
    tax.updateCost(id, { linkedType: type || null })
    linking = null
  }

  async function addVariableRow() {
    const id = tax.addVariableCost()
    await tick()
    document.getElementById(`var-month-${id}`)?.focus()
  }

  const costs = $derived(tax.plan.costs)
  const monthly = $derived(tax.costsMonthlyCents)
  const names = $derived(new Set(costs.map((c) => normalizeText(c.name))))
  const suggestions = $derived(COST_SUGGESTIONS.filter((s) => !names.has(normalizeText(s.name))))
  const noBonus = $derived(tax.noBonus)
  const base = $derived(tax.plan.leftoverBase)
  /** Revenue the reserve is taken from (the base of "sobra do mês"). */
  const reserveBase = $derived(tax.leftoverRevenueCents ?? tax.averageCents)

  async function focus(id: string) {
    await tick()
    document.getElementById(id)?.focus()
  }

  function addSuggestion(name: string, frequency: CostFrequency) {
    const id = tax.addCost({ name, frequency })
    void focus(`cost-value-${id}`)
  }

  function addEmpty() {
    const id = tax.addCost()
    void focus(`cost-name-${id}`)
  }

  function commitReserve() {
    if (reserveDraft == null) return
    const rate = parseRateInput(reserveDraft || '0')
    if (rate == null || rate > RESERVE_MAX) {
      reserveInvalid = true
      return
    }
    tax.setReserveRate(rate)
    reserveDraft = null
  }

  function chooseBase(next: LeftoverBase) {
    tax.setLeftoverBase(next)
    if (next === 'fixed' && tax.plan.leftoverFixedCents == null) void focus('leftover-fixed')
  }
</script>

<TaxCard
  title="Custos fixos"
  subtitle="O mesmo valor todo mês (anuais divididos por 12). Ligue um custo a um tipo de nota para ver o previsto e o real."
  icon="coins"
  wide
>
  {#if suggestions.length}
    <div class="suggestions" role="group" aria-label="Sugestões de custos">
      {#each suggestions as s (s.name)}
        <button type="button" class="chip" onclick={() => addSuggestion(s.name, s.frequency)}>
          <Icon name="plus" size={12} strokeWidth={2.2} />
          {s.name}{s.frequency === 'yearly' ? ' (anual)' : ''}
        </button>
      {/each}
    </div>
  {/if}

  {#if costs.length}
    <div class="wrap">
      <table>
        <thead>
          <tr>
            <th class="c-name">Custo</th>
            <th class="c-value">Valor</th>
            <th class="c-freq">Frequência</th>
            <th class="c-month">Por mês</th>
            <th class="c-del"><span class="sr-only">Remover</span></th>
          </tr>
        </thead>
        <tbody>
          {#each costs as c (c.id)}
            <tr class:linked={!!c.linkedType}>
              <td class="c-name">
                <input
                  id="cost-name-{c.id}"
                  class="input"
                  type="text"
                  placeholder="Nome do custo"
                  aria-label="Nome do custo"
                  maxlength={COST_NAME_MAX}
                  value={c.name}
                  oninput={(e) => tax.updateCost(c.id, { name: e.currentTarget.value })}
                />
                <div class="link-line">
                  <button
                    type="button"
                    class="link subtle validity"
                    class:warn={datesInvalid(c)}
                    class:set={isDated(c)}
                    aria-expanded={dating === c.id}
                    title="Início e fim do custo (vazio = sempre)"
                    onclick={() => (dating = dating === c.id ? null : c.id)}
                  >
                    <Icon name="calendar" size={12} />
                    {datesInvalid(c) ? 'vigência inválida' : costValidity(c)}
                  </button>
                  <span class="dot" aria-hidden="true">·</span>
                  {#if c.linkedType || linking === c.id}
                    {@const options = typeOptions(c)}
                    <Icon name="link" size={12} />
                    <span class="link-label">Notas do tipo</span>
                    <select
                      id="cost-link-{c.id}"
                      class="link-select"
                      aria-label="Tipo de nota ligado a {c.name || 'custo'}"
                      value={c.linkedType ?? ''}
                      onchange={(e) => linkType(c.id, e.currentTarget.value)}
                      onblur={() => {
                        if (!c.linkedType) linking = null
                      }}
                    >
                      {#if !c.linkedType}
                        <option value="">{options.length ? 'Escolha o tipo…' : 'Nenhuma despesa na tabela'}</option>
                      {/if}
                      {#each options as t (t)}
                        <option value={t} disabled={linkedElsewhere(t, c.id)}>{t}{linkedElsewhere(t, c.id) ? ' (já ligado)' : ''}</option>
                      {/each}
                      {#if c.linkedType}<option value="">Desligar</option>{/if}
                    </select>
                  {:else}
                    <button type="button" class="link subtle" onclick={() => openLink(c.id)}>
                      <Icon name="link" size={12} /> Ligar a um tipo de nota
                    </button>
                  {/if}
                </div>
                {#if dating === c.id || datesInvalid(c)}
                  <div class="dates">
                    <span class="k">Início</span>
                    <MonthInput
                      id="cost-start-{c.id}"
                      size="sm"
                      label="Início de {c.name || 'custo'} (vazio = sempre)"
                      placeholder="sempre"
                      value={c.start}
                      floatMessage
                      onchange={(month) => tax.updateCost(c.id, { start: month })}
                    />
                    <span class="k">Fim</span>
                    <MonthInput
                      id="cost-end-{c.id}"
                      size="sm"
                      label="Fim de {c.name || 'custo'} (opcional)"
                      placeholder="sem fim"
                      value={c.end}
                      error={datesInvalid(c) ? 'O fim é antes do início' : null}
                      errorText={false}
                      floatMessage
                      onchange={(month) => tax.updateCost(c.id, { end: month })}
                    />
                  </div>
                  {#if datesInvalid(c)}
                    <span class="row-hint"><Icon name="alert" size={12} /> O fim precisa ser igual ou depois do início.</span>
                  {/if}
                {/if}
              </td>
              <td class="c-value">
                <MoneyField
                  id="cost-value-{c.id}"
                  label="Valor de {c.name || 'custo'}"
                  placeholder="0,00"
                  allowEmpty
                  value={c.cents}
                  onchange={(v) => tax.updateCost(c.id, { cents: v })}
                />
              </td>
              <td class="c-freq">
                <div class="seg" role="group" aria-label="Frequência de {c.name || 'custo'}">
                  {#each FREQUENCIES as f (f.id)}
                    <button
                      type="button"
                      class:active={c.frequency === f.id}
                      aria-pressed={c.frequency === f.id}
                      onclick={() => tax.updateCost(c.id, { frequency: f.id })}>{f.label}</button
                    >
                  {/each}
                </div>
              </td>
              <td class="c-month num">
                {#if c.cents != null}
                  {formatBRL(monthlyCost(c))}
                  {#if c.frequency === 'yearly'}<span class="sub">÷ 12</span>{/if}
                {:else}
                  <span class="none">—</span>
                {/if}
              </td>
              <td class="c-del">
                <button
                  type="button"
                  class="del"
                  aria-label="Remover {c.name || 'custo'}"
                  title="Remover"
                  onclick={() => tax.removeCost(c.id)}
                >
                  <Icon name="x" size={14} />
                </button>
              </td>
            </tr>
            {#if c.linkedType}
              {@const real = tax.linkedMonths(c.id)}
              {@const planned = monthlyCost(c)}
              <tr class="plan-real-row">
                <td colspan="5">
                  <div class="plan-real">
                    <span class="pr-title">Previsto × real</span>
                    {#if real.length}
                      {#each real as m (m.month)}
                        {@const active = activeIn(c, m.month)}
                        {@const diff = m.cents - (active ? planned : 0)}
                        <span
                          class="pr"
                          class:outside={!active}
                          title="{formatMonthLong(m.month)}: {plural(m.count, 'nota', 'notas')} do tipo {c.linkedType}, {active
                            ? `previsto ${formatBRL(planned)}`
                            : 'fora da vigência do custo'}"
                        >
                          <span class="pr-month">{formatMonthShort(m.month, 'full')}</span>
                          <span class="num">{formatBRL(m.cents)}</span>
                          {#if !active}
                            <span class="pr-off">fora da vigência</span>
                          {:else if c.cents != null && diff !== 0}
                            <span class="diff num" class:over={diff > 0}>{diff > 0 ? '+' : '−'}{formatBRL(Math.abs(diff))}</span>
                          {/if}
                        </span>
                      {/each}
                      <span class="pr-note">
                        Previsto {formatBRL(planned)} por mês{isDated(c) ? ` (${costValidity(c)})` : ''}. Essas notas confirmam o custo
                        e não entram nos gastos variáveis.
                      </span>
                    {:else}
                      <span class="pr-note">Nenhuma nota do tipo {c.linkedType} com competência ainda.</span>
                    {/if}
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="blank">Nenhum custo ainda. Use as sugestões acima ou adicione um custo.</p>
  {/if}

  <div class="foot">
    <button type="button" class="btn btn-sm" onclick={addEmpty}>
      <Icon name="plus" size={14} />
      Adicionar custo
    </button>
    {#if anyDated}
      <span
        class="total"
        title="Os custos sem data, mais os com início ou fim que valem no mês de referência (o último mês considerado), com o valor cheio. No Fluxo mês a mês, cada mês tem os seus."
      >
        {tax.referenceMonth ? `Em ${formatMonthShort(tax.referenceMonth, 'full')}` : 'Por mês'}
        <strong class="num">{formatBRL(tax.fixedReferenceCents)}</strong>
        {#if monthly > 0}<span class="sub-inline num">inclui {formatBRL(monthly)} sem data (todo mês)</span>{/if}
      </span>
    {:else}
      <span class="total">
        Total por mês <strong class="num">{formatBRL(monthly)}</strong>
        <span class="sub-inline num">{formatBRL(monthly * 12)} por ano</span>
      </span>
    {/if}
  </div>
</TaxCard>

<TaxCard
  title="Gastos variáveis"
  subtitle="Mudam de mês para mês: as despesas da tabela (pela competência) e os gastos lançados aqui. Cada gasto entra no mês dele: no Fluxo mês a mês e, no Resumo, se for do mês de referência."
  icon="receipt"
  wide
>
  {#if expenseMissing.length}
    <div class="notice warn">
      <Icon name="calendar" size={15} />
      <span>
        <strong>{plural(expenseMissing.length, 'despesa sem competência', 'despesas sem competência')}</strong>
        não {expenseMissing.length === 1 ? 'entra' : 'entram'} em nenhum mês.
      </span>
      <button type="button" class="btn btn-sm" onclick={() => store.reviewCompetence(expenseMissing.map((r) => r.path))}>
        Preencher competência
      </button>
    </div>
  {/if}

  <div class="sub-head">
    <span class="sub-title">Lançados por você</span>
    <button type="button" class="btn btn-sm" onclick={addVariableRow}>
      <Icon name="plus" size={14} />
      Adicionar linha
    </button>
  </div>
  {#if manual.length}
    <div class="wrap">
      <table class="var-table">
        <thead>
          <tr>
            <th class="v-month">Mês</th>
            <th class="v-desc">Descrição</th>
            <th class="v-value">Valor</th>
            <th class="c-del"><span class="sr-only">Remover</span></th>
          </tr>
        </thead>
        <tbody>
          {#each manual as v (v.id)}
            {@const incomplete = v.month == null || v.cents == null}
            <tr class:incomplete>
              <td class="v-month">
                <MonthInput
                  id="var-month-{v.id}"
                  size="sm"
                  label="Mês do gasto (MM/AAAA)"
                  value={v.month}
                  floatMessage
                  onchange={(month) => tax.updateVariableCost(v.id, { month })}
                />
              </td>
              <td class="v-desc">
                <input
                  id="var-desc-{v.id}"
                  class="input"
                  type="text"
                  placeholder="Por exemplo, viagem ao cliente"
                  aria-label="Descrição do gasto"
                  maxlength={COST_NAME_MAX}
                  value={v.description}
                  oninput={(e) => tax.updateVariableCost(v.id, { description: e.currentTarget.value })}
                />
                {#if incomplete}
                  <span class="row-hint"><Icon name="alert" size={12} /> Informe o mês e o valor para contar.</span>
                {/if}
              </td>
              <td class="v-value">
                <MoneyField
                  id="var-value-{v.id}"
                  label="Valor de {v.description || 'gasto'}"
                  placeholder="0,00"
                  allowEmpty
                  value={v.cents}
                  onchange={(cents) => tax.updateVariableCost(v.id, { cents })}
                />
              </td>
              <td class="c-del">
                <button
                  type="button"
                  class="del"
                  aria-label="Remover {v.description || 'gasto'}"
                  title="Remover"
                  onclick={() => tax.removeVariableCost(v.id)}
                >
                  <Icon name="x" size={14} />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    <p class="blank">Nenhum gasto lançado. Despesas sem nota (táxi, material, viagens) entram aqui.</p>
  {/if}

  <div class="sub-head">
    <span class="sub-title">Por mês</span>
  </div>
  {#if variable.length}
    <div class="wrap">
      <table class="months-table">
        <thead>
          <tr>
            <th class="m-month">Mês</th>
            <th class="m-num">Despesas da tabela</th>
            <th class="m-num">Lançados</th>
            <th class="m-num">Total</th>
          </tr>
        </thead>
        <tbody>
          {#each variable as m (m.month)}
            {@const inCalc = consideredSet.has(m.month)}
            <tr class:off={!inCalc}>
              <td class="m-month">
                {formatMonthShort(m.month, 'full')}
                {#if !inCalc}<span class="tag" title="O mês não está entre os meses considerados">fora do cálculo</span>{/if}
              </td>
              <td class="m-num num">
                {m.notesCents ? formatBRL(m.notesCents) : '—'}
                {#if m.notesCount}<span class="sub">{plural(m.notesCount, 'nota', 'notas')}</span>{/if}
              </td>
              <td class="m-num num">{m.manualCents ? formatBRL(m.manualCents) : '—'}</td>
              <td class="m-num num strong">{formatBRL(m.cents)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if tax.referenceMonth}
      <p class="avg">
        No Resumo entram os de {formatMonthShort(tax.referenceMonth, 'full')}, o mês de referência:
        <strong class="num">{formatBRL(tax.variableReferenceCents)}</strong>. Os outros meses ficam no Fluxo mês a mês.
      </p>
    {/if}
  {:else}
    <p class="blank">Sem gastos variáveis ainda: marque notas como despesa na tabela ou lance os gastos acima.</p>
  {/if}
</TaxCard>

<TaxCard
  title="Reserva / contingência"
  subtitle="Uma parte do faturamento guardada todo mês para imprevistos, 13º e férias que o PJ não tem."
  icon="shield"
>
  <div class="reserve">
    <div class="suffix" class:invalid={reserveInvalid}>
      <input
        id="tax-reserve"
        type="text"
        inputmode="decimal"
        maxlength="5"
        aria-label="Reserva, em porcentagem do faturamento"
        aria-invalid={reserveInvalid}
        title={reserveInvalid ? 'Use um número de 0 a 90, por exemplo 10' : 'Porcentagem do faturamento'}
        value={reserveDraft ?? formatRateInput(tax.plan.reserveRate)}
        onfocus={() => (reserveDraft = formatRateInput(tax.plan.reserveRate))}
        oninput={(e) => {
          reserveDraft = e.currentTarget.value
          reserveInvalid = false
        }}
        onblur={() => {
          commitReserve()
          reserveDraft = null
          reserveInvalid = false
        }}
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            commitReserve()
          }
        }}
      />
      <span aria-hidden="true">%</span>
    </div>
    <div class="presets" role="group" aria-label="Reserva sugerida">
      {#each RESERVE_PRESETS as r (r)}
        <button
          type="button"
          class:active={Math.abs(tax.plan.reserveRate - r) < 1e-9}
          aria-pressed={Math.abs(tax.plan.reserveRate - r) < 1e-9}
          onclick={() => tax.setReserveRate(r)}>{formatRateInput(r)}%</button
        >
      {/each}
    </div>
  </div>
  <p class="line">
    {#if tax.plan.reserveRate > 0}
      ≈ <strong class="num">{formatBRL(Math.round(reserveBase * tax.plan.reserveRate))}</strong> por mês, sobre
      {formatBRL(reserveBase)} de faturamento.
    {:else}
      Sem reserva: a sobra do mês fica toda disponível.
    {/if}
  </p>
  <p class="hint">Referência: 13º e férias de um salário CLT equivalem a cerca de 19% dele por mês (8,3% + 11,1%).</p>
</TaxCard>

<TaxCard
  title="Base da sobra do mês"
  subtitle="O faturamento usado para calcular quanto sobra"
  icon="target"
>
  <div class="bases" role="radiogroup" aria-label="Base da sobra do mês">
    <label class="radio" class:checked={base === 'average'}>
      <input type="radio" name="leftover-base" checked={base === 'average'} onchange={() => chooseBase('average')} />
      <span class="radio-text">
        <span class="radio-label">Média dos meses <span class="amount num">{formatBRL(tax.averageCents)}</span></span>
        <span class="radio-hint">{plural(tax.consideredMonths.length, 'mês considerado', 'meses considerados')}</span>
      </span>
    </label>

    <label class="radio" class:checked={base === 'noBonus'} class:disabled={!noBonus.available}>
      <input
        type="radio"
        name="leftover-base"
        checked={base === 'noBonus'}
        disabled={!noBonus.available && base !== 'noBonus'}
        onchange={() => chooseBase('noBonus')}
      />
      <span class="radio-text">
        <span class="radio-label">
          Média sem bônus
          {#if noBonus.available}<span class="amount num">{formatBRL(noBonus.averageCents)}</span>{/if}
        </span>
        <span class="radio-hint">
          {#if noBonus.available}
            {plural(noBonus.excludedNotes, 'nota de bônus', 'notas de bônus')} ({formatBRL(noBonus.excludedCents)})
            {noBonus.excludedNotes === 1 ? 'fica' : 'ficam'} de fora
          {:else}
            Nenhuma nota do tipo Bônus nos meses considerados. Mude o Tipo da nota na tabela.
          {/if}
        </span>
      </span>
    </label>
    {#if base === 'noBonus' && !noBonus.available}
      <p class="warn-line">
        <Icon name="alert" size={13} /> Sem notas de bônus, a sobra usa a média dos meses.
        <button type="button" class="link" onclick={() => store.setView('notes')}>Ver notas</button>
      </p>
    {/if}

    <div class="radio" class:checked={base === 'fixed'}>
      <input
        id="leftover-fixed-radio"
        type="radio"
        name="leftover-base"
        checked={base === 'fixed'}
        onchange={() => chooseBase('fixed')}
      />
      <span class="radio-text">
        <label class="radio-label" for="leftover-fixed-radio">Valor fixo</label>
        <span class="radio-hint">Por exemplo, só os contratos fixos</span>
        <span class="fixed">
          <MoneyField
            id="leftover-fixed"
            label="Faturamento fixo por mês"
            placeholder="0,00"
            allowEmpty
            value={tax.plan.leftoverFixedCents}
            onchange={(v) => {
              tax.setLeftoverFixed(v)
              if (v != null) tax.setLeftoverBase('fixed')
            }}
          />
        </span>
      </span>
    </div>
  </div>
  <p class="hint">
    <Icon name="info" size={13} /> O pró-labore e o Fator R continuam usando o faturamento real (a média dos meses).
  </p>
</TaxCard>

<style>
  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 28px;
    padding: 0 10px 0 8px;
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12.5px;
    white-space: nowrap;
    transition:
      background-color 0.12s var(--ease),
      border-color 0.12s var(--ease),
      color 0.12s var(--ease);
  }

  .chip:hover {
    border-style: solid;
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  /* ------------------------------------------------------------ costs table */

  .wrap {
    container: costs / inline-size;
    margin: 0 calc(-1 * var(--s-4));
    overflow-x: auto;
  }

  /* Fixed layout: the name takes whatever the other columns leave. */
  table {
    width: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
    font-size: 13px;
  }

  th {
    height: 32px;
    padding: 0 8px;
    border-bottom: 1px solid var(--border);
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
  }

  td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: middle;
  }

  th:first-child,
  td:first-child {
    padding-left: var(--s-4);
  }

  th:last-child,
  td:last-child {
    padding-right: var(--s-4);
  }

  .c-name .input {
    width: 100%;
  }

  .c-value {
    width: 156px;
  }

  .c-freq {
    width: 146px;
  }

  .c-month {
    width: 116px;
    text-align: right;
    white-space: nowrap;
  }

  th.c-month {
    text-align: right;
  }

  .c-del {
    width: 50px;
    text-align: right;
  }

  .sub {
    display: block;
    font-size: 11px;
    color: var(--text-3);
  }

  .none {
    color: var(--text-3);
  }

  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .seg button {
    height: 26px;
    padding: 0 10px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
    font-size: 12px;
    font-weight: 500;
  }

  .seg button:hover {
    color: var(--text);
  }

  .seg button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
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

  .blank {
    padding: var(--s-2) 0 var(--s-1);
    font-size: 13px;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ fixed costs: link to a type */

  .link-line {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-top: 4px;
    min-width: 0;
    font-size: 12px;
    color: var(--text-3);
  }

  .link-line :global(svg) {
    flex: none;
  }

  .dot {
    color: var(--border-strong);
  }

  .validity.set {
    color: var(--accent-text);
  }

  .validity.warn {
    color: var(--warn);
  }

  .dates {
    display: grid;
    grid-template-columns: auto minmax(0, 132px) auto minmax(0, 132px);
    justify-content: start;
    align-items: center;
    gap: 6px 8px;
    margin-top: 6px;
  }

  .dates .k {
    font-size: 12px;
    color: var(--text-3);
  }

  .pr.outside {
    border-style: dashed;
    color: var(--text-3);
  }

  .pr-off {
    font-size: 11px;
    color: var(--text-3);
  }

  .link-label {
    white-space: nowrap;
  }

  .link-select {
    min-width: 0;
    max-width: 100%;
    height: 24px;
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font-size: 12px;
  }

  .link.subtle {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--text-3);
    font-size: 12px;
  }

  .link.subtle:hover {
    color: var(--accent-text);
  }

  tr.linked td {
    border-bottom-color: transparent;
  }

  .plan-real-row td {
    padding-top: 0;
  }

  .plan-real {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    font-size: 12px;
    color: var(--text-2);
  }

  .pr-title {
    font-weight: 600;
    color: var(--text-3);
  }

  .pr {
    display: inline-flex;
    align-items: baseline;
    gap: 5px;
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    white-space: nowrap;
  }

  .pr-month {
    color: var(--text-3);
  }

  .diff {
    color: var(--ok);
  }

  .diff.over {
    color: var(--expense);
  }

  .pr-note {
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ variable costs */

  .notice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 12px;
    border-radius: var(--radius);
    font-size: 13px;
  }

  .notice.warn {
    background: var(--warn-soft);
    color: var(--warn);
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

  .sub-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    margin-top: 2px;
  }

  .sub-title {
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
  }

  .var-table .v-month {
    width: 150px;
  }

  .var-table .v-desc .input {
    width: 100%;
  }

  .var-table .v-value {
    width: 156px;
  }

  .var-table td {
    vertical-align: top;
  }

  .var-table .c-del {
    padding-top: 11px;
  }

  .row-hint {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    font-size: 11.5px;
    color: var(--warn);
  }

  .months-table .m-month {
    width: auto;
    white-space: nowrap;
  }

  .months-table .m-num {
    width: 150px;
    text-align: right;
    white-space: nowrap;
  }

  .months-table th.m-num {
    text-align: right;
  }

  .months-table td {
    padding-top: 7px;
    padding-bottom: 7px;
  }

  .months-table tr.off td {
    color: var(--text-3);
  }

  .months-table .sub {
    display: inline;
    margin-left: 6px;
  }

  .strong {
    font-weight: 600;
  }

  .tag {
    display: inline-block;
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-3);
    font-size: 10.5px;
    font-weight: 500;
    line-height: 16px;
    color: var(--text-3);
  }

  .avg {
    font-size: 12.5px;
    color: var(--text-2);
  }

  .avg strong {
    font-weight: 600;
    color: var(--text);
  }

  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-3);
  }

  .total {
    display: inline-flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 4px 8px;
    font-size: 13px;
    color: var(--text-2);
  }

  .total strong {
    font-size: 16px;
    font-weight: 650;
    color: var(--text);
  }

  .sub-inline {
    font-size: 12px;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ reserve */

  .reserve {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s-2) var(--s-3);
  }

  .suffix {
    display: flex;
    align-items: center;
    width: 96px;
    height: 36px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    transition:
      border-color 0.12s var(--ease),
      box-shadow 0.12s var(--ease);
  }

  .suffix:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .suffix.invalid {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-soft);
  }

  .suffix input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 15px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .suffix span {
    margin-left: 4px;
    font-size: 13px;
    color: var(--text-3);
  }

  .presets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .presets button {
    height: 26px;
    padding: 0 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 500;
  }

  .presets button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .presets button.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .line {
    font-size: 13px;
    color: var(--text-2);
  }

  .line strong {
    font-weight: 600;
    color: var(--text);
  }

  .hint {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-3);
  }

  .hint :global(svg) {
    flex: none;
    margin-top: 2px;
  }

  /* ------------------------------------------------------------ leftover base */

  .bases {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .radio {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 9px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .radio:hover:not(.disabled) {
    background: var(--surface-2);
  }

  .radio.checked {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .radio.disabled {
    cursor: default;
  }

  .radio.disabled .radio-label {
    color: var(--text-3);
  }

  .radio input {
    flex: none;
    width: 15px;
    height: 15px;
    margin: 2px 0 0;
    accent-color: var(--accent);
    cursor: pointer;
  }

  .radio-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .radio-label {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    cursor: pointer;
  }

  .amount {
    font-weight: 600;
    white-space: nowrap;
  }

  .radio-hint {
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-3);
  }

  .fixed {
    display: block;
    max-width: 200px;
    margin-top: 6px;
  }

  .warn-line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin: -2px 0 2px 12px;
    font-size: 12px;
    color: var(--warn);
  }

  @container costs (max-width: 700px) {
    .var-table .v-month {
      width: 132px;
    }

    .var-table .v-value {
      width: 136px;
    }

    .months-table .m-num {
      width: 120px;
    }

    .c-value {
      width: 136px;
    }

    .c-freq {
      width: 128px;
    }

    .seg button {
      padding: 0 7px;
    }

    .c-month {
      width: 112px;
    }
  }
</style>

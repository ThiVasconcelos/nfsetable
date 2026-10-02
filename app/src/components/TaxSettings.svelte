<script lang="ts">
  import { formatRateInput, parseRateInput } from '../lib/format'
  import { ACTIVITY_LABEL, REGIME_LABEL } from '../lib/labels'
  import { tax } from '../lib/tax.svelte'
  import type { SimplesActivity, TaxRegime } from '../lib/types'
  import CnaePicker from './CnaePicker.svelte'
  import Icon from './Icon.svelte'
  import MoneyField from './MoneyField.svelte'
  import MonthInput from './MonthInput.svelte'

  const REGIMES: { id: TaxRegime; hint: string }[] = [
    { id: 'mei', hint: 'DAS fixo, até R$ 81 mil por ano' },
    { id: 'simples', hint: 'ME até R$ 360 mil por ano' },
    { id: 'presumido', hint: 'IRPJ, CSLL, PIS, Cofins e ISS' },
  ]
  const s = $derived(tax.settings)
  const cnae = $derived(tax.cnaeInfo)
  /** What "Automático" means in the regime (no pró-labore at all in the MEI). */
  const autoHint = $derived(
    s.regime === 'presumido'
      ? 'Um salário mínimo: sem Fator R neste regime'
      : tax.activity === 'fatorR'
        ? 'O menor valor que deixa todos os meses no Anexo III (Fator R de 28%), nunca abaixo do salário mínimo'
        : '28% do faturamento, nunca abaixo do salário mínimo',
  )

  // ISS rate: edited as text, applied on Enter or blur.
  let issDraft = $state<string | null>(null)
  let issInvalid = $state(false)

  function commitIss() {
    if (issDraft == null) return
    const rate = parseRateInput(issDraft)
    if (rate == null || rate > 0.1) {
      issInvalid = true
      return
    }
    tax.updateSettings({ issRate: rate })
    issDraft = null
  }

  function textKeys(event: KeyboardEvent, commit: () => void, reset: () => void) {
    if (event.key === 'Enter') {
      event.preventDefault()
      commit()
    } else if (event.key === 'Escape') {
      reset()
    }
  }
</script>

<aside class="settings" aria-labelledby="tax-settings-title">
  <h2 id="tax-settings-title" class="title">Configurações</h2>

  <section class="block" aria-labelledby="tax-company">
    <h3 id="tax-company">Empresa</h3>

    <fieldset class="field">
      <legend class="label">Regime atual</legend>
      <div class="radios">
        {#each REGIMES as r (r.id)}
          <label class="radio" class:checked={s.regime === r.id}>
            <input
              type="radio"
              name="tax-regime"
              value={r.id}
              checked={s.regime === r.id}
              onchange={() => tax.updateSettings({ regime: r.id })}
            />
            <span class="radio-text">
              <span class="radio-label">{REGIME_LABEL[r.id]}</span>
              <span class="radio-hint">{r.hint}</span>
            </span>
          </label>
        {/each}
      </div>
      {#if s.regime === 'mei'}
        <p class="hint">No MEI não há pró-labore: o INSS do titular já vem no DAS.</p>
      {/if}
    </fieldset>

    <div class="field">
      <span class="label" id="tax-cnae-label">Atividade (CNAE)</span>
      {#if tax.catalog}
        <CnaePicker
          id="tax-cnae"
          labelledby="tax-cnae-label"
          options={tax.catalog.cnaes}
          value={cnae}
          onchange={(code) => tax.updateSettings({ cnae: code })}
        />
        {#if cnae}
          <ul class="rules">
            <li>
              <Icon name="layers" size={13} />
              {ACTIVITY_LABEL[cnae.activity]}
            </li>
            <li class={cnae.meiAllowed ? 'ok' : 'no'}>
              <Icon name={cnae.meiAllowed ? 'check' : 'x'} size={13} />
              {cnae.meiAllowed ? `Permitido ao MEI como ${cnae.meiOccupation ?? 'ocupação do Anexo XI'}` : 'Não permitido ao MEI'}
            </li>
            {#if cnae.note}
              <li class="note"><Icon name="info" size={13} /> {cnae.note}</li>
            {/if}
          </ul>
        {/if}
      {:else if tax.catalogError}
        <select
          class="input"
          aria-labelledby="tax-cnae-label"
          value={s.activity}
          onchange={(e) => tax.updateSettings({ activity: e.currentTarget.value as SimplesActivity })}
        >
          <option value="fatorR">TI com Fator R (Anexo III ou V)</option>
          <option value="annexIii">Sempre Anexo III</option>
        </select>
        <p class="hint warn">Não foi possível carregar a lista de CNAEs: {tax.catalogError}</p>
      {:else}
        <div class="placeholder"><span class="spinner"></span> Carregando atividades…</div>
      {/if}
    </div>

    <div class="row2">
      <div class="field">
        <label class="label" for="tax-opening">Mês de abertura</label>
        <MonthInput
          id="tax-opening"
          label="Mês de abertura (opcional, MM/AAAA)"
          value={s.openingMonth}
          floatMessage
          onchange={(month) => {
            if (month !== s.openingMonth) tax.updateSettings({ openingMonth: month })
          }}
        />
        <span class="hint">Opcional</span>
      </div>
      <div class="field">
        <label class="label" for="tax-iss">ISS do município</label>
        <div class="suffix" class:invalid={issInvalid}>
          <input
            id="tax-iss"
            type="text"
            inputmode="decimal"
            maxlength="5"
            aria-invalid={issInvalid}
            title={issInvalid ? 'Use um número entre 0 e 10, por exemplo 5 ou 2,5' : 'Usado no Lucro Presumido'}
            value={issDraft ?? formatRateInput(s.issRate)}
            onfocus={() => (issDraft = formatRateInput(s.issRate))}
            oninput={(e) => {
              issDraft = e.currentTarget.value
              issInvalid = false
            }}
            onblur={() => {
              commitIss()
              issDraft = null
              issInvalid = false
            }}
            onkeydown={(e) =>
              textKeys(e, commitIss, () => {
                issDraft = formatRateInput(s.issRate)
                issInvalid = false
              })}
          />
          <span aria-hidden="true">%</span>
        </div>
        <span class="hint">Recife: 5%</span>
      </div>
    </div>
  </section>

  {#if s.regime !== 'mei'}
  <section class="block" aria-labelledby="tax-owner">
    <h3 id="tax-owner">Pró-labore</h3>
    <div class="radios">
      <label class="radio" class:checked={s.proLaboreMode === 'auto'}>
        <input type="radio" name="tax-prolabore" checked={s.proLaboreMode === 'auto'} onchange={() => tax.setProLaboreMode('auto')} />
        <span class="radio-text">
          <span class="radio-label">Automático</span>
          <span class="radio-hint">{autoHint}</span>
        </span>
      </label>
      <label class="radio" class:checked={s.proLaboreMode === 'fixed'}>
        <input type="radio" name="tax-prolabore" checked={s.proLaboreMode === 'fixed'} onchange={() => tax.setProLaboreMode('fixed')} />
        <span class="radio-text">
          <span class="radio-label">Valor fixo</span>
          <span class="radio-hint">O valor que você já tira por mês</span>
        </span>
      </label>
    </div>
    {#if s.proLaboreMode === 'fixed'}
      <MoneyField
        id="tax-prolabore"
        label="Pró-labore fixo por mês"
        value={s.proLaboreCents}
        onchange={(v) => tax.updateSettings({ proLaboreCents: v ?? 0 })}
      />
    {/if}

    <div class="field">
      <label class="label" for="tax-payroll">Outras despesas com folha</label>
      <MoneyField
        id="tax-payroll"
        label="Outras despesas com folha por mês"
        value={s.payrollCents}
        onchange={(v) => tax.updateSettings({ payrollCents: v ?? 0 })}
      />
      <span class="hint">Salários, 13º e FGTS por mês; contam para o Fator R</span>
    </div>

    <div class="field">
      <span class="label" id="tax-dependents-label">Dependentes</span>
      <div class="stepper" role="group" aria-labelledby="tax-dependents-label">
        <button
          type="button"
          aria-label="Menos um dependente"
          disabled={s.dependents <= 0}
          onclick={() => tax.updateSettings({ dependents: s.dependents - 1 })}
        >
          <Icon name="minus" size={14} />
        </button>
        <span class="num" aria-live="polite">{s.dependents}</span>
        <button
          type="button"
          aria-label="Mais um dependente"
          disabled={s.dependents >= 20}
          onclick={() => tax.updateSettings({ dependents: s.dependents + 1 })}
        >
          <Icon name="plus" size={14} />
        </button>
      </div>
      <span class="hint">Reduzem o IRRF do pró-labore</span>
    </div>
  </section>
  {/if}

  <section class="block" aria-labelledby="tax-plan">
    <h3 id="tax-plan">Meta</h3>
    <div class="field">
      <label class="label" for="tax-desired">Quanto quero tirar por mês</label>
      <MoneyField
        id="tax-desired"
        label="Quanto quero tirar por mês"
        placeholder="Opcional"
        allowEmpty
        value={s.desiredNetCents}
        onchange={(v) => tax.updateSettings({ desiredNetCents: v ? v : null })}
      />
      <span class="hint">Calcula quanto faturar, em “Quanto cobrar”</span>
    </div>
  </section>
</aside>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--s-5);
    min-height: 0;
    padding: 20px var(--s-4) var(--s-6);
    border-right: 1px solid var(--border);
    background: var(--surface);
    overflow-y: auto;
  }

  .title {
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.02em;
    text-transform: uppercase;
    color: var(--text-3);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  h3 {
    font-size: 13.5px;
    font-weight: 600;
    color: var(--text);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  .label {
    padding: 0;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-2);
  }

  .hint {
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-3);
  }

  .hint.warn {
    color: var(--warn);
  }

  .row2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s-3);
  }

  /* ------------------------------------------------------------ radios */

  .radios {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .radio {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition:
      border-color 0.12s var(--ease),
      background-color 0.12s var(--ease);
  }

  .radio:hover {
    background: var(--surface-2);
  }

  .radio.checked {
    border-color: var(--accent);
    background: var(--accent-soft);
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
    gap: 1px;
    min-width: 0;
  }

  .radio-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
  }

  .radio-hint {
    font-size: 12px;
    line-height: 1.35;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ CNAE rules */

  .rules {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-2);
  }

  .rules li {
    display: flex;
    align-items: flex-start;
    gap: 6px;
  }

  .rules li :global(svg) {
    margin-top: 2px;
    color: var(--text-3);
  }

  .rules li.ok :global(svg) {
    color: var(--ok);
  }

  .rules li.no {
    color: var(--text-2);
  }

  .rules li.no :global(svg) {
    color: var(--danger);
  }

  .rules li.note {
    color: var(--warn);
  }

  .rules li.note :global(svg) {
    color: var(--warn);
  }

  .placeholder {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 52px;
    padding: 0 12px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius-sm);
    font-size: 12.5px;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ text inputs */

  .input {
    width: 100%;
  }

  .suffix {
    display: flex;
    align-items: center;
    height: 32px;
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
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  .suffix span {
    margin-left: 4px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  /* ------------------------------------------------------------ stepper */

  .stepper {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    height: 32px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    overflow: hidden;
  }

  .stepper button {
    display: grid;
    place-items: center;
    width: 32px;
    height: 100%;
    border: 0;
    background: transparent;
    color: var(--text-2);
  }

  .stepper button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text);
  }

  .stepper button:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .stepper span {
    min-width: 36px;
    font-size: 13px;
    font-weight: 500;
    text-align: center;
  }
</style>

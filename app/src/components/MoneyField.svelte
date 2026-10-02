<script lang="ts">
  import { formatAmount, parseMoneyInput } from '../lib/format'

  interface Props {
    /** Amount in cents; null = empty. */
    value: number | null
    /** Called on Enter or when the field loses focus, only when the amount changed. */
    onchange: (cents: number | null) => void
    label: string
    id?: string
    placeholder?: string
    /** Empty means null (otherwise empty means zero). */
    allowEmpty?: boolean
    disabled?: boolean
  }

  let { value, onchange, label, id, placeholder = '0,00', allowEmpty = false, disabled = false }: Props = $props()

  let focused = $state(false)
  let draft = $state('')
  let invalid = $state(false)

  const formatted = $derived(value == null ? '' : formatAmount(value))
  const shown = $derived(focused ? draft : formatted)

  /**
   * Applies the draft. Returns the amount applied, or undefined when the draft is invalid. The
   * caller must not read `value` right after: inside a keyed list the prop only follows on the next
   * update.
   */
  function commit(): { cents: number | null } | undefined {
    const parsed = parseMoneyInput(draft)
    if (parsed.kind === 'invalid' || (parsed.kind === 'value' && parsed.cents < 0)) {
      invalid = true
      return undefined
    }
    const next = parsed.kind === 'empty' ? (allowEmpty ? null : 0) : parsed.cents
    if (next !== value) onchange(next)
    return { cents: next }
  }

  function onFocus() {
    draft = formatted
    invalid = false
    focused = true
  }

  function onBlur() {
    // An invalid amount is dropped: the field shows the last valid one again.
    if (!commit()) invalid = false
    focused = false
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault()
      const applied = commit()
      if (applied) draft = applied.cents == null ? '' : formatAmount(applied.cents)
    } else if (event.key === 'Escape') {
      draft = formatted
      invalid = false
    }
  }
</script>

<div class="money" class:invalid class:disabled>
  <span class="prefix" aria-hidden="true">R$</span>
  <input
    {id}
    type="text"
    inputmode="decimal"
    autocomplete="off"
    aria-label={label}
    aria-invalid={invalid}
    title={invalid ? 'Valor inválido. Use o formato 1.234,56' : 'Enter ou sair do campo aplica'}
    {placeholder}
    {disabled}
    value={shown}
    oninput={(e) => {
      draft = e.currentTarget.value
      invalid = false
    }}
    onfocus={onFocus}
    onblur={onBlur}
    onkeydown={onKeydown}
  />
</div>

<style>
  .money {
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

  .money:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .money.invalid {
    border-color: var(--danger);
    box-shadow: 0 0 0 3px var(--danger-soft);
  }

  .money.disabled {
    opacity: 0.55;
  }

  .prefix {
    margin-right: 6px;
    font-size: 12.5px;
    color: var(--text-3);
  }

  input {
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

  input::placeholder {
    color: var(--text-3);
  }
</style>

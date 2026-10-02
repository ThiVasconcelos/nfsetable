<script lang="ts">
  // A month field ("MM/AAAA") with a calendar: the typing mask inserts the slash, the text is
  // normalized on Enter or when the field is left ("3/26", "032026", "mar/26", "março 2026"...),
  // and the calendar button opens a month picker (year header with arrows, 3 x 4 grid of months,
  // full keyboard support). No <input type="month">: it has no picker on WebKitGTK (Linux).
  import { tick } from 'svelte'
  import { addMonths, formatMonth, formatMonthLong, formatMonthShort, maskMonthTyping, monthKeyOf, parseMonthInput } from '../lib/format'
  import Icon from './Icon.svelte'

  interface Props {
    /** Month key `yyyy-mm`, or null. */
    value: string | null
    id?: string
    /** Accessible name of the field. */
    label: string
    /** Id of an element that describes the field (a hint below it). */
    describedby?: string
    placeholder?: string
    /** Months already in the list: marked in the calendar. */
    taken?: ReadonlySet<string> | null
    /** An empty field means "no month" (otherwise it is an error). */
    allowEmpty?: boolean
    /** Error from the parent (e.g. a month already in the list), shown like the field's own. */
    error?: string | null
    /** Show the parent's error text (false: only the red border; the parent explains it). */
    errorText?: boolean
    /** The parent shows the messages itself (only the red border here). */
    quiet?: boolean
    /** The message floats below the field instead of taking space (table cells). */
    floatMessage?: boolean
    size?: 'sm' | 'md'
    autofocus?: boolean
    /** Select the text when the field gets the focus (editors in the table). */
    selectOnFocus?: boolean
    /** A valid month was committed: Enter, picked in the calendar, or the field left with a change. */
    onchange: (month: string | null, how: 'enter' | 'blur' | 'pick') => void
    /** Esc in the field (calendar closed): the text went back to the value. */
    oncancel?: () => void
    /** The focus left the field and its calendar (after `onchange`, if any). */
    onleave?: (valid: boolean) => void
  }

  let {
    value,
    id,
    label,
    describedby,
    placeholder = 'MM/AAAA',
    taken = null,
    allowEmpty = true,
    error = null,
    errorText = true,
    quiet = false,
    floatMessage = false,
    size = 'md',
    autofocus = false,
    selectOnFocus = false,
    onchange,
    oncancel,
    onleave,
  }: Props = $props()

  const PICKER_WIDTH = 252
  const uid = $props.id()
  const messageId = `month-msg-${uid}`
  const today = monthKeyOf()

  // svelte-ignore state_referenced_locally
  let text = $state(value ? formatMonth(value) : '')
  let invalid = $state(false)
  let focused = $state(false)
  let open = $state(false)
  let viewYear = $state(Number(today.slice(0, 4)))
  let cursor = $state(today)
  let yearEditing = $state(false)
  let yearDraft = $state('')
  let position = $state({ top: 0, left: 0 })
  let root: HTMLDivElement | undefined = $state()
  let input: HTMLInputElement | undefined = $state()
  let popover: HTMLDivElement | undefined = $state()
  /** Last month handed to `onchange` (a blur right after Enter or a pick does not repeat it). */
  // svelte-ignore state_referenced_locally
  let committed: string | null = value
  // svelte-ignore state_referenced_locally
  let synced: string | null = value

  // A new value from the parent replaces the text, unless the user is typing in the field.
  $effect(() => {
    const v = value
    if (v === synced) return
    synced = v
    committed = v
    if (!focused) {
      text = v ? formatMonth(v) : ''
      invalid = false
    }
  })

  $effect(() => {
    if (autofocus && input) {
      input.focus()
      if (selectOnFocus) input.select()
    }
  })

  const parsed = $derived(parseMonthInput(text))
  /** Month shown as chosen in the calendar: the valid text, else the value. */
  const selected = $derived(parsed.kind === 'value' ? parsed.month : value)
  const message = $derived(invalid ? `Use MM/AAAA, por exemplo ${formatMonth(`${today.slice(0, 4)}-03`)}` : errorText ? error : null)
  const months = $derived(Array.from({ length: 12 }, (_, i) => `${viewYear}-${String(i + 1).padStart(2, '0')}`))

  function parse(): { ok: true; month: string | null } | { ok: false } {
    const p = parseMonthInput(text)
    if (p.kind === 'value') return { ok: true, month: p.month }
    if (p.kind === 'empty' && allowEmpty) return { ok: true, month: null }
    return { ok: false }
  }

  /** Normalizes the text and hands a valid month to the parent. */
  function commit(how: 'enter' | 'blur'): boolean {
    const r = parse()
    if (!r.ok) {
      invalid = true
      return false
    }
    invalid = false
    text = r.month ? formatMonth(r.month) : ''
    if (how === 'enter' || r.month !== committed) {
      committed = r.month
      onchange(r.month, how)
    }
    return true
  }

  function onInput(event: Event) {
    const el = event.currentTarget as HTMLInputElement
    // Typed or pasted text gets the mask (also over a selection); deleting never does.
    const inputType = (event as InputEvent).inputType ?? ''
    const grew = inputType ? inputType.startsWith('insert') : el.value.length > text.length
    const next = maskMonthTyping(el.value, grew)
    text = next
    if (el.value !== next) el.value = next
    invalid = false
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      event.preventDefault()
      event.stopPropagation()
      commit('enter')
    } else if (event.key === 'Escape') {
      if (open) {
        event.preventDefault()
        event.stopPropagation()
        close(true)
        return
      }
      const reverted = value ? formatMonth(value) : ''
      // Nothing typed to undo and no editor to cancel: Esc goes on to the container (a popover closes).
      if (!oncancel && text === reverted && !invalid) return
      event.preventDefault()
      event.stopPropagation()
      text = reverted
      invalid = false
      oncancel?.()
    } else if (event.key === 'ArrowDown' && (event.altKey || !open)) {
      event.preventDefault()
      event.stopPropagation()
      void show(true)
    }
  }

  function onFocusIn(event: FocusEvent) {
    focused = true
    if (selectOnFocus && event.target === input) input?.select()
  }

  function onFocusOut(event: FocusEvent) {
    const next = event.relatedTarget as Node | null
    if (next && root?.contains(next)) return
    if (!next) {
      // Focus lost to nothing: a focused element inside went away (the year field, a month of the
      // previous year) and the focus is about to move inside again, or a click landed on a blank
      // area. Decide once the pending focus moves are done.
      setTimeout(() => {
        if (focused && !root?.contains(document.activeElement)) leave()
      }, 0)
      return
    }
    leave()
  }

  /** The focus left the field and its calendar: close, normalize and tell the parent. */
  function leave() {
    focused = false
    open = false
    yearEditing = false
    // Not inside `onleave?.(...)`: an optional call skips its arguments when there is no handler.
    const ok = commit('blur')
    onleave?.(ok)
  }

  // ------------------------------------------------------------ calendar

  async function show(focusGrid: boolean) {
    if (!root) return
    const start = selected ?? today
    cursor = start
    viewYear = Number(start.slice(0, 4))
    yearEditing = false
    open = true
    place()
    await tick()
    place()
    if (focusGrid) focusCursor()
  }

  function close(refocus: boolean) {
    open = false
    yearEditing = false
    if (refocus) input?.focus({ preventScroll: true })
  }

  /** Below the field when it fits, else above; always inside the window. */
  function place() {
    if (!root) return
    const r = root.getBoundingClientRect()
    const h = popover?.offsetHeight ?? 270
    const below = r.bottom + 6 + h <= window.innerHeight - 8
    position = {
      top: below ? r.bottom + 6 : Math.max(8, r.top - 6 - h),
      left: Math.max(8, Math.min(r.left, window.innerWidth - PICKER_WIDTH - 8)),
    }
  }

  function focusCursor() {
    void tick().then(() => {
      popover?.querySelector<HTMLButtonElement>(`[data-month="${cursor}"]`)?.focus({ preventScroll: true })
    })
  }

  function moveCursor(delta: number) {
    cursor = addMonths(cursor, delta)
    viewYear = Number(cursor.slice(0, 4))
    focusCursor()
  }

  function shiftYear(delta: number) {
    viewYear = Math.min(2099, Math.max(2000, viewYear + delta))
    cursor = `${viewYear}-${cursor.slice(5, 7)}`
  }

  function pick(month: string) {
    text = formatMonth(month)
    invalid = false
    committed = month
    close(true)
    onchange(month, 'pick')
  }

  function clear() {
    text = ''
    invalid = false
    committed = null
    close(true)
    onchange(null, 'pick')
  }

  function onGridKeydown(event: KeyboardEvent) {
    const moves: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -3, ArrowDown: 3, PageUp: -12, PageDown: 12 }
    if (event.key in moves) {
      event.preventDefault()
      moveCursor(moves[event.key])
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault()
      cursor = `${viewYear}-${event.key === 'Home' ? '01' : '12'}`
      focusCursor()
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      pick(cursor)
    }
  }

  function onPopoverKeydown(event: KeyboardEvent) {
    event.stopPropagation()
    if (event.key === 'Escape') {
      event.preventDefault()
      if (yearEditing) yearEditing = false
      else close(true)
    }
  }

  async function startYearEdit() {
    yearDraft = String(viewYear)
    yearEditing = true
    await tick()
    popover?.querySelector<HTMLInputElement>('.year-input')?.select()
  }

  function applyYear() {
    const y = Number(yearDraft)
    if (/^\d{4}$/.test(yearDraft) && y >= 2000 && y <= 2099) {
      viewYear = y
      cursor = `${y}-${cursor.slice(5, 7)}`
    }
    yearEditing = false
    focusCursor()
  }

  $effect(() => {
    if (!open) return
    const onPointer = (e: PointerEvent) => {
      if (root?.contains(e.target as Node)) return
      // The focused month button goes away with the calendar: leave the field explicitly.
      const hadFocus = !!popover?.contains(document.activeElement)
      close(false)
      if (hadFocus) leave()
    }
    // Scrolling keeps the calendar next to the field; it closes when the field leaves the window.
    let frame = 0
    const onScroll = (e: Event) => {
      if (e.target instanceof Node && popover?.contains(e.target)) return
      cancelAnimationFrame(frame)
      frame = requestAnimationFrame(() => {
        const r = root?.getBoundingClientRect()
        if (!r || r.bottom < 0 || r.top > window.innerHeight) close(false)
        else place()
      })
    }
    const onResize = () => close(false)
    document.addEventListener('pointerdown', onPointer, true)
    document.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', onResize)
    return () => {
      cancelAnimationFrame(frame)
      document.removeEventListener('pointerdown', onPointer, true)
      document.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', onResize)
    }
  })
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div
  bind:this={root}
  class="month-field {size}"
  class:invalid={invalid || !!error}
  onfocusin={onFocusIn}
  onfocusout={onFocusOut}
  onclick={(e) => e.stopPropagation()}
  ondblclick={(e) => e.stopPropagation()}
>
  <div class="box">
    <input
      bind:this={input}
      {id}
      class="field num"
      type="text"
      inputmode="text"
      autocomplete="off"
      spellcheck="false"
      maxlength="20"
      {placeholder}
      aria-label={label}
      aria-invalid={invalid || !!error}
      aria-describedby={[message && !quiet ? messageId : '', describedby ?? ''].filter(Boolean).join(' ') || undefined}
      value={text}
      oninput={onInput}
      onkeydown={onKeydown}
    />
    <button
      type="button"
      class="cal"
      tabindex="-1"
      aria-label="Abrir calendário"
      aria-haspopup="dialog"
      aria-expanded={open}
      title="Escolher no calendário"
      onmousedown={(e) => e.preventDefault()}
      onclick={() => (open ? close(true) : void show(true))}
    >
      <Icon name="calendar" size={size === 'sm' ? 13 : 14} />
    </button>
  </div>
  {#if message && !quiet}
    <span class="message" class:float={floatMessage} id={messageId} role={invalid ? 'alert' : undefined}>{message}</span>
  {/if}

  {#if open}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      bind:this={popover}
      class="picker"
      role="dialog"
      aria-label="Escolher o mês"
      tabindex="-1"
      style="top: {position.top}px; left: {position.left}px; width: {PICKER_WIDTH}px"
      onkeydown={onPopoverKeydown}
    >
      <div class="head">
        <button type="button" class="nav" aria-label="Ano anterior" title="Ano anterior (PageUp)" onclick={() => shiftYear(-1)}>
          <Icon name="chevronLeft" size={15} />
        </button>
        {#if yearEditing}
          <input
            class="year-input num"
            type="text"
            inputmode="numeric"
            maxlength="4"
            aria-label="Ano"
            bind:value={yearDraft}
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault()
                applyYear()
              }
            }}
            onblur={() => yearEditing && applyYear()}
          />
        {:else}
          <button type="button" class="year num" title="Digitar o ano" onclick={startYearEdit}>{viewYear}</button>
        {/if}
        <button type="button" class="nav" aria-label="Próximo ano" title="Próximo ano (PageDown)" onclick={() => shiftYear(1)}>
          <Icon name="chevronRight" size={15} />
        </button>
      </div>
      <div class="grid" role="grid" aria-label="Meses de {viewYear}" tabindex="-1">
        {#each [0, 1, 2, 3] as r (r)}
          <div class="row" role="row">
            <!-- Keyed by position: moving to another year keeps the focused button in place. -->
            {#each months.slice(r * 3, r * 3 + 3) as month, i (r * 3 + i)}
              {@const isTaken = !!taken?.has(month)}
              <button
                type="button"
                role="gridcell"
                data-month={month}
                tabindex={month === cursor ? 0 : -1}
                aria-selected={month === selected}
                aria-label="{formatMonthLong(month)}{isTaken ? ', já está na lista' : ''}{month === today ? ', mês atual' : ''}"
                title={isTaken ? 'Já está na lista' : undefined}
                class:selected={month === selected}
                class:current={month === today}
                class:taken={isTaken}
                onclick={() => pick(month)}
                onkeydown={onGridKeydown}
                onfocus={() => (cursor = month)}
              >
                {formatMonthShort(month)}
                {#if isTaken}<span class="dot" aria-hidden="true"></span>{/if}
              </button>
            {/each}
          </div>
        {/each}
      </div>
      <div class="foot">
        <button type="button" class="link" onclick={() => pick(today)}>Este mês</button>
        {#if allowEmpty && (value || text)}
          <button type="button" class="link muted" onclick={clear}>Limpar</button>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .month-field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .box {
    position: relative;
    display: flex;
    align-items: center;
  }

  .field {
    width: 100%;
    min-width: 0;
    height: 32px;
    padding: 0 30px 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font-size: 13px;
    outline: none;
    transition:
      border-color 0.12s var(--ease),
      box-shadow 0.12s var(--ease);
  }

  .sm .field {
    height: 28px;
    padding: 0 26px 0 8px;
    font-size: 12.5px;
  }

  .field::placeholder {
    color: var(--text-3);
  }

  .field:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .invalid .field {
    border-color: var(--danger);
  }

  .invalid .field:focus {
    box-shadow: 0 0 0 3px var(--danger-soft);
  }

  .cal {
    position: absolute;
    right: 3px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border: 0;
    border-radius: 4px;
    background: transparent;
    color: var(--text-3);
  }

  .sm .cal {
    width: 20px;
    height: 20px;
  }

  .cal:hover,
  .cal[aria-expanded='true'] {
    background: var(--surface-3);
    color: var(--text);
  }

  .message {
    font-size: 11.5px;
    line-height: 1.35;
    color: var(--danger);
  }

  .message.float {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 6;
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    white-space: nowrap;
  }

  /* ------------------------------------------------------------ picker */

  .picker {
    position: fixed;
    z-index: 70;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    animation: pop 0.12s var(--ease);
  }

  .picker:focus {
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }

  .nav {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-2);
  }

  .nav:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .year {
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
  }

  .year:hover {
    background: var(--surface-2);
  }

  .year-input {
    width: 72px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid var(--accent);
    border-radius: var(--radius-sm);
    background: var(--surface);
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    text-align: center;
    outline: none;
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .grid {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .grid:focus {
    outline: none;
  }

  .row {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 4px;
  }

  .row button {
    position: relative;
    height: 34px;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text);
    font-size: 13px;
    transition:
      background-color 0.1s var(--ease),
      border-color 0.1s var(--ease);
  }

  .row button:hover {
    border-color: var(--border-strong);
    background: var(--surface);
  }

  .row button:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft-2);
  }

  .row button.current {
    border-color: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .row button.selected {
    border-color: var(--accent);
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }

  .row button.taken:not(.selected) {
    color: var(--text-3);
    background: transparent;
    border-color: var(--border);
    border-style: dashed;
  }

  .dot {
    position: absolute;
    top: 5px;
    right: 6px;
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--text-3);
  }

  .foot {
    display: flex;
    justify-content: space-between;
    padding-top: 2px;
    font-size: 12.5px;
  }

  .muted {
    color: var(--text-3);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(-2px);
    }
  }
</style>

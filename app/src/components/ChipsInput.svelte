<script lang="ts">
  // A list of short texts edited as chips: type and press Enter (or a comma) to add, Backspace on an
  // empty field removes the last one, × removes one.
  import { sameText } from '../lib/format'
  import Icon from './Icon.svelte'

  interface Props {
    values: string[]
    /** Accessible name of the text field. */
    label: string
    placeholder?: string
    id?: string
    /** Monospace chips (file name patterns). */
    mono?: boolean
    maxLength?: number
    /** Text typed and not added yet (for live previews). */
    draft?: string
    onchange: (values: string[]) => void
  }

  let { values, label, placeholder = '', id, mono = false, maxLength = 80, draft = $bindable(''), onchange }: Props = $props()

  let input: HTMLInputElement | undefined = $state()

  function add(text: string) {
    const t = text.trim()
    draft = ''
    if (!t || values.some((v) => sameText(v, t))) return
    onchange([...values, t])
  }

  function remove(index: number) {
    onchange(values.filter((_, i) => i !== index))
    input?.focus()
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ',') {
      if (!draft.trim()) return
      event.preventDefault()
      add(draft)
    } else if (event.key === 'Backspace' && !draft && values.length) {
      event.preventDefault()
      onchange(values.slice(0, -1))
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="chips-input" onclick={() => input?.focus()}>
  {#each values as value, i (value)}
    <span class="chip" class:mono>
      <span class="text">{value}</span>
      <button type="button" aria-label="Remover {value}" title="Remover" onclick={() => remove(i)}>
        <Icon name="x" size={11} strokeWidth={2.2} />
      </button>
    </span>
  {/each}
  <input
    bind:this={input}
    {id}
    type="text"
    aria-label={label}
    placeholder={values.length ? '' : placeholder}
    maxlength={maxLength}
    bind:value={draft}
    onkeydown={onKeydown}
    onblur={() => add(draft)}
  />
</div>

<style>
  .chips-input {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-height: 34px;
    padding: 3px 6px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    background: var(--surface);
    cursor: text;
    transition:
      border-color 0.12s var(--ease),
      box-shadow 0.12s var(--ease);
  }

  .chips-input:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft-2);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    max-width: 100%;
    height: 24px;
    padding: 0 2px 0 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 12.5px;
  }

  .chip.mono .text {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .text {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip button {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: inherit;
  }

  .chip button:hover {
    background: var(--accent-soft-2);
  }

  input {
    flex: 1;
    min-width: 120px;
    height: 26px;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 13px;
  }

  input::placeholder {
    color: var(--text-3);
  }
</style>

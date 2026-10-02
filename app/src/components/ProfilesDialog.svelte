<script lang="ts">
  // "Perfis e regras": the built-in profiles and the user's rules (how a document is recognized:
  // file name patterns and texts; what it sets: type and kind; and, for the profiles saved from a
  // region, where the value is). Rules are created, edited and deleted here; saving reads the
  // files again with them.
  import { tick } from 'svelte'
  import { errorMessage } from '../lib/api'
  import { formatInt, normalizeText, plural } from '../lib/format'
  import { describeRule } from '../lib/labels'
  import { classifies, hasValueRules, nameMatches } from '../lib/rules'
  import { KIND_LABEL, store, type RuleSeed } from '../lib/store.svelte'
  import type { DocKind, Profile } from '../lib/types'
  import ChipsInput from './ChipsInput.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  type KindChoice = DocKind | 'keep'

  /** Types often used for expenses, offered next to the ones of the table. */
  const EXPENSE_TYPES = ['Internet', 'Aluguel', 'Contador', 'Energia', 'Telefone', 'Softwares']
  const KIND_CHOICES: { id: KindChoice; label: string }[] = [
    { id: 'revenue', label: 'Receita' },
    { id: 'expense', label: 'Despesa' },
    { id: 'keep', label: 'Não mudar' },
  ]
  const PREVIEW_MAX = 6

  let mode = $state<'list' | 'edit'>('list')
  /** Opened from a note ("Criar regra a partir desta nota"): closing after saving goes back to it. */
  let fromSeed = $state(false)
  let editing = $state<Profile | null>(null)
  let name = $state('')
  let patterns = $state<string[]>([])
  let patternDraft = $state('')
  let texts = $state<string[]>([])
  let textDraft = $state('')
  let docType = $state('')
  let kind = $state<KindChoice>('keep')
  let tried = $state(false)
  let saving = $state(false)
  let saveError = $state<string | null>(null)
  let confirming = $state<string | null>(null)
  let deleting = $state<string | null>(null)

  const userCount = $derived(store.profiles.filter((p) => !p.builtin).length)

  // Opening: the editor when a rule was asked for (from a note), else the list.
  let wasOpen = false
  $effect(() => {
    const open = store.profilesOpen
    if (open && !wasOpen) {
      confirming = null
      void store.loadProfiles()
      const seed = store.ruleSeed
      if (seed) {
        fromSeed = true
        void startEdit(null, seed)
      } else {
        fromSeed = false
        mode = 'list'
      }
    }
    wasOpen = open
  })

  function close() {
    store.profilesOpen = false
    store.ruleSeed = null
    mode = 'list'
  }

  async function startEdit(profile: Profile | null, seed: RuleSeed | null = null) {
    editing = profile
    name = profile?.name ?? seed?.name ?? ''
    patterns = [...(profile?.namePatterns ?? seed?.namePatterns ?? [])]
    texts = [...(profile?.fingerprint ?? seed?.fingerprint ?? [])]
    docType = profile?.docType ?? seed?.docType ?? ''
    kind = (profile ? profile.kind : seed?.kind) ?? 'keep'
    patternDraft = ''
    textDraft = ''
    tried = false
    saveError = null
    mode = 'edit'
    await tick()
    document.getElementById(seed ? 'rule-patterns' : 'rule-name')?.focus()
  }

  function backToList() {
    if (fromSeed) {
      close()
      return
    }
    mode = 'list'
  }

  // ------------------------------------------------------------ editor state

  const allPatterns = $derived([...patterns, patternDraft.trim()].filter(Boolean))
  const allTexts = $derived([...texts, textDraft.trim()].filter(Boolean))
  const valueRules = $derived(editing ? (editing.fields.net_value ?? []) : [])
  /** Files of the table whose name matches the patterns being typed. */
  const matches = $derived(allPatterns.length ? store.rows.filter((r) => nameMatches(allPatterns, r.name)) : [])
  const typeSuggestions = $derived.by(() => {
    const q = normalizeText(docType)
    const all = [...new Set([...EXPENSE_TYPES, ...store.knownTypes])].filter((t) => !['cancelada'].includes(normalizeText(t)))
    return all.filter((t) => normalizeText(t) !== q && (!q || normalizeText(t).includes(q))).slice(0, 8)
  })

  const errors = $derived.by(() => {
    const list: string[] = []
    if (!name.trim()) list.push('Dê um nome para a regra.')
    const sets = !!docType.trim() || kind !== 'keep'
    if (!sets && !valueRules.length) list.push('Escolha um tipo ou a natureza das notas.')
    if (sets && !allPatterns.length && !allTexts.length)
      list.push('Diga como reconhecer as notas: um padrão de nome de arquivo ou um texto.')
    return list
  })

  async function save() {
    tried = true
    if (errors.length || saving) return
    saving = true
    saveError = null
    const profile: Profile = {
      id: editing?.id ?? '',
      name: name.trim(),
      builtin: false,
      fingerprint: allTexts,
      namePatterns: allPatterns,
      docType: docType.trim() || null,
      kind: kind === 'keep' ? null : kind,
      fields: editing ? $state.snapshot(editing.fields) : {},
    }
    try {
      await store.saveRule(profile)
      saving = false
      backToList()
    } catch (e) {
      saving = false
      saveError = errorMessage(e)
    }
  }

  async function remove(profile: Profile) {
    deleting = profile.id
    await store.deleteProfile(profile)
    deleting = null
    confirming = null
  }

  function tagOf(profile: Profile): string {
    if (profile.builtin) return 'Embutido'
    return hasValueRules(profile) ? 'Seu perfil' : 'Sua regra'
  }
</script>

<Modal
  open={store.profilesOpen}
  size="lg"
  title={mode === 'edit' ? (editing ? `Editar “${editing.name}”` : 'Nova regra') : 'Perfis e regras'}
  subtitle={mode === 'edit'
    ? 'Como reconhecer as notas e o que definir nelas. As notas são lidas de novo ao salvar.'
    : 'Regras dizem de que tipo é uma nota e se é receita ou despesa. Perfis também sabem onde fica o valor.'}
  onclose={close}
>
  {#if mode === 'list'}
    <div class="toolbar">
      <button type="button" class="btn btn-primary btn-sm" onclick={() => startEdit(null)}>
        <Icon name="plus" size={14} />
        Nova regra
      </button>
      <span class="count">{plural(store.profiles.length, 'perfil', 'perfis')} · {formatInt(userCount)} seus</span>
    </div>

    <ul class="list">
      {#each store.profiles as profile (profile.id)}
        <li>
          <div class="main">
            <div class="title">
              <span class="name">{profile.name}</span>
              <span class="tag" class:builtin={profile.builtin}>{tagOf(profile)}</span>
            </div>
            <div class="facts">
              <span class="k">Reconhece</span>
              <span class="v">
                {#each profile.namePatterns as p (p)}
                  <span class="chip mono" title="Nome do arquivo"><Icon name="fileText" size={11} />{p}</span>
                {/each}
                {#each profile.fingerprint as f (f)}
                  <span class="chip" title="Texto da nota contém"><Icon name="search" size={11} />“{f}”</span>
                {/each}
                {#if !profile.namePatterns.length && !profile.fingerprint.length}
                  <span class="muted">Qualquer documento</span>
                {/if}
              </span>
            </div>
            {#if classifies(profile)}
              <div class="facts">
                <span class="k">Define</span>
                <span class="v">
                  {#if profile.docType}<span class="chip plain">Tipo {profile.docType}</span>{/if}
                  {#if profile.kind}
                    <span class="chip plain" class:expense={profile.kind === 'expense'}>{KIND_LABEL[profile.kind]}</span>
                  {/if}
                </span>
              </div>
            {/if}
            {#if hasValueRules(profile)}
              <div class="facts">
                <span class="k">Valor</span>
                <ul class="rules">
                  {#each profile.fields.net_value ?? [] as rule, i (i)}
                    <li>{describeRule(rule)}</li>
                  {/each}
                </ul>
              </div>
            {/if}
          </div>
          {#if !profile.builtin}
            <div class="actions">
              {#if confirming === profile.id}
                <span class="confirm">Excluir?</span>
                <button type="button" class="btn btn-sm" onclick={() => (confirming = null)}>Não</button>
                <button
                  type="button"
                  class="btn btn-sm danger-solid"
                  disabled={deleting === profile.id}
                  onclick={() => remove(profile)}
                >
                  {#if deleting === profile.id}<span class="spinner"></span>{/if}
                  Excluir
                </button>
              {:else}
                <button type="button" class="btn btn-ghost btn-sm" onclick={() => startEdit(profile)}>
                  <Icon name="pencil" size={14} />
                  Editar
                </button>
                <button type="button" class="btn btn-ghost btn-sm btn-danger" onclick={() => (confirming = profile.id)}>
                  <Icon name="trash" size={14} />
                  Excluir
                </button>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>

    {#if !userCount}
      <p class="note">
        <Icon name="info" size={15} />
        <span>
          Você ainda não criou regras. Use “Nova regra” ou, na tabela, o menu ⋯ de uma nota e “Criar regra a partir desta
          nota”.
        </span>
      </p>
    {/if}

    {#if store.info?.profilesDir}
      <div class="dir">
        <span>Os perfis e as regras ficam em</span>
        <code class="mono" title={store.info.profilesDir}>{store.info.profilesDir}</code>
      </div>
    {/if}
  {:else}
    <form
      class="editor"
      onsubmit={(e) => {
        e.preventDefault()
        void save()
      }}
    >
      <div class="field">
        <label class="label" for="rule-name">Nome</label>
        <input id="rule-name" class="input" type="text" maxlength="60" placeholder="Por exemplo, Internet" bind:value={name} />
      </div>

      <div class="field">
        <label class="label" for="rule-patterns">Nome do arquivo</label>
        <ChipsInput
          id="rule-patterns"
          label="Padrões do nome do arquivo"
          placeholder="Por exemplo, internet-* (Enter adiciona)"
          mono
          values={patterns}
          bind:draft={patternDraft}
          onchange={(v) => (patterns = v)}
        />
        <p class="hint">
          <code>*</code> vale qualquer sequência e <code>?</code> um caractere (ex.: <code>internet-*.pdf</code>). Sem eles,
          basta o nome conter o texto. Maiúsculas e acentos não importam.
        </p>
      </div>

      <div class="field">
        <label class="label" for="rule-texts">Texto da nota contém</label>
        <ChipsInput
          id="rule-texts"
          label="Textos que a nota contém"
          placeholder="Por exemplo, o CNPJ ou o nome de quem emitiu"
          values={texts}
          bind:draft={textDraft}
          onchange={(v) => (texts = v)}
        />
        <p class="hint">Todos os textos precisam aparecer na nota. Opcional se houver padrão de nome.</p>
      </div>

      <div class="row2">
        <div class="field">
          <label class="label" for="rule-type">Tipo</label>
          <input
            id="rule-type"
            class="input"
            type="text"
            maxlength="40"
            placeholder="Não mudar"
            autocomplete="off"
            bind:value={docType}
          />
          {#if typeSuggestions.length}
            <div class="suggestions" role="group" aria-label="Tipos sugeridos">
              {#each typeSuggestions as t (t)}
                <button type="button" class="suggestion" onclick={() => (docType = t)}>{t}</button>
              {/each}
            </div>
          {/if}
        </div>

        <fieldset class="field">
          <legend class="label">Natureza</legend>
          <div class="seg" role="radiogroup" aria-label="Natureza">
            {#each KIND_CHOICES as c (c.id)}
              <label class="seg-option" class:active={kind === c.id} class:expense={c.id === 'expense'}>
                <input type="radio" name="rule-kind" value={c.id} checked={kind === c.id} onchange={() => (kind = c.id)} />
                {c.label}
              </label>
            {/each}
          </div>
        </fieldset>
      </div>

      {#if valueRules.length}
        <div class="value-rules">
          <span class="label">Leitura do valor</span>
          <ul class="rules">
            {#each valueRules as rule, i (i)}
              <li>{describeRule(rule)}</li>
            {/each}
          </ul>
          <p class="hint">Continua igual: a regra só ganha os padrões, o tipo e a natureza.</p>
        </div>
      {/if}

      <div class="preview" aria-live="polite">
        <Icon name="fileText" size={15} />
        <div>
          {#if allPatterns.length}
            <p class="preview-count">
              <strong>{plural(matches.length, 'arquivo da tabela combina', 'arquivos da tabela combinam')}</strong> pelo nome
            </p>
            {#if matches.length}
              <ul class="matches">
                {#each matches.slice(0, PREVIEW_MAX) as row (row.path)}
                  <li class="mono" title={row.path}>{row.name}</li>
                {/each}
              </ul>
              {#if matches.length > PREVIEW_MAX}
                <p class="more">e mais {formatInt(matches.length - PREVIEW_MAX)}</p>
              {/if}
            {/if}
          {:else}
            <p class="preview-count">Sem padrão de nome: vale para {allTexts.length ? 'as notas com esses textos' : 'qualquer arquivo'}.</p>
          {/if}
          {#if allTexts.length}
            <p class="more">O texto da nota é conferido na próxima leitura.</p>
          {/if}
        </div>
      </div>

      {#if tried && errors.length}
        <ul class="errors" role="alert">
          {#each errors as e (e)}
            <li><Icon name="alertCircle" size={13} /> {e}</li>
          {/each}
        </ul>
      {/if}
      {#if saveError}
        <p class="errors" role="alert"><Icon name="alertCircle" size={13} /> Não foi possível salvar: {saveError}</p>
      {/if}
    </form>
  {/if}

  {#snippet footer()}
    {#if mode === 'edit'}
      <button type="button" class="btn" onclick={backToList}>{fromSeed ? 'Cancelar' : 'Voltar'}</button>
      <span class="spacer"></span>
      <button type="button" class="btn btn-primary" disabled={saving} onclick={() => save()}>
        {#if saving}<span class="spinner"></span>{/if}
        Salvar regra
      </button>
    {:else}
      <span class="spacer"></span>
      <button type="button" class="btn" onclick={close}>Fechar</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s-3);
    margin-bottom: var(--s-3);
  }

  .count {
    font-size: 12.5px;
    color: var(--text-3);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .list > li {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    flex-wrap: wrap;
  }

  .name {
    font-weight: 600;
  }

  .tag {
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
    font-size: 11.5px;
    font-weight: 500;
  }

  .tag.builtin {
    background: var(--surface-3);
    color: var(--text-2);
  }

  .facts {
    display: flex;
    align-items: baseline;
    gap: var(--s-2);
    font-size: 12.5px;
  }

  .k {
    flex: none;
    width: 70px;
    font-size: 12px;
    color: var(--text-3);
  }

  .v {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    min-width: 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 1px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .chip :global(svg) {
    flex: none;
    color: var(--text-3);
  }

  .chip.mono {
    font-family: var(--font-mono);
    font-size: 11.5px;
  }

  .chip.plain {
    background: var(--surface);
  }

  .chip.expense {
    border-color: transparent;
    background: var(--expense-soft);
    color: var(--expense);
  }

  .muted {
    color: var(--text-3);
  }

  .rules {
    margin: 0;
    padding: 0 0 0 16px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }

  .confirm {
    font-size: 12.5px;
    color: var(--text-2);
  }

  .danger-solid {
    border-color: var(--danger);
    background: var(--danger);
    color: #fff;
  }

  .danger-solid:hover:not(:disabled) {
    background: var(--danger);
    filter: brightness(0.95);
  }

  .note {
    display: flex;
    gap: var(--s-2);
    margin-top: var(--s-3);
    padding: var(--s-3);
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .note :global(svg) {
    flex: none;
    margin-top: 1px;
    color: var(--accent-text);
  }

  .dir {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: var(--s-4);
    font-size: 12px;
    color: var(--text-3);
  }

  .dir code {
    padding: 6px 10px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-2);
    overflow-wrap: anywhere;
  }

  /* ------------------------------------------------------------ editor */

  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
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
    line-height: 1.45;
    color: var(--text-3);
  }

  .hint code {
    padding: 0 4px;
    border-radius: 4px;
    background: var(--surface-2);
    font-size: 11.5px;
  }

  .row2 {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--s-4);
    align-items: start;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .suggestion {
    height: 24px;
    padding: 0 9px;
    border: 1px dashed var(--border-strong);
    border-radius: 999px;
    background: var(--surface);
    color: var(--text-2);
    font-size: 12px;
  }

  .suggestion:hover {
    border-style: solid;
    border-color: var(--accent);
    color: var(--accent-text);
  }

  .seg {
    display: inline-flex;
    padding: 2px;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface-2);
  }

  .seg-option {
    display: inline-flex;
    align-items: center;
    height: 30px;
    padding: 0 12px;
    border-radius: 5px;
    color: var(--text-2);
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
  }

  .seg-option input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }

  .seg-option:has(input:focus-visible) {
    box-shadow: 0 0 0 2px var(--focus-ring);
  }

  .seg-option.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
  }

  .seg-option.active.expense {
    color: var(--expense);
  }

  .value-rules {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: var(--s-3);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .preview {
    display: flex;
    gap: var(--s-2);
    padding: var(--s-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 12.5px;
    color: var(--text-2);
  }

  .preview :global(svg) {
    flex: none;
    margin-top: 1px;
    color: var(--accent-text);
  }

  .preview-count strong {
    font-weight: 600;
    color: var(--text);
  }

  .matches {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    font-size: 12px;
    color: var(--text-2);
  }

  .matches li {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .more {
    margin-top: 4px;
    font-size: 12px;
    color: var(--text-3);
  }

  .errors {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
    color: var(--danger);
  }

  .errors li,
  p.errors {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .spacer {
    flex: 1;
  }

  @media (max-width: 760px) {
    .row2 {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

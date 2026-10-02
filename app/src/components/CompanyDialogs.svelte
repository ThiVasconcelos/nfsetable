<script lang="ts">
  // "Nova empresa", "Renomear" (name and optional CNPJ) and "Excluir" (says what is deleted).
  import { tick } from 'svelte'
  import { COMPANY_NAME_MAX, checkCnpj, companies, maskCnpj } from '../lib/companies.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  let name = $state('')
  let cnpj = $state('')
  let tried = $state(false)
  let saving = $state(false)

  const dialog = $derived(companies.dialog)
  const target = $derived(
    dialog && dialog.mode !== 'create' ? (companies.list.find((c) => c.id === dialog.id) ?? null) : null,
  )
  const editing = $derived(dialog?.mode === 'create' || dialog?.mode === 'rename')
  const cnpjState = $derived(cnpj.trim() ? checkCnpj(cnpj) : 'ok')
  const nameError = $derived(!name.trim() ? 'Dê um nome para a empresa.' : null)
  const cnpjError = $derived(cnpjState === 'incomplete' ? 'Complete o CNPJ ou deixe em branco.' : null)

  // Opening: the fields start from the company (or empty for a new one).
  let opened: string | null = null
  $effect(() => {
    const key = dialog ? `${dialog.mode}:${'id' in dialog ? dialog.id : ''}` : null
    if (key === opened) return
    opened = key
    if (!dialog) return
    name = dialog.mode === 'create' ? '' : (target?.name ?? '')
    cnpj = dialog.mode === 'create' ? '' : (target?.cnpj ?? '')
    tried = false
    saving = false
    if (dialog.mode !== 'delete') void tick().then(() => document.getElementById('company-name')?.focus())
  })

  function close() {
    if (!saving) companies.dialog = null
  }

  async function save() {
    tried = true
    if (nameError || cnpjError || saving || !dialog) return
    saving = true
    const ok =
      dialog.mode === 'create'
        ? await companies.create(name, cnpj || null)
        : dialog.mode === 'rename'
          ? await companies.rename(dialog.id, name, cnpj || null)
          : false
    saving = false
    if (ok) companies.dialog = null
  }

  async function remove() {
    if (!dialog || dialog.mode !== 'delete' || saving) return
    saving = true
    const ok = await companies.remove(dialog.id)
    saving = false
    if (ok) companies.dialog = null
  }
</script>

<Modal
  open={!!dialog && editing}
  size="sm"
  title={dialog?.mode === 'create' ? 'Nova empresa' : 'Empresa'}
  subtitle={dialog?.mode === 'create'
    ? 'Cada empresa tem as próprias fontes, edições das notas e planejamento dos impostos.'
    : 'O nome aparece no cabeçalho e nas exportações.'}
  onclose={close}
>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault()
      void save()
    }}
  >
    <div class="field">
      <label class="label" for="company-name">Nome</label>
      <input
        id="company-name"
        class="input"
        type="text"
        maxlength={COMPANY_NAME_MAX}
        placeholder="Por exemplo, Minha SaaS Ltda"
        aria-invalid={tried && !!nameError}
        bind:value={name}
      />
      {#if tried && nameError}<span class="error">{nameError}</span>{/if}
    </div>
    <div class="field">
      <label class="label" for="company-cnpj">CNPJ <span class="optional">(opcional)</span></label>
      <input
        id="company-cnpj"
        class="input num"
        type="text"
        inputmode="text"
        autocomplete="off"
        maxlength="18"
        placeholder="00.000.000/0000-00"
        aria-invalid={(tried && !!cnpjError) || cnpjState === 'invalid'}
        value={cnpj}
        oninput={(e) => {
          const masked = maskCnpj(e.currentTarget.value)
          cnpj = masked
          e.currentTarget.value = masked
        }}
      />
      {#if tried && cnpjError}
        <span class="error">{cnpjError}</span>
      {:else if cnpjState === 'invalid'}
        <span class="warn"><Icon name="alert" size={12} /> Os dígitos verificadores não conferem. Confira o número.</span>
      {:else}
        <span class="hint">Guardado só neste computador, na pasta dos dados.</span>
      {/if}
    </div>
  </form>

  {#snippet footer()}
    <button type="button" class="btn" disabled={saving} onclick={close}>Cancelar</button>
    <span class="spacer"></span>
    <button type="button" class="btn btn-primary" disabled={saving} onclick={() => save()}>
      {#if saving}<span class="spinner"></span>{/if}
      {dialog?.mode === 'create' ? 'Criar empresa' : 'Salvar'}
    </button>
  {/snippet}
</Modal>

<Modal
  open={dialog?.mode === 'delete' && !!target}
  size="sm"
  title="Excluir “{target?.name ?? ''}”?"
  subtitle="Não dá para desfazer."
  onclose={close}
>
  <div class="delete">
    <p>Serão apagados desta empresa:</p>
    <ul>
      <li>as fontes (pastas e arquivos escolhidos) e os termos ignorados;</li>
      <li>as edições das notas: valores, tipos, competências, natureza e notas removidas;</li>
      <li>todo o planejamento dos impostos: regime, CNAE, pró-labore, previstos, custos e reserva.</li>
    </ul>
    <p class="keep">
      <Icon name="shield" size={14} />
      Os arquivos PDF, as regras (compartilhadas) e as outras empresas não são tocados.
    </p>
  </div>

  {#snippet footer()}
    <button type="button" class="btn" disabled={saving} onclick={close}>Cancelar</button>
    <span class="spacer"></span>
    <button type="button" class="btn danger-solid" disabled={saving} onclick={() => remove()}>
      {#if saving}<span class="spinner"></span>{/if}
      Excluir empresa
    </button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-2);
  }

  .optional {
    font-weight: 400;
    color: var(--text-3);
  }

  .hint {
    font-size: 12px;
    color: var(--text-3);
  }

  .error {
    font-size: 12px;
    color: var(--danger);
  }

  .warn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--warn);
  }

  .delete {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .delete ul {
    margin: 0;
    padding-left: 20px;
  }

  .keep {
    display: flex;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    background: var(--surface-2);
    font-size: 12.5px;
  }

  .keep :global(svg) {
    flex: none;
    margin-top: 2px;
    color: var(--ok);
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

  .spacer {
    flex: 1;
  }
</style>

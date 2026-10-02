<script lang="ts">
  // "Configurações": the data folder (profiles, notes edits and the tax planning), e.g. a synced
  // folder for backup. Changing it asks whether to copy the current data or use what is there.
  import { errorMessage, pickDataDir } from '../lib/api'
  import { companies } from '../lib/companies.svelte'
  import { dataDir } from '../lib/datadir.svelte'
  import { pathKey } from '../lib/format'
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
  import Modal from './Modal.svelte'

  /** The folder waiting for "copiar" or "usar" (null path = back to the default folder). */
  let pending = $state<{ path: string | null } | null>(null)
  let pickError = $state<string | null>(null)

  const info = $derived(store.info)
  const isDefault = $derived(!info || pathKey(info.dataDir) === pathKey(info.defaultDataDir))
  const pendingLabel = $derived(pending ? (pending.path ?? info?.defaultDataDir ?? '') : '')

  $effect(() => {
    if (!store.settingsOpen) {
      pending = null
      pickError = null
      dataDir.error = null
    }
  })

  async function choose() {
    pickError = null
    dataDir.error = null
    try {
      const path = await pickDataDir()
      if (!path) return
      if (info && pathKey(path) === pathKey(info.dataDir)) {
        pickError = 'Essa já é a pasta em uso.'
        return
      }
      pending = { path }
    } catch (e) {
      pickError = errorMessage(e)
    }
  }

  async function apply(copy: boolean) {
    if (!pending) return
    const ok = await dataDir.switchTo(pending.path, copy)
    if (ok) pending = null
  }
</script>

<Modal
  open={store.settingsOpen}
  title="Configurações"
  subtitle="Preferências do nfsetable neste computador"
  onclose={() => (store.settingsOpen = false)}
>
  <section class="block" aria-labelledby="settings-data">
    <h3 id="settings-data"><Icon name="hardDrive" size={15} /> Pasta dos dados</h3>
    <p class="lead">
      Onde ficam os dados de todas as empresas (fontes, edições das notas e planejamento dos impostos) e os perfis e
      regras. Uma pasta sincronizada (como o Google Drive) vira backup.
    </p>

    {#if info?.dataDirError}
      <div class="notice warn" role="alert">
        <Icon name="alert" size={15} />
        <span>{info.dataDirError}</span>
      </div>
    {/if}

    {#if info}
      <dl class="paths">
        <div>
          <dt>Em uso</dt>
          <dd>
            <code class="mono" title={info.dataDir}>{info.dataDir}</code>
            {#if isDefault}<span class="tag">padrão</span>{/if}
          </dd>
        </div>
        {#if !isDefault}
          <div>
            <dt>Padrão</dt>
            <dd><code class="mono" title={info.defaultDataDir}>{info.defaultDataDir}</code></dd>
          </div>
        {/if}
      </dl>
    {/if}

    {#if pending}
      <div class="confirm" role="group" aria-label="Como usar a pasta escolhida">
        <p class="confirm-title">
          {pending.path ? 'Usar a pasta' : 'Voltar para a pasta padrão'}
          <code class="mono">{pendingLabel}</code>
        </p>
        <div class="choices">
          <button type="button" class="choice primary" disabled={dataDir.switching} onclick={() => apply(true)}>
            <span class="choice-title">
              {#if dataDir.switching}<span class="spinner"></span>{:else}<Icon name="copy" size={15} />{/if}
              Copiar meus dados para lá
            </span>
            <span class="choice-hint">Leva as empresas, os perfis, as notas e o planejamento de agora (substitui os de mesmo nome).</span>
          </button>
          <button type="button" class="choice" disabled={dataDir.switching} onclick={() => apply(false)}>
            <span class="choice-title"><Icon name="folderOpen" size={15} /> Usar os dados que já estão lá</span>
            <span class="choice-hint">Para abrir os dados de outro computador ou de um backup.</span>
          </button>
        </div>
        <button type="button" class="link cancel" disabled={dataDir.switching} onclick={() => (pending = null)}>Cancelar</button>
      </div>
    {:else}
      <div class="actions">
        <button type="button" class="btn" onclick={choose}>
          <Icon name="folderOpen" size={15} />
          Escolher outra pasta…
        </button>
        {#if !isDefault}
          <button type="button" class="btn btn-ghost" onclick={() => (pending = { path: null })}>Voltar para a pasta padrão</button>
        {/if}
      </div>
    {/if}

    {#if pickError || dataDir.error}
      <p class="error" role="alert"><Icon name="alertCircle" size={14} /> {pickError ?? dataDir.error}</p>
    {/if}
  </section>

  <section class="block startup" aria-labelledby="settings-startup">
    <h3 id="settings-startup"><Icon name="building" size={15} /> Ao abrir o app</h3>
    <div class="choices-row" role="radiogroup" aria-labelledby="settings-startup">
      <label class="option" class:checked={companies.startup === 'ask'}>
        <input type="radio" name="startup" checked={companies.startup === 'ask'} onchange={() => companies.setStartup('ask')} />
        <span>
          <span class="option-title">Perguntar a empresa</span>
          <span class="option-hint">Com duas ou mais empresas, escolha qual abrir.</span>
        </span>
      </label>
      <label class="option" class:checked={companies.startup === 'last'}>
        <input type="radio" name="startup" checked={companies.startup === 'last'} onchange={() => companies.setStartup('last')} />
        <span>
          <span class="option-title">Abrir a última usada</span>
          <span class="option-hint">Vai direto para a última empresa aberta.</span>
        </span>
      </label>
    </div>
  </section>

  {#snippet footer()}
    <span class="spacer"></span>
    <button type="button" class="btn" onclick={() => (store.settingsOpen = false)}>Fechar</button>
  {/snippet}
</Modal>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
  }

  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    font-weight: 600;
  }

  h3 :global(svg) {
    color: var(--accent-text);
  }

  .lead {
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-2);
  }

  .notice {
    display: flex;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius);
    font-size: 12.5px;
    line-height: 1.45;
  }

  .notice.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }

  .notice :global(svg) {
    flex: none;
    margin-top: 1px;
  }

  .paths {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
  }

  .paths > div {
    display: flex;
    align-items: baseline;
    gap: var(--s-3);
  }

  dt {
    flex: none;
    width: 56px;
    font-size: 12px;
    color: var(--text-3);
  }

  dd {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
  }

  code {
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .tag {
    flex: none;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--surface-3);
    color: var(--text-2);
    font-size: 11.5px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s-2);
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
    padding: var(--s-3);
    border: 1px solid var(--accent-soft-2);
    border-radius: var(--radius);
    background: var(--accent-soft);
  }

  .confirm-title {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 500;
  }

  .confirm-title code {
    background: var(--surface);
  }

  .choices {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s-2);
  }

  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 10px 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    text-align: left;
    transition: border-color 0.12s var(--ease);
  }

  .choice:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .choice.primary {
    border-color: var(--accent);
  }

  .choice-title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    font-weight: 600;
  }

  .choice-title :global(svg) {
    color: var(--accent-text);
  }

  .choice-hint {
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-3);
  }

  .cancel {
    align-self: flex-start;
    font-size: 12.5px;
  }

  .error {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--danger);
  }

  .spacer {
    flex: 1;
  }

  .startup {
    margin-top: var(--s-5);
    padding-top: var(--s-4);
    border-top: 1px solid var(--border);
  }

  .choices-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--s-2);
  }

  .option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }

  .option.checked {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .option input {
    margin: 2px 0 0;
    accent-color: var(--accent);
  }

  .option-title {
    display: block;
    font-size: 13px;
    font-weight: 600;
  }

  .option-hint {
    display: block;
    margin-top: 2px;
    font-size: 12px;
    line-height: 1.4;
    color: var(--text-3);
  }

  @media (max-width: 640px) {
    .choices {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>

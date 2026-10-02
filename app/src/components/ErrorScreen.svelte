<script lang="ts">
  import { copyText } from '../lib/clipboard'
  import Icon from './Icon.svelte'

  interface Props {
    /** "pdfium": the PDF library is missing; "boot": the backend did not answer. */
    kind: 'pdfium' | 'boot'
    message: string
  }

  let { kind, message }: Props = $props()

  let copied = $state<string | null>(null)

  const COMMANDS = [
    { os: 'Windows (PowerShell)', cmd: 'powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1' },
    { os: 'Linux / macOS', cmd: 'bash scripts/fetch-pdfium.sh' },
  ]

  async function copy(text: string) {
    if (await copyText(text)) {
      copied = text
      setTimeout(() => {
        if (copied === text) copied = null
      }, 1800)
    }
  }
</script>

<div class="screen">
  <div class="panel" role="alert" aria-labelledby="error-title">
    <div class="badge" aria-hidden="true"><Icon name="alert" size={22} /></div>

    {#if kind === 'pdfium'}
      <h1 id="error-title">Falta a biblioteca PDFium</h1>
      <p class="lead">
        O nfsetable usa o PDFium (o leitor de PDF do Chrome) para ler as notas, e ele não foi encontrado. Sem ele não dá
        para abrir os PDFs.
      </p>

      <section>
        <h2>Rodando a partir do código-fonte?</h2>
        <p>Baixe a biblioteca com o script do projeto (na raiz do repositório) e abra o app de novo:</p>
        {#each COMMANDS as c (c.os)}
          <div class="cmd">
            <span class="os">{c.os}</span>
            <code class="mono">{c.cmd}</code>
            <button type="button" class="btn btn-ghost btn-sm" onclick={() => copy(c.cmd)} aria-label="Copiar comando para {c.os}">
              <Icon name={copied === c.cmd ? 'check' : 'copy'} size={14} />
              {copied === c.cmd ? 'Copiado' : 'Copiar'}
            </button>
          </div>
        {/each}
        <p class="small">Os scripts ficam em <code class="mono">scripts/fetch-pdfium.ps1</code> e <code class="mono">scripts/fetch-pdfium.sh</code> e salvam a biblioteca em <code class="mono">vendor/pdfium/</code>.</p>
      </section>

      <section>
        <h2>Instalou pelo instalador?</h2>
        <p>Reinstale a versão mais recente: o PDFium vem junto com o app.</p>
      </section>
    {:else}
      <h1 id="error-title">Não foi possível iniciar o nfsetable</h1>
      <p class="lead">A parte do app que lê os arquivos não respondeu. Feche e abra o app de novo.</p>
    {/if}

    {#if message}
      <details>
        <summary>Detalhes técnicos</summary>
        <pre class="mono">{message}</pre>
        <button type="button" class="btn btn-sm" onclick={() => copy(message)}>
          <Icon name={copied === message ? 'check' : 'copy'} size={14} />
          {copied === message ? 'Copiado' : 'Copiar detalhes'}
        </button>
      </details>
    {/if}
  </div>
</div>

<style>
  .screen {
    height: 100%;
    display: grid;
    place-items: center;
    padding: var(--s-5);
    overflow: auto;
    background: var(--bg);
  }

  .panel {
    width: min(620px, 100%);
    padding: var(--s-6);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .badge {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    margin-bottom: var(--s-4);
    border-radius: 12px;
    background: var(--warn-soft);
    color: var(--warn);
  }

  h1 {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .lead {
    margin-top: var(--s-2);
    color: var(--text-2);
  }

  section {
    margin-top: var(--s-5);
  }

  h2 {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 4px;
  }

  section p {
    color: var(--text-2);
    font-size: 13.5px;
  }

  .cmd {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    grid-template-areas:
      'os copy'
      'code code';
    align-items: center;
    row-gap: 2px;
    margin-top: var(--s-2);
    padding: 6px 6px 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-2);
  }

  .os {
    grid-area: os;
    font-size: 12px;
    color: var(--text-3);
  }

  .cmd .btn {
    grid-area: copy;
  }

  .cmd code {
    grid-area: code;
    overflow-wrap: anywhere;
  }

  .small {
    margin-top: var(--s-2);
    font-size: 12.5px !important;
    color: var(--text-3) !important;
  }

  code {
    color: var(--text);
  }

  details {
    margin-top: var(--s-5);
    font-size: 13px;
  }

  summary {
    cursor: pointer;
    color: var(--text-2);
  }

  pre {
    margin: var(--s-2) 0;
    padding: var(--s-3);
    border-radius: var(--radius);
    background: var(--surface-2);
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--text-2);
  }
</style>

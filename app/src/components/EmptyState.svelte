<script lang="ts">
  import { store } from '../lib/store.svelte'
  import Icon from './Icon.svelte'
</script>

<div class="empty" class:dragging={store.dragging}>
  <div class="art" aria-hidden="true">
    <svg viewBox="0 0 120 96" width="120" height="96" fill="none">
      <rect x="30" y="10" width="54" height="68" rx="6" fill="var(--surface)" stroke="var(--border-strong)" />
      <path d="M40 26h34M40 34h34M40 42h22" stroke="var(--border-strong)" stroke-width="3" stroke-linecap="round" />
      <rect x="40" y="54" width="34" height="12" rx="3" fill="var(--accent-soft-2)" />
      <path d="M8 44a6 6 0 0 1 6-6h22l6 7h56a6 6 0 0 1 6 6v33a6 6 0 0 1-6 6H14a6 6 0 0 1-6-6z" fill="var(--accent)" opacity="0.92" />
      <path d="M58 58v16M51 67l7 7 7-7" stroke="#fff" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" />
    </svg>
  </div>

  <h1>Nenhuma nota por aqui ainda</h1>
  <p class="lead">Arraste uma pasta com suas notas para cá ou clique em Adicionar pasta.</p>

  <div class="actions">
    <button type="button" class="btn btn-primary" onclick={() => store.pickFolders()}>
      <Icon name="folderPlus" size={16} />
      Adicionar pasta
    </button>
    <button type="button" class="btn" onclick={() => store.pickFiles()}>
      <Icon name="filePlus" size={16} />
      Adicionar arquivos
    </button>
  </div>

  <ul class="points">
    <li>
      <Icon name="lock" size={16} />
      <div>
        <strong>Tudo no seu computador</strong>
        <span>Os PDFs são lidos aqui mesmo, sem internet.</span>
      </div>
    </li>
    <li>
      <Icon name="fileText" size={16} />
      <div>
        <strong>DANFSe e outros layouts</strong>
        <span>O valor líquido é encontrado sozinho.</span>
      </div>
    </li>
    <li>
      <Icon name="marquee" size={16} />
      <div>
        <strong>Corrija em segundos</strong>
        <span>Marque o valor no PDF e teste nas outras notas.</span>
      </div>
    </li>
  </ul>
</div>

<style>
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s-3);
    min-height: 0;
    padding: var(--s-6);
    border: 1.5px dashed var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface);
    text-align: center;
    transition:
      border-color 0.15s var(--ease),
      background-color 0.15s var(--ease);
    overflow: auto;
  }

  .empty.dragging {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .art {
    margin-bottom: var(--s-2);
  }

  h1 {
    font-size: 20px;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .lead {
    max-width: 480px;
    color: var(--text-2);
    font-size: 14px;
  }

  .actions {
    display: flex;
    gap: var(--s-2);
    margin-top: var(--s-2);
  }

  .actions .btn {
    height: 36px;
    padding: 0 16px;
  }

  .points {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 200px));
    gap: var(--s-5);
    margin: var(--s-6) 0 0;
    padding: var(--s-5) 0 0;
    border-top: 1px solid var(--border);
    list-style: none;
    text-align: left;
  }

  .points li {
    display: flex;
    gap: 10px;
    font-size: 12.5px;
  }

  .points :global(svg) {
    margin-top: 2px;
    color: var(--accent-text);
  }

  .points div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .points strong {
    font-weight: 600;
    color: var(--text);
  }

  .points span {
    color: var(--text-3);
  }

  @media (max-width: 1100px), (max-height: 680px) {
    .points {
      display: none;
    }
  }
</style>

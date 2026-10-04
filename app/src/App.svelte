<script lang="ts">
  import { onMount } from 'svelte'
  import * as api from './lib/api'
  import AppFooter from './components/AppFooter.svelte'
  import AppHeader from './components/AppHeader.svelte'
  import CompanyDialogs from './components/CompanyDialogs.svelte'
  import CompanyPicker from './components/CompanyPicker.svelte'
  import DocTable from './components/DocTable.svelte'
  import EmptyState from './components/EmptyState.svelte'
  import ErrorScreen from './components/ErrorScreen.svelte'
  import ExcludedDialog from './components/ExcludedDialog.svelte'
  import FilterBar from './components/FilterBar.svelte'
  import Icon from './components/Icon.svelte'
  import IconSprite from './components/IconSprite.svelte'
  import PreviewPanel from './components/PreviewPanel.svelte'
  import ProfilesDialog from './components/ProfilesDialog.svelte'
  import ProgressBar from './components/ProgressBar.svelte'
  import RuleTestDialog from './components/RuleTestDialog.svelte'
  import SettingsDialog from './components/SettingsDialog.svelte'
  import Sidebar from './components/Sidebar.svelte'
  import SummaryCards from './components/SummaryCards.svelte'
  import TaxView from './components/TaxView.svelte'
  import Toasts from './components/Toasts.svelte'
  import { companies, startApp } from './lib/companies.svelte'
  import { dataDir } from './lib/datadir.svelte'
  import { store } from './lib/store.svelte'
  import { tax } from './lib/tax.svelte'

  const CLOSE_WAIT_MS = 3000

  /** Preview zoom, kept while browsing rows. */
  let zoom = $state(1)
  let windowWidth = $state(1280)

  onMount(() => {
    // The companies, then the notes and the planning of the active one, are read at startup.
    void startApp()
    // A change typed right before the window is hidden or closed is saved without the delay.
    const flushAll = () => {
      void tax.flush()
      void store.notesDoc.flush()
      void companies.doc.flush()
    }
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') flushAll()
    }
    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('pagehide', flushAll)
    // Closing the window waits for the pending saves (the delayed ones would be lost otherwise),
    // never more than a few seconds.
    const saveBeforeClose = () =>
      Promise.race([
        Promise.all([
          tax.flush(),
          store.notesDoc.flush(),
          companies.doc.flush(),
          api.saveCachedResults().catch(() => {}),
        ]).then(() => {}),
        new Promise<void>((resolve) => setTimeout(resolve, CLOSE_WAIT_MS)),
      ])
    const closing = api.onCloseRequested(saveBeforeClose)
    return () => {
      document.removeEventListener('visibilitychange', onVisibility)
      window.removeEventListener('pagehide', flushAll)
      void closing.then((unlisten) => unlisten())
    }
  })

  const pdfiumMissing = $derived(store.boot === 'ready' && !!store.info && !store.info.pdfiumOk)

  // The sources panel folds into a rail while the preview is open on smaller windows,
  // unless the user chose otherwise with the toggle.
  const railMode = $derived(store.sidebarOverride ?? (!!store.activeRow && windowWidth < 1500))
</script>

<svelte:window bind:innerWidth={windowWidth} />
<IconSprite />

{#if store.boot === 'loading'}
  <div class="boot" aria-busy="true" aria-label="Carregando"><span class="spinner"></span></div>
{:else if store.boot === 'failed'}
  <ErrorScreen kind="boot" message={store.bootError ?? ''} />
{:else if pdfiumMissing}
  <ErrorScreen kind="pdfium" message={store.info?.pdfiumError ?? ''} />
{:else}
  {#if companies.picking}
    <!-- 2+ companies: which one to open (nothing of them is read or scanned before). -->
    <CompanyPicker />
  {:else}
    <div class="app" data-phase={store.phase} data-busy={store.busy ? 'true' : 'false'} data-view={store.view}>
      <AppHeader />

      <div class="banner-slot">
        {#if store.info?.dataDirError && !dataDir.bannerDismissed}
          <div class="banner" role="alert">
            <Icon name="alert" size={15} />
            <span>{store.info.dataDirError}</span>
            <button type="button" class="btn btn-sm" onclick={() => (store.settingsOpen = true)}>Configurações</button>
            <button
              type="button"
              class="btn btn-ghost btn-icon btn-sm"
              aria-label="Fechar aviso"
              onclick={() => (dataDir.bannerDismissed = true)}
            >
              <Icon name="x" size={14} />
            </button>
          </div>
        {/if}
      </div>

      {#if store.view === 'taxes'}
        <TaxView />
      {:else}
        <div class="body" class:rail={railMode}>
          <ProgressBar />
          <Sidebar collapsed={railMode} ontoggle={() => (store.sidebarOverride = !railMode)} />
          <main class="main" aria-label="Notas">
            {#if !store.sources.length}
              <EmptyState />
            {:else}
              <SummaryCards />
              <FilterBar />
              <DocTable />
            {/if}
          </main>
          {#if store.activeRow}
            {@const row = store.activeRow}
            {#key row.path}
              <PreviewPanel {row} bind:zoom />
            {/key}
          {/if}
        </div>
      {/if}

      <AppFooter />

      {#if store.dragging}
        <div class="drop-overlay" aria-hidden="true">
          <div class="drop-card">
            <Icon name="drop" size={28} />
            <span>Solte para adicionar as pastas ou PDFs</span>
          </div>
        </div>
      {/if}
    </div>

    <RuleTestDialog />
    <ProfilesDialog />
    <ExcludedDialog />
  {/if}
  <!-- Shared by the picker and the app, so they stay open when one replaces the other (a new data
       folder can lead to the picker). -->
  <SettingsDialog />
  <CompanyDialogs />
  <Toasts />
{/if}

<style>
  .boot {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-3);
  }

  .app {
    position: relative;
    height: 100%;
    display: grid;
    grid-template-rows: var(--header-h) auto minmax(0, 1fr) auto;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px var(--s-3) 6px var(--s-5);
    border-bottom: 1px solid var(--border);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: 13px;
  }

  .banner > span {
    flex: 1;
    min-width: 0;
  }

  .banner :global(svg) {
    flex: none;
  }

  .body {
    --sidebar-current: var(--sidebar-w);
    position: relative;
    display: grid;
    grid-template-columns: var(--sidebar-current) minmax(0, 1fr) auto;
    min-height: 0;
  }

  .body.rail {
    --sidebar-current: 56px;
  }

  .main {
    container: main / inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    min-width: 0;
    min-height: 0;
    padding: 20px var(--s-5) var(--s-4);
  }

  @media (max-height: 700px) {
    .main {
      gap: var(--s-3);
      padding-top: var(--s-4);
    }
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 80;
    display: grid;
    place-items: center;
    background: rgba(62, 99, 221, 0.08);
    border: 2px dashed var(--accent);
    pointer-events: none;
  }

  .drop-card {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    padding: var(--s-4) var(--s-5);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-pop);
    color: var(--accent-text);
    font-size: 15px;
    font-weight: 500;
  }

  /* Narrow windows: the preview slides over the table instead of squeezing it. */
  @media (max-width: 1199px) {
    .body {
      grid-template-columns: var(--sidebar-current) minmax(0, 1fr);
    }

    .body :global(.preview) {
      position: absolute;
      top: 0;
      right: 0;
      bottom: 0;
      z-index: 25;
      width: min(var(--preview-w), calc(100% - 120px));
      box-shadow: var(--shadow-pop);
    }

    .main {
      padding: var(--s-4);
    }
  }
</style>

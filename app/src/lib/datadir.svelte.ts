// The data folder (profiles and store documents): switching it and reloading everything that lives
// there. The backend copies the current data first ("Copiar meus dados para lá") or just starts
// using the folder ("Usar os dados que já estão lá").

import * as api from './api'
import { errorMessage } from './api'
import { companies } from './companies.svelte'
import { store } from './store.svelte'
import { tax } from './tax.svelte'

class DataDirState {
  switching = $state(false)
  error = $state<string | null>(null)
  /** The "pasta indisponível" banner was closed (for this session). */
  bannerDismissed = $state(false)

  /**
   * Moves the data to `path` (null = the default folder). Pending saves land in the current folder
   * first, so a copy takes them along; then the companies and the profiles are read from the new
   * folder. From "Escolha a empresa", or when the folder brings its own companies ("usar os dados
   * que já estão lá") and there are 2+ of them to ask about, the picker shows the new list and
   * nothing of a company is read before the choice; otherwise the notes and the planning of the
   * active company are read and its files are read again.
   */
  async switchTo(path: string | null, copy: boolean): Promise<boolean> {
    if (this.switching) return false
    this.switching = true
    this.error = null
    try {
      await Promise.all([store.notesDoc.flush(), tax.flush(), companies.doc.flush()])
      const info = await api.setDataDir(path, copy)
      store.info = info
      this.bannerDismissed = false
      store.resetView()
      await companies.load()
      await store.loadProfiles()
      if (companies.picking || (!copy && companies.shouldAsk)) {
        store.forgetCache()
        companies.pickAgain()
      } else {
        await companies.loadActive()
        store.reextract()
      }
      store.toast(
        copy ? 'Dados copiados. A pasta nova já está em uso.' : 'Pronto: usando os dados da pasta escolhida.',
        'success',
      )
      return true
    } catch (e) {
      this.error = errorMessage(e)
      return false
    } finally {
      this.switching = false
    }
  }
}

export const dataDir = new DataDirState()

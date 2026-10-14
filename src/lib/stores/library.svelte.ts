// @req RF-05 RF-06
import { api, type Playlist } from '../ipc';
import { notifyError } from './toast.svelte';

/**
 * Estado compartilhado da biblioteca. As telas buscam as próprias listas (com consulta própria) e
 * dependem de `version` em um $effect para recarregar quando chega `library://changed`.
 */
class LibraryStore {
  version = $state(0);
  playlists = $state<Playlist[]>([]);
  private started = false;
  private t: ReturnType<typeof setTimeout> | null = null;

  async start() {
    if (this.started) return;
    this.started = true;
    await api.on('library://changed', () => {
      if (this.t) clearTimeout(this.t);
      this.t = setTimeout(() => { void this.reloadPlaylists(); this.version++; }, 120);
    });
    await this.reloadPlaylists();
  }

  async reloadPlaylists() {
    try { this.playlists = await api.playlistList(); } catch (e) { notifyError(e); }
  }
}
export const library = new LibraryStore();

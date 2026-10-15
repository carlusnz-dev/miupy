// @req RF-05 RF-06 RNF-11 RNF-13
import { api, type Playlist } from '../ipc';
import { mergeList } from '../util/diff';
import { tagsFor } from '../util/libraryEvents';
import { invalidate, Resource } from './resource.svelte';

/**
 * Estado compartilhado da biblioteca. `library://changed` chega aqui uma vez só: o `reason` decide
 * quais telas atualizam (`invalidate`) e as playlists são um `Resource` como qualquer outro, então
 * a lista fica em cache e só troca o que mudou.
 */
class LibraryStore {
  private readonly lists = new Resource<Playlist[]>(['playlists'], mergeList((p: Playlist) => p.id));
  private started = false;

  get playlists(): Playlist[] { return this.lists.data ?? []; }
  /** Já houve uma resposta de `playlist_list` (a tela mostra o esqueleto até lá). */
  get playlistsReady() { return this.lists.data !== undefined; }
  get playlistsError() { return this.lists.data === undefined ? this.lists.error : null; }

  async start() {
    if (this.started) return;
    this.started = true;
    this.lists.attach();
    await api.on('library://changed', (e) => invalidate(tagsFor(e?.reason)));
    await this.reloadPlaylists();
  }

  reloadPlaylists() {
    return this.lists.load('playlists', () => api.playlistList());
  }
}
export const library = new LibraryStore();

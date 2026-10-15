// Ações compartilhadas entre telas (com retorno visual por toast).
// @req RF-04 RF-10 RF-11
import { api, type Track } from './ipc';
import { favorites } from './stores/favorites.svelte';
import { notify, notifyError } from './stores/toast.svelte';

/** Atualiza o coração na hora e volta ao estado anterior, com aviso, se o comando falhar. */
export async function toggleFavorite(t: Track) {
  const favorite = !favorites.is(t);
  const change = favorites.apply(t.id, favorite);
  try {
    await api.librarySetFavorite({ trackId: t.id, favorite });
    change.confirm();
  } catch (e) {
    change.revert();
    notifyError(e);
  }
}

export async function downloadTracks(ids: string[]) {
  if (!ids.length) return;
  try {
    const jobs = await api.downloadEnqueue({ trackIds: ids });
    notify(jobs.length ? `${jobs.length} ${jobs.length === 1 ? 'faixa na fila' : 'faixas na fila'} de download.` : 'Essas faixas já estão baixadas.');
  } catch (e) { notifyError(e); }
}

export async function downloadPlaylist(playlistId: string) {
  try {
    const jobs = await api.downloadEnqueuePlaylist({ playlistId });
    notify(jobs.length ? `${jobs.length} faixas na fila de download.` : 'Todas as faixas já estão baixadas.');
  } catch (e) { notifyError(e); }
}

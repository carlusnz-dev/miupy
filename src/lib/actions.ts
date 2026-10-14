// Ações compartilhadas entre telas (com retorno visual por toast).
// @req RF-04 RF-10 RF-11
import { api, type Track } from './ipc';
import { notify, notifyError } from './stores/toast.svelte';

export async function toggleFavorite(t: Track) {
  try { await api.librarySetFavorite({ trackId: t.id, favorite: !t.isFavorite }); }
  catch (e) { notifyError(e); }
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

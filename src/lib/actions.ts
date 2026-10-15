// Ações compartilhadas entre telas (com retorno visual por toast).
// @req RF-04 RF-06 RF-10 RF-11
import { api, type Track } from './ipc';
import { favorites } from './stores/favorites.svelte';
import { notify, notifyError } from './stores/toast.svelte';
import { ui } from './stores/ui.svelte';

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

/** Pede confirmação e tira as faixas da discoteca (e das playlists); devolve se removeu. @req RF-06 */
export async function removeTracks(tracks: Track[]): Promise<boolean> {
  if (!tracks.length) return false;
  const one = tracks.length === 1;
  const text = `${one ? `"${tracks[0].title}" sai` : `${tracks.length} faixas saem`} da discoteca e das playlists.`;
  const hasFile = tracks.some((t) => t.filePath);
  let deleteFile = false;
  if (hasFile) {
    const answer = await ui.confirmOption('Remover da discoteca', text, 'Remover', 'Apagar também o arquivo baixado');
    if (answer === null) return false;
    deleteFile = answer;
  } else if (!(await ui.confirm('Remover da discoteca', text, 'Remover', true))) return false;
  try {
    for (const t of tracks) await api.libraryDeleteTrack({ trackId: t.id, deleteFile });
    notify(one ? 'Faixa removida da discoteca.' : `${tracks.length} faixas removidas da discoteca.`);
    return true;
  } catch (e) { notifyError(e); return false; }
}

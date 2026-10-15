// @req RNF-11 RNF-13
// Quais telas precisam de dados novos para cada `reason` de `library://changed` (ipc.md). Uma foto de
// artista chegando não muda a lista de faixas; um fim de download não muda as playlists.

/** Grupos de dados em cache: listas de faixas, estatísticas, playlists e dados de artista. */
export type CacheTag = 'tracks' | 'stats' | 'playlists' | 'artist';

const ALL: readonly CacheTag[] = ['tracks', 'stats', 'playlists', 'artist'];

const BY_REASON: Record<string, readonly CacheTag[]> = {
  favorite: ['tracks', 'stats', 'playlists'],
  favorites: ['tracks', 'stats', 'playlists'],
  download_queued: ['tracks'],
  download_failed: ['tracks'],
  download_cancelled: ['tracks'],
  download_done: ['tracks', 'stats'],
  playlist_created: ['playlists', 'stats'],
  playlist_deleted: ['playlists', 'stats'],
  playlist_updated: ['playlists'],
  playlist_cover_updated: ['playlists'],
  playlist_tracks_added: ['playlists', 'tracks', 'stats'],
  playlist_tracks_removed: ['playlists', 'tracks', 'stats'],
  playlist_reordered: ['playlists', 'tracks'],
  artist_photo_updated: ['artist', 'playlists'],
  artist_updated: ['artist', 'playlists', 'tracks'],
};

/** Grupos afetados por um motivo. Motivo desconhecido ou ausente afeta tudo (o seguro). */
export function tagsFor(reason: string | undefined): readonly CacheTag[] {
  return (reason && BY_REASON[reason]) || ALL;
}

const CURRENT_TRACK = new Set(['favorite', 'favorites', 'download_done', 'artist_updated']);

/** Motivos que mudam a faixa que está tocando (favorito, tags, capa, arquivo). */
export function affectsCurrentTrack(reason: string | undefined): boolean {
  return !reason || !(reason in BY_REASON) || CURRENT_TRACK.has(reason);
}

/** Motivos que mudam a pontuação das faixas (favoritar, apagar). Os outros não mexem em plays, pulos nem favoritos. */
export function affectsScores(reason: string | undefined): boolean {
  return reason === 'favorite' || reason === 'favorites' || reason === 'track_deleted';
}

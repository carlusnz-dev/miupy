import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { MiupyApi } from './api';
import type { IpcError } from './types';

export function toIpcError(e: unknown): IpcError {
  if (e && typeof e === 'object' && 'code' in e && 'message' in e) return e as IpcError;
  return { code: 'internal', message: typeof e === 'string' ? e : e instanceof Error ? e.message : 'Erro desconhecido' };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toIpcError(e);
  }
}

/** Cliente real: um método por comando de docs/arquitetura/ipc.md. */
export function createTauriApi(): MiupyApi {
  return {
    kind: 'tauri',
    playerGetState: () => call('player_get_state'),
    playerPlay: (a) => call('player_play', a),
    playerPause: () => call('player_pause'),
    playerResume: () => call('player_resume'),
    playerStop: () => call('player_stop'),
    playerSeek: (a) => call('player_seek', a),
    playerNext: () => call('player_next'),
    playerPrevious: () => call('player_previous'),
    playerSetVolume: (a) => call('player_set_volume', a),
    playerSetMuted: (a) => call('player_set_muted', a),

    libraryListTracks: (a) => call('library_list_tracks', a ?? {}),
    libraryGetTrack: (a) => call('library_get_track', a),
    librarySetFavorite: (a) => call('library_set_favorite', a),
    libraryDeleteTrack: (a) => call('library_delete_track', a),

    playlistList: () => call('playlist_list'),
    playlistCreate: (a) => call('playlist_create', a),
    playlistUpdate: (a) => call('playlist_update', a),
    playlistDelete: (a) => call('playlist_delete', a),
    playlistGetTracks: (a) => call('playlist_get_tracks', a),
    playlistAddTracks: (a) => call('playlist_add_tracks', a),
    playlistRemoveTracks: (a) => call('playlist_remove_tracks', a),
    playlistMoveTrack: (a) => call('playlist_move_track', a),

    youtubeSearch: (a) => call('youtube_search', a),
    youtubeAddToLibrary: (a) => call('youtube_add_to_library', a),

    downloadEnqueue: (a) => call('download_enqueue', a),
    downloadEnqueuePlaylist: (a) => call('download_enqueue_playlist', a),
    downloadList: () => call('download_list'),
    downloadCancel: (a) => call('download_cancel', a),

    metadataRefreshTrack: (a) => call('metadata_refresh_track', a),
    metadataGetArtist: (a) => call('metadata_get_artist', a),
    metadataGetLyrics: (a) => call('metadata_get_lyrics', a),

    statsLibrary: () => call('stats_library'),
    statsWrapped: (a) => call('stats_wrapped', a),

    profileGet: () => call('profile_get'),
    profileUpdate: (a) => call('profile_update', a),
    settingsGet: () => call('settings_get'),
    settingsUpdate: (a) => call('settings_update', a),

    async on(event, handler) {
      const un = await listen<never>(event, (e) => handler(e.payload as never));
      return un;
    },
    resolveAsset(path) {
      if (!path) return null;
      if (/^(https?:|data:|blob:|asset:)/.test(path)) return path;
      return convertFileSrc(path);
    },
  };
}

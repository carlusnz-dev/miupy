// @req RF-03 RF-05 RF-10 RF-15 RF-19
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
    playerGetQueue: () => call('player_get_queue'),
    playerSetShuffle: (a) => call('player_set_shuffle', a), // @req RF-24

    libraryListTracks: (a) => call('library_list_tracks', a ?? {}),
    libraryGetTrack: (a) => call('library_get_track', a),
    libraryListTracksByArtist: (a) => call('library_list_tracks_by_artist', a),
    librarySetFavorite: (a) => call('library_set_favorite', a),
    libraryDeleteTrack: (a) => call('library_delete_track', a),

    playlistList: () => call('playlist_list'),
    favoritesSetPlaylist: (a) => call('favorites_set_playlist', a),
    playlistCreate: (a) => call('playlist_create', a),
    playlistUpdate: (a) => call('playlist_update', a),
    playlistDelete: (a) => call('playlist_delete', a),
    playlistGetTracks: (a) => call('playlist_get_tracks', a),
    playlistAddTracks: (a) => call('playlist_add_tracks', a),
    playlistRemoveTracks: (a) => call('playlist_remove_tracks', a),
    playlistMoveTrack: (a) => call('playlist_move_track', a),
    playlistSetCover: (a) => call('playlist_set_cover', a), // @req RF-22

    youtubeSearch: (a) => call('youtube_search', a),
    youtubeSuggest: (a) => call('youtube_suggest', a), // @req RF-23
    youtubeSearchHistory: () => call('youtube_search_history'), // @req RF-34
    youtubeSearchHistoryClear: () => call('youtube_search_history_clear'), // @req RF-34
    ytdlpStatus: () => call('ytdlp_status'), // @req RF-26
    ytdlpUpdate: () => call('ytdlp_update'),
    youtubeAddToLibrary: (a) => call('youtube_add_to_library', a),

    downloadEnqueue: (a) => call('download_enqueue', a),
    downloadEnqueuePlaylist: (a) => call('download_enqueue_playlist', a),
    downloadList: () => call('download_list'),
    downloadCancel: (a) => call('download_cancel', a),
    downloadClearFinished: () => call('download_clear_finished'),

    metadataRefreshTrack: (a) => call('metadata_refresh_track', a),
    metadataRefreshLibrary: (a) => call('metadata_refresh_library', a), // @req RF-12 RF-20
    metadataRefreshStatus: () => call('metadata_refresh_status'), // @req RF-32
    metadataGetArtist: (a) => call('metadata_get_artist', a),
    metadataGetLyrics: (a) => call('metadata_get_lyrics', a),

    statsLibrary: () => call('stats_library'),
    statsWrapped: (a) => call('stats_wrapped', a),
    statsWrappedYears: () => call('stats_wrapped_years'),
    statsTaste: () => call('stats_taste'),
    statsTrackScores: (a) => call('stats_track_scores', a ?? {}), // @req RF-25
    statsDna: (a) => call('stats_dna', a ?? {}),
    recommendationsList: (a) => call('recommendations_list', a ?? {}),
    recommendationsRefresh: (a) => call('recommendations_refresh', a ?? {}),
    recommendationsDismiss: (a) => call('recommendations_dismiss', a),

    profileGet: () => call('profile_get'),
    profileUpdate: (a) => call('profile_update', a),
    profileSetAvatar: (a) => call('profile_set_avatar', a), // @req RF-21
    equalizerListPresets: () => call('equalizer_list_presets'),
    equalizerSavePreset: (a) => call('equalizer_save_preset', a),
    equalizerDeletePreset: (a) => call('equalizer_delete_preset', a),
    settingsGet: () => call('settings_get'),
    settingsUpdate: (a) => call('settings_update', a),
    systemGpuStatus: () => call('system_gpu_status'),
    systemGpuStartupConfirmed: () => call('system_gpu_startup_confirmed'), // @req RNF-09

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

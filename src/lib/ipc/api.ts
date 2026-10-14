// @req RF-03 RF-05 RF-10 RF-15 RF-19
import type {
  ArtistInfo, Dna, DownloadJob, EventMap, EventName, ImageUpload, LibraryStats, Lyrics, PlayerState, Playlist,
  Profile, Recommendation, SearchResult, Settings, ShuffleMode, TasteProfile, Track, TrackQuery, TrackScore, Wrapped, YtDlpStatus,
} from './types';

/** Interface única implementada pelo cliente real (Tauri) e pelo mock em memória. */
export interface MiupyApi {
  readonly kind: 'tauri' | 'mock';

  playerGetState(): Promise<PlayerState>;
  playerPlay(a: { trackId: string; queue?: string[] }): Promise<PlayerState>;
  playerPause(): Promise<PlayerState>;
  playerResume(): Promise<PlayerState>;
  playerStop(): Promise<PlayerState>;
  playerSeek(a: { positionMs: number }): Promise<PlayerState>;
  playerNext(): Promise<PlayerState>;
  playerPrevious(): Promise<PlayerState>;
  playerSetVolume(a: { volume: number }): Promise<PlayerState>;
  playerSetMuted(a: { muted: boolean }): Promise<PlayerState>;
  playerSetShuffle(a: { mode: ShuffleMode }): Promise<PlayerState>; // @req RF-24
  playerGetQueue(): Promise<Track[]>; // @req RF-03

  libraryListTracks(a?: { query?: TrackQuery }): Promise<Track[]>;
  libraryGetTrack(a: { trackId: string }): Promise<Track>;
  libraryListTracksByArtist(a: { artistId: string; query?: TrackQuery }): Promise<Track[]>;
  librarySetFavorite(a: { trackId: string; favorite: boolean }): Promise<Track>;
  libraryDeleteTrack(a: { trackId: string; deleteFile: boolean }): Promise<void>;

  playlistList(): Promise<Playlist[]>;
  playlistCreate(a: { name: string; description?: string }): Promise<Playlist>;
  playlistUpdate(a: { playlistId: string; name?: string; description?: string }): Promise<Playlist>;
  playlistDelete(a: { playlistId: string }): Promise<void>;
  playlistGetTracks(a: { playlistId: string; query?: TrackQuery }): Promise<Track[]>;
  playlistAddTracks(a: { playlistId: string; trackIds: string[] }): Promise<Playlist>;
  playlistRemoveTracks(a: { playlistId: string; trackIds: string[] }): Promise<Playlist>;
  playlistSetCover(a: { playlistId: string; image: ImageUpload | null }): Promise<Playlist>; // @req RF-22
  playlistMoveTrack(a: { playlistId: string; trackId: string; newPosition: number }): Promise<void>;

  youtubeSearch(a: { query: string; limit?: number }): Promise<SearchResult[]>;
  youtubeSuggest(a: { query: string }): Promise<string[]>; // @req RF-23
  ytdlpStatus(): Promise<YtDlpStatus>; // @req RF-26
  ytdlpUpdate(): Promise<YtDlpStatus>;
  youtubeAddToLibrary(a: { sourceIds: string[]; playlistId?: string }): Promise<Track[]>;

  downloadEnqueue(a: { trackIds: string[] }): Promise<DownloadJob[]>;
  downloadEnqueuePlaylist(a: { playlistId: string }): Promise<DownloadJob[]>;
  downloadList(): Promise<DownloadJob[]>;
  downloadCancel(a: { jobId: string }): Promise<void>;
  downloadClearFinished(): Promise<number>;

  metadataRefreshLibrary(a: { onlyMissing?: boolean }): Promise<number>; // @req RF-12 RF-20
  metadataRefreshTrack(a: { trackId: string }): Promise<Track>;
  metadataGetArtist(a: { artistId: string }): Promise<ArtistInfo>;
  metadataGetLyrics(a: { trackId: string }): Promise<Lyrics>;

  statsLibrary(): Promise<LibraryStats>;
  statsWrapped(a: { year: number }): Promise<Wrapped>;
  statsWrappedYears(): Promise<number[]>;
  statsTrackScores(a?: { trackIds?: string[] }): Promise<TrackScore[]>; // @req RF-25
  statsDna(a?: { windowDays?: number }): Promise<Dna>;
  statsTaste(): Promise<TasteProfile>; // @req RF-19
  recommendationsList(a?: { playlistId?: string; limit?: number }): Promise<Recommendation[]>;
  recommendationsRefresh(a?: { playlistId?: string }): Promise<Recommendation[]>;
  recommendationsDismiss(a: { sourceId: string }): Promise<void>;

  profileGet(): Promise<Profile>;
  profileSetAvatar(a: { image: ImageUpload | null }): Promise<Profile>; // @req RF-21
  profileUpdate(a: { profile: Profile }): Promise<Profile>;
  settingsGet(): Promise<Settings>;
  settingsUpdate(a: { settings: Partial<Settings> }): Promise<Settings>;

  /** Assina um evento; devolve a função que cancela a assinatura. */
  on<E extends EventName>(event: E, handler: (payload: EventMap[E]) => void): Promise<() => void>;
  /** Converte um caminho local (capa) em URL exibível; null se não houver. */
  resolveAsset(path: string | null): string | null;
}

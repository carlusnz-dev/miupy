import type {
  ArtistInfo, DownloadJob, EventMap, EventName, LibraryStats, Lyrics, PlayerState, Playlist,
  Profile, SearchResult, Settings, Track, TrackQuery, Wrapped,
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

  libraryListTracks(a?: { query?: TrackQuery }): Promise<Track[]>;
  libraryGetTrack(a: { trackId: string }): Promise<Track>;
  librarySetFavorite(a: { trackId: string; favorite: boolean }): Promise<Track>;
  libraryDeleteTrack(a: { trackId: string; deleteFile: boolean }): Promise<void>;

  playlistList(): Promise<Playlist[]>;
  playlistCreate(a: { name: string; description?: string }): Promise<Playlist>;
  playlistUpdate(a: { playlistId: string; name?: string; description?: string }): Promise<Playlist>;
  playlistDelete(a: { playlistId: string }): Promise<void>;
  playlistGetTracks(a: { playlistId: string; query?: TrackQuery }): Promise<Track[]>;
  playlistAddTracks(a: { playlistId: string; trackIds: string[] }): Promise<Playlist>;
  playlistRemoveTracks(a: { playlistId: string; trackIds: string[] }): Promise<Playlist>;
  playlistMoveTrack(a: { playlistId: string; trackId: string; newPosition: number }): Promise<void>;

  youtubeSearch(a: { query: string; limit?: number }): Promise<SearchResult[]>;
  youtubeAddToLibrary(a: { sourceIds: string[]; playlistId?: string }): Promise<Track[]>;

  downloadEnqueue(a: { trackIds: string[] }): Promise<DownloadJob[]>;
  downloadEnqueuePlaylist(a: { playlistId: string }): Promise<DownloadJob[]>;
  downloadList(): Promise<DownloadJob[]>;
  downloadCancel(a: { jobId: string }): Promise<void>;

  metadataRefreshTrack(a: { trackId: string }): Promise<Track>;
  metadataGetArtist(a: { artistId: string }): Promise<ArtistInfo>;
  metadataGetLyrics(a: { trackId: string }): Promise<Lyrics>;

  statsLibrary(): Promise<LibraryStats>;
  statsWrapped(a: { year: number }): Promise<Wrapped>;

  profileGet(): Promise<Profile>;
  profileUpdate(a: { profile: Profile }): Promise<Profile>;
  settingsGet(): Promise<Settings>;
  settingsUpdate(a: { settings: Partial<Settings> }): Promise<Settings>;

  /** Assina um evento; devolve a função que cancela a assinatura. */
  on<E extends EventName>(event: E, handler: (payload: EventMap[E]) => void): Promise<() => void>;
  /** Converte um caminho local (capa) em URL exibível; null se não houver. */
  resolveAsset(path: string | null): string | null;
}

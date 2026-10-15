// @req RF-18 RF-19 RF-20
// Tipos do contrato docs/arquitetura/ipc.md (manter em sincronia).

export type LavaTheme = 'orange' | 'purple' | 'blue';
export type SortDir = 'asc' | 'desc';
export type TrackSort = 'title' | 'artist' | 'album' | 'duration' | 'addedAt' | 'position';
export type DownloadStatus = 'none' | 'queued' | 'downloading' | 'done' | 'error';
// @req RF-24
export type ShuffleMode = 'off' | 'random' | 'surfle';
export type PlayerStatus = 'stopped' | 'loading' | 'playing' | 'paused';

export interface Track {
  id: string;
  title: string;
  artistId: string | null;
  artistName: string | null;
  album: string | null;
  durationMs: number;
  filePath: string | null;
  coverPath: string | null;
  source: 'youtube' | 'local';
  sourceId: string | null;
  codec: string | null;
  bitrate: number | null;
  isFavorite: boolean;
  downloadStatus: DownloadStatus;
  addedAt: number;
  genres: string[]; // @req RF-20
  releaseYear: number | null;
  loudnessLufs: number | null; // @req RF-28
}

export interface Playlist {
  id: string;
  name: string;
  description: string | null;
  coverPath: string | null;
  mosaicPaths: string[]; // @req RF-22
  isFavorites: boolean; // @req RF-30
  autoDownload: boolean; // @req RF-31
  trackCount: number;
  totalDurationMs: number;
  createdAt: number;
  updatedAt: number;
}

export interface TrackQuery {
  search?: string;
  sortBy?: TrackSort;
  sortDir?: SortDir;
  favoritesOnly?: boolean;
}

export interface PlayerState {
  status: PlayerStatus;
  track: Track | null;
  positionMs: number;
  durationMs: number;
  volume: number;
  muted: boolean;
  queue: string[];
  queueIndex: number;
  streaming: boolean; // @req RF-18
  shuffle: ShuffleMode; // @req RF-24
}

// @req RF-19
export type RecommendationReasonKind = 'similarTrack' | 'artist' | 'genre';
export interface RecommendationReason { kind: RecommendationReasonKind; label: string; }
export interface Recommendation {
  sourceId: string; title: string; artist: string; durationMs: number;
  thumbnailUrl: string | null; score: number; reasons: RecommendationReason[];
}
export interface TasteProfile {
  topArtists: { artistId: string; name: string; score: number }[];
  topGenres: { genre: string; score: number }[];
  totalListenedMs: number; skipRate: number;
}

export interface SearchResult {
  sourceId: string;
  title: string;
  artist: string;
  durationMs: number;
  thumbnailUrl: string | null;
  inLibrary: boolean;
  trackId: string | null; // @req RF-04
  isFavorite: boolean;
}

export interface DownloadJob {
  id: string;
  trackId: string;
  status: DownloadStatus;
  progress: number;
  error: string | null;
}

export interface ArtistInfo {
  id: string;
  name: string;
  mbid: string | null;
  country: string | null;
  genres: string[];
  bio: string | null;
  imagePath: string | null;
  imageCredit: string | null; // @req RF-27
  trackCount: number;
}

export interface Lyrics {
  trackId: string;
  plain: string | null;
  synced: string | null;
}

export interface Profile {
  displayName: string;
  username: string;
  email: string;
  phone: string;
  avatarPath: string | null;
}

// @req RF-28
export type Normalization = 'off' | 'quiet' | 'normal' | 'loud';
export interface Settings {
  lavaTheme: LavaTheme;
  volume: number;
  gpuMode: GpuMode;
  normalization: Normalization;
  eqPresetId: string;
  // @req RF-33
  sidebarWidth: number;
  favoritesPlaylistId: string | null; // @req RF-30
}
export type GpuMode = 'auto' | 'on' | 'off';
export interface GpuStatus { mode: GpuMode; envOverride: GpuMode | null; hardware: boolean; reason: string; restartRequired: boolean; }

export interface RankedTrack {
  track: Track;
  plays: number;
  listenedMs: number;
}
export interface RankedArtist {
  artistId: string;
  name: string;
  plays: number;
  listenedMs: number;
}

export interface LibraryStats {
  trackCount: number;
  downloadedCount: number;
  playlistCount: number;
  artistCount: number;
  favoriteCount: number;
  totalDurationMs: number;
  totalListenedMs: number;
  totalPlays: number;
  topTracks: RankedTrack[];
}

export interface Wrapped {
  year: number;
  totalListenedMs: number;
  totalPlays: number;
  distinctTracks: number;
  distinctArtists: number;
  tracksAdded: number;
  topTracks: RankedTrack[];
  topArtists: RankedArtist[];
  listenedMsByMonth: number[];
  busiestDay: { date: string; listenedMs: number } | null;
}

// @req RF-21 RF-22 RF-25 RF-26
export interface ImageUpload { dataBase64: string; mimeType: string; }
export interface TrackScore { trackId: string; score: number; plays: number; skips: number; lastPlayedAt: number | null; }
export interface DnaTrend { kind: 'artist' | 'genre'; id: string | null; label: string; current: number; previous: number; delta: number; }
export interface Dna {
  windowDays: number;
  topArtists: { artistId: string; name: string; score: number }[];
  topGenres: { genre: string; score: number }[];
  skipRate: number; previousSkipRate: number;
  listenedMs: number; previousListenedMs: number;
  rising: DnaTrend[]; falling: DnaTrend[];
}
export type YtDlpState = 'missing' | 'downloading' | 'ready' | 'error';
export interface YtDlpStatus { state: YtDlpState; version: string | null; path: string | null; checkedAt: number | null; progress: number; error: string | null; }

export type IpcErrorCode =
  | 'not_found'
  | 'invalid_input'
  | 'conflict'
  | 'io'
  | 'network'
  | 'unavailable'
  | 'internal';

export interface IpcError {
  code: IpcErrorCode | string;
  message: string;
}

export interface ProgressPayload {
  positionMs: number;
  durationMs: number;
}

// @req RF-32
export interface MetadataRefreshStatus {
  running: boolean;
  done: number;
  total: number;
  failed: number;
  currentTrackId: string | null;
  startedAt: number | null;
  finishedAt: number | null;
}

export interface EventMap {
  'player://state': PlayerState;
  'player://progress': ProgressPayload;
  'download://progress': DownloadJob;
  // @req RF-31
  'download://auto': { playlistId: string; count: number; reason: 'enabled' | 'added' | 'retry' | 'offline' };
  'library://changed': { reason: string };
  'recommendations://changed': { count: number };
  'player://error': { trackId: string; code: string; message: string };
  'metadata://progress': MetadataRefreshStatus;
  'ytdlp://status': YtDlpStatus;
}
export type EventName = keyof EventMap;

// @req RF-29
export const EQ_BANDS_HZ = [60,170,310,600,1000,3000,6000,12000] as const;
export interface EqPreset { id: string; name: string; builtin: boolean; gainsDb: number[]; }
export interface EqPresetInput { id?: string; name: string; gainsDb: number[]; }

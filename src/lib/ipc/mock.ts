// @req RF-18 RF-19 RF-20
// Mock em memória do contrato IPC, para navegar a UI sem backend (npm run dev no navegador).
// Mesma interface (MiupyApi) do cliente real. Dados fictícios.
import type { MiupyApi } from './api';
import { isWaitingNetwork } from './download';
import type {
  ArtistInfo, Dna, DownloadJob, EventMap, EventName, LibraryStats, Lyrics, MetadataRefreshStatus, PlayerState, Playlist, YtDlpStatus,
  EqPreset, Profile, RankedArtist, RankedTrack, Recommendation, SearchResult, Settings, Track, TrackQuery, Wrapped,
} from './types';

const MIN = 60_000;
const DAY = 86_400_000;
// @req RF-22 RF-27: capas locais simuladas como data URL para o navegador puro.
function mockArt(seed: number): string {
  const colors = ['#ed7149', '#7d60bf', '#388fb0', '#efae5c', '#765375', '#5a9a8f', '#c76572', '#9a674d'];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300"><rect width="300" height="300" fill="${colors[seed % colors.length]}"/><circle cx="${80 + seed * 14}" cy="100" r="105" fill="#ffffff" opacity=".18"/><path d="M0 230 Q 90 ${90 + seed * 8} 170 210 T 300 180 V300 H0" fill="#171821" opacity=".38"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

function err(code: string, message: string): never {
  throw { code, message };
}
/** `?latency=ms` soma atraso a toda chamada do mock: serve para ver os esqueletos (RNF-11). */
const extraLatency = typeof location === 'undefined' ? 0 : Number(new URLSearchParams(location.search).get('latency')) || 0;
const wait = (ms = 70) => new Promise((r) => setTimeout(r, ms + extraLatency));
let seq = 1000;
const uid = (p: string) => `${p}-${(seq++).toString(16).padStart(6, '0')}`;

/** PRNG determinístico para os dados de exemplo ficarem estáveis entre recargas. */
function rng(seed: number) {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x6d2b79f5) >>> 0;
    let t = s;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const ARTISTS = [
  { id: 'art-1', name: 'Aurora Basalto', country: 'BR', genres: ['indie', 'dream pop'], bio: 'Duo de Belo Horizonte que mistura sintetizadores analógicos com guitarras reverberadas. Formado em 2016, lançou três álbuns gravados em fita.' },
  { id: 'art-2', name: 'Cinza Vulcânica', country: 'PT', genres: ['pós-rock', 'ambient'], bio: 'Quarteto instrumental conhecido por crescendos longos e texturas densas.' },
  { id: 'art-3', name: 'Marés de Obsidiana', country: 'BR', genres: ['mpb', 'eletrônica'], bio: 'Projeto solo que une violão de nylon e batidas programadas.' },
  { id: 'art-4', name: 'Lume', country: 'BR', genres: ['pop', 'synthpop'], bio: null },
  { id: 'art-5', name: 'Quartzo Rosa', country: 'AR', genres: ['rock alternativo'], bio: 'Banda de Buenos Aires com letras em português e espanhol.' },
  { id: 'art-6', name: 'Noite de Magma', country: 'BR', genres: ['trip hop', 'downtempo'], bio: null },
  { id: 'art-7', name: 'Ilha Cinzenta', country: 'CV', genres: ['morna', 'jazz'], bio: 'Vozes e metais de Cabo Verde em arranjos minimalistas.' },
  { id: 'art-8', name: 'Faísca', country: 'BR', genres: ['funk', 'soul'], bio: null },
];

const TITLES: [number, string, string | null, number][] = [
  [0, 'Cratera de Neon', 'Fogo Lento', 224], [0, 'Brasa Azul', 'Fogo Lento', 197], [0, 'Madrugada de Cinzas', 'Fogo Lento', 251],
  [0, 'Ponte de Vidro', 'Luz Baixa', 208], [1, 'Erupção Silenciosa', 'Placas', 412], [1, 'Camadas', 'Placas', 335],
  [1, 'Deriva Continental', 'Placas', 506], [1, 'Pó e Eco', 'Fumarolas', 289], [2, 'Maré Alta', 'Vidro Negro', 213],
  [2, 'Violão de Lava', 'Vidro Negro', 242], [2, 'Sal e Pedra', 'Vidro Negro', 186], [2, 'Horizonte Quente', 'Sal', 231],
  [3, 'Acende', 'Faísca Viva', 178], [3, 'Dança no Vulcão', 'Faísca Viva', 203], [3, 'Luz de Néon', 'Faísca Viva', 195],
  [3, 'Verão Eterno', null, 221], [4, 'Cuarzo', 'Cristales', 262], [4, 'Fronteira', 'Cristales', 238],
  [4, 'Rosa do Deserto', 'Cristales', 219], [5, 'Magma Lento', 'Camada Terra', 301], [5, 'Fumaça e Neblina', 'Camada Terra', 276],
  [5, 'Subsolo', 'Camada Terra', 254], [6, 'Morna de Cinza', 'Mar Aberto', 270], [6, 'Sodade Mineral', 'Mar Aberto', 293],
  [7, 'Chama Viva', 'Calor Humano', 188], [7, 'Soul de Pedra', 'Calor Humano', 205], [7, 'Brasil em Brasa', null, 199],
  [0, 'Último Sol', 'Luz Baixa', 233],
];

const LYRICS_SYNCED = `[00:00.00] (introdução)
[00:12.00] A noite desce devagar sobre a pedra
[00:19.00] e o chão ainda guarda o calor do dia
[00:27.00] Cada brasa conta uma história antiga
[00:35.00] que ninguém ousou escrever em papel
[00:44.00] Fica comigo até a lava esfriar
[00:52.00] e o vento levar o que restou
[01:02.00] Fica comigo até a lava esfriar
[01:10.00] que o amanhecer já vem de longe
[01:25.00] (instrumental)
[01:50.00] A noite sobe devagar sobre a pedra
[01:58.00] e o chão ainda guarda o calor do dia`;

/** `?perfTracks=N` (RNF-13): replica o acervo de exemplo para medir com uma discoteca de verdade. */
function perfTrackCount(): number {
  const n = typeof location === 'undefined' ? 0 : Number(new URLSearchParams(location.search).get('perfTracks'));
  return Number.isFinite(n) && n > TITLES.length ? Math.min(n, 5000) : TITLES.length;
}

function buildTracks(): Track[] {
  const r = rng(7);
  const now = Date.now();
  const count = perfTrackCount();
  return Array.from({ length: count }, (_, i) => [...TITLES[i % TITLES.length], i] as const).map(([ai, title0, album, secs, i0]) => {
    const i = i0;
    const title = i < TITLES.length ? title0 : `${title0} ${Math.floor(i / TITLES.length) + 1}`;
    const a = ARTISTS[ai];
    const status = r() < 0.7 ? 'done' : r() < 0.5 ? 'none' : 'queued';
    const dl = i % 9 === 8 ? 'error' : status;
    return {
      id: `trk-${(i + 1).toString().padStart(3, '0')}`,
      title,
      artistId: a.id,
      artistName: a.name,
      album,
      durationMs: secs * 1000,
      filePath: dl === 'done' ? `/dados/miupy/audio/trk-${i + 1}.m4a` : null,
      coverPath: i % 4 === 0 ? null : mockArt(ai),
      source: i % 7 === 6 ? 'local' : 'youtube',
      sourceId: `yt${i}abc${(i * 37) % 997}`,
      codec: dl === 'done' ? 'aac' : null,
      bitrate: dl === 'done' ? 128 : null,
      isFavorite: r() < 0.25,
      downloadStatus: dl === 'queued' ? 'none' : dl,
      addedAt: now - Math.floor(r() * 200) * DAY,
      genres: [...a.genres], releaseYear: 2018 + i % 8, loudnessLufs: dl === 'done' ? -16 + i % 8 : null,
    } satisfies Track;
  });
}

const YT_POOL: Omit<SearchResult, 'inLibrary' | 'trackId' | 'isFavorite'>[] = [
  { sourceId: 'yt-n01', title: 'Vulcão (ao vivo)', artist: 'Aurora Basalto', durationMs: 262000, thumbnailUrl: null },
  { sourceId: 'yt-n02', title: 'Rio de Fogo', artist: 'Marés de Obsidiana', durationMs: 215000, thumbnailUrl: null },
  { sourceId: 'yt-n03', title: 'Lava Lamp', artist: 'Noite de Magma', durationMs: 244000, thumbnailUrl: null },
  { sourceId: 'yt-n04', title: 'Cinzas ao Vento', artist: 'Cinza Vulcânica', durationMs: 388000, thumbnailUrl: null },
  { sourceId: 'yt-n05', title: 'Faísca no Escuro', artist: 'Faísca', durationMs: 192000, thumbnailUrl: null },
  { sourceId: 'yt-n06', title: 'Coração de Basalto', artist: 'Lume', durationMs: 207000, thumbnailUrl: null },
  { sourceId: 'yt-n07', title: 'Mar de Pedra', artist: 'Ilha Cinzenta', durationMs: 281000, thumbnailUrl: null },
  { sourceId: 'yt-n08', title: 'Quartzo (versão acústica)', artist: 'Quartzo Rosa', durationMs: 226000, thumbnailUrl: null },
];

export function createMockApi(): MiupyApi {
  let tracks = buildTracks();
  const now = Date.now();
  let playlists: (Playlist & { ids: string[] })[] = [
    { id: 'pl-1', name: 'Fogo Baixo', description: 'Para trabalhar com calma.', coverPath: null, mosaicPaths: [], isFavorites: false, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: now - 90 * DAY, updatedAt: now - 3 * DAY, ids: ['trk-005', 'trk-006', 'trk-008', 'trk-020', 'trk-021', 'trk-022'] },
    { id: 'pl-2', name: 'Estrada de Verão', description: null, coverPath: null, mosaicPaths: [], isFavorites: false, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: now - 40 * DAY, updatedAt: now - 1 * DAY, ids: ['trk-013', 'trk-014', 'trk-015', 'trk-016', 'trk-025', 'trk-026', 'trk-001'] },
    { id: 'pl-3', name: 'Favoritas da Casa', description: 'O que nunca sai de rotação.', coverPath: null, mosaicPaths: [], isFavorites: false, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: now - 10 * DAY, updatedAt: now - 10 * DAY, ids: ['trk-002', 'trk-010', 'trk-017', 'trk-023'] },
  ];
  playlists[2].ids.push(...tracks.filter((t) => t.isFavorite && !playlists[2].ids.includes(t.id)).map((t) => t.id));
  tracks.forEach((t) => { t.isFavorite = playlists[2].ids.includes(t.id); });
  let jobs: DownloadJob[] = [];
  let profile: Profile = { displayName: 'Carlos Antunes', username: 'cabeto', email: 'carlos@example.com', phone: '+55 31 99999-0000', avatarPath: null };
  // @req RF-33
  let settings: Settings = { lavaTheme: 'orange', volume: 0.8, gpuMode: 'auto', normalization: 'normal', eqPresetId: 'flat', sidebarWidth: 280, favoritesPlaylistId: 'pl-3' };
  // @req RF-29
  const presets: EqPreset[] = [
    ['flat','Plano',[0,0,0,0,0,0,0,0]], ['bass','Graves',[6,5,3,1,0,0,0,0]],
    ['treble','Agudos',[0,0,0,0,1,3,5,6]], ['vocal','Vocal',[-2,-1,0,2,4,4,2,0]],
    ['rock','Rock',[4,3,-1,-2,-1,2,4,5]], ['pop','Pop',[-1,1,3,4,3,0,-1,-1]],
    ['electronic','Eletrônica',[5,4,1,0,-1,2,3,4]], ['acoustic','Acústico',[3,2,1,1,2,2,3,2]],
  ].map(([id,name,gainsDb]) => ({id:id as string,name:name as string,gainsDb:gainsDb as number[],builtin:true}));
  let eqSeq = 0;
  let searchHistory: string[] = ['Daft Punk', 'Björk', 'Sigur Rós']; // @req RF-34
  let ytdlp: YtDlpStatus = { state: 'ready', version: '2026.08.19', path: '/dados/miupy/bin/yt-dlp', checkedAt: Date.now() - DAY, progress: 1, error: null }; // @req RF-26
  // @req RF-32
  let metadataStatus: MetadataRefreshStatus = { running: false, done: 0, total: 0, failed: 0, currentTrackId: null, startedAt: null, finishedAt: null };
  const youtubeIds = new Map<string, string>(); // sourceId -> trackId
  const dismissed = new Set<string>(); // @req RF-19

  const listeners = new Map<string, Set<(p: never) => void>>();
  const emit = <E extends EventName>(e: E, p: EventMap[E]) => listeners.get(e)?.forEach((h) => h(p as never));
  // RNF-13: o medidor (lib/dev/perf.ts) dispara eventos por aqui para simular rajadas.
  if (typeof window !== 'undefined') (window as unknown as { __miupyMockEmit: unknown }).__miupyMockEmit = emit;

  const plays = new Map<string, number>();
  const r0 = rng(21);
  tracks.forEach((t, i) => plays.set(t.id, Math.floor(r0() * r0() * 90) + (i < 6 ? 25 : 0)));

  const byId = (id: string) => tracks.find((t) => t.id === id) ?? err('not_found', 'Faixa não encontrada.');
  const refreshPl = (p: Playlist & { ids: string[] }) => {
    const ts = p.ids.map((i) => tracks.find((t) => t.id === i)).filter(Boolean) as Track[];
    // @req RF-22: uma imagem representativa por artista, até quatro.
    p.mosaicPaths = [...new Map(ts.filter((t) => t.artistId && t.coverPath).map((t) => [t.artistId, t.coverPath!])).values()].slice(0, 4);
    p.trackCount = ts.length;
    p.totalDurationMs = ts.reduce((s, t) => s + t.durationMs, 0);
  };
  playlists.forEach(refreshPl);
  const plOut = (p: Playlist & { ids: string[] }): Playlist => {
    refreshPl(p);
    p.isFavorites = settings.favoritesPlaylistId === p.id;
    const { ids: _ids, ...rest } = p;
    return rest;
  };
  const getPl = (id: string) => playlists.find((p) => p.id === id) ?? err('not_found', 'Playlist não encontrada.');
  const changed = (reason: string) => { emit('library://changed', { reason }); emit('recommendations://changed', { count: 0 }); };
  const recommendations = (playlistId?: string, limit = 20): Recommendation[] => {
    const basis = playlistId ? getPl(playlistId).ids.map(byId) : tracks;
    if (!basis.length) return [];
    return YT_POOL.filter((p) => !dismissed.has(p.sourceId) && !tracks.some((t) => t.sourceId === p.sourceId))
      .slice(0, Math.min(100, limit)).map((p, i) => {
        const match = basis.find((t) => t.artistName === p.artist) ?? basis[i % basis.length];
        return { ...p, score: Math.max(0.2, 0.94 - i * 0.07), reasons: [
          { kind: 'artist' as const, label: match.artistName ?? p.artist },
          ...(match.genres[0] ? [{ kind: 'genre' as const, label: match.genres[0] }] : []),
        ] };
      });
  };

  function applyQuery(list: Track[], q?: TrackQuery): Track[] {
    let out = [...list];
    if (q?.favoritesOnly) out = out.filter((t) => t.isFavorite);
    const s = q?.search?.trim().toLowerCase();
    if (s) out = out.filter((t) => [t.title, t.artistName, t.album].some((v) => v?.toLowerCase().includes(s)));
    const by = q?.sortBy;
    if (by && by !== 'position') {
      const dir = q?.sortDir === 'desc' ? -1 : 1;
      const key = (t: Track) =>
        by === 'title' ? t.title.toLowerCase() : by === 'artist' ? (t.artistName ?? '').toLowerCase()
        : by === 'album' ? (t.album ?? '').toLowerCase() : by === 'duration' ? t.durationMs : t.addedAt;
      out.sort((a, b) => (key(a) < key(b) ? -dir : key(a) > key(b) ? dir : 0));
    }
    return out;
  }

  // --- Player simulado ---
  let player: PlayerState = { status: 'stopped', track: null, positionMs: 0, durationMs: 0, volume: settings.volume, muted: false, queue: [], queueIndex: -1, streaming: false, shuffle: 'off' };
  let timer: ReturnType<typeof setInterval> | null = null;
  const snap = (): PlayerState => ({ ...player, queue: [...player.queue] });
  const pushState = () => { emit('player://state', snap()); return snap(); };
  function stopTimer() { if (timer) clearInterval(timer); timer = null; }
  function startTimer() {
    stopTimer();
    timer = setInterval(() => {
      if (player.status !== 'playing') return;
      player.positionMs = Math.min(player.durationMs, player.positionMs + 500);
      if (player.positionMs >= player.durationMs) {
        if (player.queueIndex + 1 < player.queue.length) loadAt(player.queueIndex + 1);
        else { player.status = 'stopped'; player.positionMs = 0; stopTimer(); pushState(); }
        return;
      }
      emit('player://progress', { positionMs: player.positionMs, durationMs: player.durationMs });
    }, 500);
  }
  function loadAt(idx: number) {
    const t = byId(player.queue[idx]);
    player.queueIndex = idx;
    player.track = t;
    player.streaming = t.source === 'youtube' && !t.filePath;
    player.durationMs = t.durationMs;
    player.positionMs = 0;
    player.status = player.streaming ? 'loading' : 'playing';
    pushState();
    if (player.streaming) setTimeout(() => {
      if (player.track?.id !== t.id || player.status !== 'loading') return;
      if (t.id === 'trk-009') { // @req RF-18: falha tardia de streaming no mock.
        player.status = 'stopped'; pushState();
        emit('player://error', { trackId: t.id, code: 'unavailable', message: 'Este vídeo não está disponível para reprodução.' });
      } else { player.status = 'playing'; startTimer(); pushState(); }
    }, 450);
    else startTimer();
  }

  // --- Downloads simulados ---
  let dlTimer: ReturnType<typeof setInterval> | null = null;
  function ensureDl() {
    if (dlTimer) return;
    dlTimer = setInterval(() => {
      const active = jobs.find((j) => j.status === 'downloading') ?? jobs.find((j) => j.status === 'queued');
      if (!active) { clearInterval(dlTimer!); dlTimer = null; return; }
      const t = tracks.find((x) => x.id === active.trackId);
      if (active.status === 'queued') { active.status = 'downloading'; if (t) t.downloadStatus = 'downloading'; }
      active.progress = Math.min(1, active.progress + 0.06 + Math.random() * 0.08);
      if (active.progress >= 1) {
        active.status = 'done';
        if (t) { t.downloadStatus = 'done'; t.filePath = `/dados/miupy/audio/${t.id}.m4a`; t.codec = 'aac'; t.bitrate = 128; }
        changed('download');
      }
      emit('download://progress', { ...active });
    }, 450);
  }
  function enqueue(ids: string[]): DownloadJob[] {
    const out: DownloadJob[] = [];
    for (const id of ids) {
      const t = byId(id);
      if (t.downloadStatus === 'done') continue;
      const ex = jobs.find((j) => j.trackId === id && (j.status === 'queued' || j.status === 'downloading'));
      if (ex) { out.push({ ...ex }); continue; }
      jobs = jobs.filter((j) => j.trackId !== id);
      const job: DownloadJob = { id: uid('job'), trackId: id, status: 'queued', progress: 0, error: null };
      t.downloadStatus = 'queued';
      jobs.push(job);
      out.push({ ...job });
      emit('download://progress', { ...job });
    }
    changed('download');
    ensureDl();
    return out;
  }

  // @req RF-31
  function autoEnqueue(p: Playlist & { ids: string[] }, ids: string[], reason: 'enabled' | 'added') {
    if (!p.autoDownload) return;
    const pending = [...new Set(ids)].filter((id) => ['none', 'error'].includes(byId(id).downloadStatus));
    enqueue(pending);
    emit('download://auto', { playlistId: p.id, count: pending.length, reason });
  }

  const artistTracks = (id: string) => tracks.filter((t) => t.artistId === id);

  const api: MiupyApi = {
    kind: 'mock',

    async playerGetState() { await wait(20); return snap(); },
    async playerPlay({ trackId, queue }) {
      await wait(30);
      const target = byId(trackId);
      if (target.source === 'local' && !target.filePath) err('unavailable', 'Arquivo local indisponível.');
      const q = queue && queue.length ? queue : player.queue.includes(trackId) ? player.queue : [trackId];
      player.queue = q.includes(trackId) ? [...q] : [trackId, ...q];
      loadAt(player.queue.indexOf(trackId));
      return snap();
    },
    async playerPause() { if (player.status === 'playing') { player.status = 'paused'; } return pushState(); },
    async playerResume() { if (player.track && player.status === 'paused') { player.status = 'playing'; startTimer(); } return pushState(); },
    async playerStop() { player.status = 'stopped'; player.positionMs = 0; stopTimer(); return pushState(); },
    async playerSeek({ positionMs }) {
      player.positionMs = Math.max(0, Math.min(player.durationMs, positionMs));
      emit('player://progress', { positionMs: player.positionMs, durationMs: player.durationMs });
      return snap();
    },
    async playerNext() { if (player.queueIndex + 1 < player.queue.length) loadAt(player.queueIndex + 1); return snap(); },
    async playerPrevious() {
      if (player.positionMs > 3000 || player.queueIndex <= 0) { player.positionMs = 0; return pushState(); }
      loadAt(player.queueIndex - 1);
      return snap();
    },
    async playerSetVolume({ volume }) { player.volume = Math.max(0, Math.min(1, volume)); settings.volume = player.volume; return pushState(); },
    async playerSetMuted({ muted }) { player.muted = muted; return pushState(); },
    async playerSetShuffle({ mode }) { player.shuffle = mode; return pushState(); }, // @req RF-24: aceita off, random e surfle
    async playerGetQueue() { return player.queue.map((id) => tracks.find((t) => t.id === id)).filter((t): t is Track => !!t).map((t) => ({ ...t })); },

    async libraryListTracks(a) { await wait(); return applyQuery(tracks, a?.query).map((t) => ({ ...t })); },
    async libraryGetTrack({ trackId }) { return { ...byId(trackId) }; },
    async libraryListTracksByArtist({ artistId, query }) { return applyQuery(artistTracks(artistId), query).map((t) => ({ ...t })); },
    // @req RF-04 RF-30
    async librarySetFavorite({ trackId, favorite }) {
      const t = byId(trackId);
      if (favorite && !settings.favoritesPlaylistId) {
        const p = { id: uid('pl'), name: 'Favoritas', description: null, coverPath: null, mosaicPaths: [], isFavorites: true, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: Date.now(), updatedAt: Date.now(), ids: [] as string[] };
        playlists.push(p); settings.favoritesPlaylistId = p.id;
      }
      const p = settings.favoritesPlaylistId ? getPl(settings.favoritesPlaylistId) : null;
      const addedFavorite = !!p && favorite && !p.ids.includes(trackId);
      if (p) p.ids = favorite ? [...new Set([...p.ids, trackId])] : p.ids.filter((id) => id !== trackId);
      if (addedFavorite && p) autoEnqueue(p, [trackId], 'added');
      t.isFavorite = !!p?.ids.includes(trackId);
      if (player.track?.id === trackId) { player.track = { ...t }; pushState(); }
      changed('favorites');
      return { ...t };
    },
    async libraryDeleteTrack({ trackId, deleteFile }) {
      byId(trackId);
      void deleteFile;
      tracks = tracks.filter((t) => t.id !== trackId);
      playlists.forEach((p) => (p.ids = p.ids.filter((i) => i !== trackId)));
      changed('track-deleted');
    },

    async playlistList() { await wait(); return playlists.map(plOut); },
    // @req RF-30
    async favoritesSetPlaylist({ playlistId }) {
      const target = getPl(playlistId);
      if (settings.favoritesPlaylistId === playlistId) return plOut(target);
      const old = settings.favoritesPlaylistId ? getPl(settings.favoritesPlaylistId) : null;
      for (const id of old?.ids ?? []) if (!target.ids.includes(id)) target.ids.push(id);
      settings.favoritesPlaylistId = playlistId;
      tracks.forEach((t) => { t.isFavorite = target.ids.includes(t.id); });
      changed('favorites');
      return plOut(target);
    },
    async playlistCreate({ name, description }) {
      if (!name.trim()) err('invalid_input', 'O nome da playlist não pode ficar vazio.');
      if (playlists.some((p) => p.name.toLowerCase() === name.trim().toLowerCase())) err('conflict', 'Já existe uma playlist com esse nome.');
      const p = { id: uid('pl'), name: name.trim(), description: description?.trim() || null, coverPath: null, mosaicPaths: [], isFavorites: false, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: Date.now(), updatedAt: Date.now(), ids: [] as string[] };
      playlists.push(p);
      changed('playlist');
      return plOut(p);
    },
    async playlistUpdate({ playlistId, name, description, autoDownload }) {
      const p = getPl(playlistId);
      if (name !== undefined) { if (!name.trim()) err('invalid_input', 'O nome da playlist não pode ficar vazio.'); p.name = name.trim(); }
      if (description !== undefined) p.description = description.trim() || null;
      // @req RF-31
      if (autoDownload !== undefined) { const was = p.autoDownload; p.autoDownload = autoDownload; if (autoDownload && !was) autoEnqueue(p, p.ids, 'enabled'); }
      p.updatedAt = Date.now();
      changed('playlist');
      return plOut(p);
    },
    async playlistDelete({ playlistId }) { getPl(playlistId); playlists = playlists.filter((p) => p.id !== playlistId); if (settings.favoritesPlaylistId === playlistId) { settings.favoritesPlaylistId = null; tracks.forEach((t) => { t.isFavorite = false; }); changed('favorites'); } changed('playlist_deleted'); },
    async playlistGetTracks({ playlistId, query }) {
      await wait();
      const p = getPl(playlistId);
      const list = p.ids.map((i) => tracks.find((t) => t.id === i)).filter(Boolean) as Track[];
      return applyQuery(list, query).map((t) => ({ ...t }));
    },
    async playlistAddTracks({ playlistId, trackIds }) {
      const p = getPl(playlistId);
      const added: string[] = [];
      for (const id of trackIds) { byId(id); if (!p.ids.includes(id)) { p.ids.push(id); added.push(id); } }
      autoEnqueue(p, added, 'added');
      p.updatedAt = Date.now();
      if (p.id === settings.favoritesPlaylistId) { tracks.forEach((t) => { t.isFavorite = p.ids.includes(t.id); }); changed('favorites'); }
      changed('playlist');
      return plOut(p);
    },
    async playlistRemoveTracks({ playlistId, trackIds }) {
      const p = getPl(playlistId);
      p.ids = p.ids.filter((i) => !trackIds.includes(i));
      if (p.id === settings.favoritesPlaylistId) { tracks.forEach((t) => { t.isFavorite = p.ids.includes(t.id); }); changed('favorites'); }
      p.updatedAt = Date.now();
      changed('playlist');
      return plOut(p);
    },
    async playlistSetCover({ playlistId, image }) { // @req RF-22
      const p = getPl(playlistId);
      p.coverPath = image ? `data:${image.mimeType};base64,${image.dataBase64}` : null;
      p.updatedAt = Date.now(); changed('playlist'); return plOut(p);
    },
    async playlistMoveTrack({ playlistId, trackId, newPosition }) {
      const p = getPl(playlistId);
      const from = p.ids.indexOf(trackId);
      if (from < 0) err('not_found', 'A faixa não está na playlist.');
      p.ids.splice(from, 1);
      p.ids.splice(Math.max(0, Math.min(p.ids.length, newPosition)), 0, trackId);
      changed('playlist');
    },

    async youtubeSearch({ query, limit }) {
      await wait(500);
      const q = query.trim().toLowerCase();
      if (!q) err('invalid_input', 'Digite algo para buscar.');
      const shown = query.split(/\s+/).filter(Boolean).join(' '); // @req RF-34 (como o Rust: sem espaços extras, caixa nem acento)
      const key = (v: string) => v.normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase('pt-BR');
      searchHistory = [shown, ...searchHistory.filter((h) => key(h) !== key(shown))].slice(0, 50);
      const lib = tracks.filter((t) => [t.title, t.artistName].some((v) => v?.toLowerCase().includes(q)));
      const fromLib: SearchResult[] = lib.map((t) => ({ sourceId: t.sourceId ?? t.id, title: t.title, artist: t.artistName ?? '', durationMs: t.durationMs, thumbnailUrl: null, inLibrary: true, trackId: t.id, isFavorite: t.isFavorite }));
      const pool: SearchResult[] = YT_POOL.map((p) => ({ ...p, inLibrary: youtubeIds.has(p.sourceId), trackId: youtubeIds.get(p.sourceId) ?? null, isFavorite: tracks.find((t) => t.id === youtubeIds.get(p.sourceId))?.isFavorite ?? false }));
      const rest = pool.filter((p) => `${p.title} ${p.artist}`.toLowerCase().includes(q) && !lib.some((t) => t.sourceId === p.sourceId));
      const extra = rest.length ? [] : pool.slice(0, 3); // consulta sem correspondência: sugere algo
      return [...fromLib, ...rest, ...extra].slice(0, limit ?? 20);
    },
    async youtubeSuggest({ query }) { // @req RF-23
      await wait(90);
      const q = query.trim().toLocaleLowerCase('pt-BR');
      if (!q) return [];
      return [...new Set([...tracks.map((t) => t.title), ...ARTISTS.map((a) => a.name), ...YT_POOL.map((t) => t.title)])]
        .filter((x) => x.toLocaleLowerCase('pt-BR').includes(q))
        .concat(['', ' ao vivo', ' acústico', ' letra', ' álbum completo'].map((s) => query.trim() + s))
        .filter((x, i, all) => all.indexOf(x) === i).slice(0, 8);
    },
    async youtubeSearchHistory() { return searchHistory.slice(0, 10); }, // @req RF-34
    async youtubeSearchHistoryClear() { searchHistory = []; }, // @req RF-34
    async ytdlpStatus() { return { ...ytdlp }; }, // @req RF-26
    async ytdlpUpdate() {
      ytdlp = { ...ytdlp, state: 'downloading', progress: 0, error: null }; emit('ytdlp://status', { ...ytdlp });
      const id = setInterval(() => {
        ytdlp.progress = Math.min(1, ytdlp.progress + 0.25);
        if (ytdlp.progress === 1) { clearInterval(id); ytdlp.state = 'ready'; ytdlp.checkedAt = Date.now(); }
        emit('ytdlp://status', { ...ytdlp });
      }, 300);
      return { ...ytdlp };
    },
    async youtubeAddToLibrary({ sourceIds, playlistId }) {
      await wait(200);
      const out: Track[] = [];
      for (const sid of sourceIds) {
        const ex = tracks.find((t) => t.sourceId === sid) ?? (youtubeIds.has(sid) ? tracks.find((t) => t.id === youtubeIds.get(sid)) : undefined);
        if (ex) { out.push({ ...ex }); continue; }
        const src = YT_POOL.find((p) => p.sourceId === sid) ?? err('not_found', 'Resultado não encontrado.');
        const art = ARTISTS.find((a) => a.name === src.artist);
        const t: Track = { id: uid('trk'), title: src.title, artistId: art?.id ?? null, artistName: src.artist, album: null, durationMs: src.durationMs, filePath: null, coverPath: null, source: 'youtube', sourceId: sid, codec: null, bitrate: null, isFavorite: false, downloadStatus: 'none', addedAt: Date.now(), genres: [], releaseYear: null, loudnessLufs: null };
        tracks.push(t);
        youtubeIds.set(sid, t.id);
        out.push({ ...t });
      }
      if (playlistId) { const p = getPl(playlistId); const added: string[] = []; out.forEach((t) => { if (!p.ids.includes(t.id)) { p.ids.push(t.id); added.push(t.id); } }); autoEnqueue(p, added, 'added'); }
      changed('youtube-add');
      return out;
    },

    async downloadEnqueue({ trackIds }) { await wait(40); return enqueue(trackIds); },
    async downloadEnqueuePlaylist({ playlistId }) { await wait(40); return enqueue(getPl(playlistId).ids); },
    async downloadList() { await wait(30); return jobs.map((j) => ({ ...j })); },
    async downloadClearFinished() { // @req RF-31
      for (const j of jobs) if (isWaitingNetwork(j) && !playlists.some((p) => p.autoDownload && p.ids.includes(j.trackId))) j.error = j.error?.slice('auto_offline:'.length) ?? null;
      const n = jobs.filter((j) => j.status === 'done' || (j.status === 'error' && !isWaitingNetwork(j))).length;
      jobs = jobs.filter((j) => j.status !== 'done' && (j.status !== 'error' || isWaitingNetwork(j)));
      return n;
    },
    async downloadCancel({ jobId }) {
      const j = jobs.find((x) => x.id === jobId) ?? err('not_found', 'Download não encontrado.');
      if (j.status !== 'done') {
        jobs = jobs.filter((x) => x.id !== jobId);
        const t = tracks.find((x) => x.id === j.trackId);
        if (t) t.downloadStatus = 'none';
        emit('download://progress', { ...j, status: 'none', error: null });
        changed('download');
      }
    },

    async metadataRefreshTrack({ trackId }) { await wait(400); const t = byId(trackId); if (!t.album) t.album = 'Single'; changed('metadata'); return { ...t }; },
    async metadataRefreshLibrary({ onlyMissing = true }) { // @req RF-12 RF-20 RF-32
      if (metadataStatus.running) err('conflict', 'Um reprocessamento já está em andamento.');
      const batch = onlyMissing ? tracks.filter((t) => !t.album) : [...tracks];
      metadataStatus = { running: true, done: 0, total: batch.length, failed: 0, currentTrackId: null, startedAt: Date.now(), finishedAt: null };
      emit('metadata://progress', { ...metadataStatus });
      void (async () => {
        for (const [index, track] of batch.entries()) {
          metadataStatus.currentTrackId = track.id;
          await wait(50);
          if (!track.album) track.album = 'Single';
          metadataStatus.done = index + 1;
          emit('metadata://progress', { ...metadataStatus });
        }
        changed('metadata');
        metadataStatus = { ...metadataStatus, running: false, currentTrackId: null, finishedAt: Date.now() };
        emit('metadata://progress', { ...metadataStatus });
      })();
      return batch.length;
    },
    async metadataRefreshStatus() { return { ...metadataStatus }; }, // @req RF-32
    async metadataGetArtist({ artistId }) {
      await wait();
      const a = ARTISTS.find((x) => x.id === artistId) ?? err('not_found', 'Artista não encontrado.');
      const info: ArtistInfo = { id: a.id, name: a.name, mbid: null, country: a.country, genres: a.genres, bio: a.bio, imagePath: mockArt(Number(a.id.slice(-1)) - 1), imageCredit: 'Arte de demonstração · miupy', trackCount: artistTracks(a.id).length };
      return info;
    },
    async metadataGetLyrics({ trackId }) {
      await wait(150);
      byId(trackId);
      const idx = Number(trackId.slice(4)) % 3;
      const l: Lyrics = idx === 0 ? { trackId, plain: null, synced: null } : idx === 1 ? { trackId, plain: LYRICS_SYNCED.replace(/\[[\d:.]+\] ?/g, ''), synced: null } : { trackId, plain: null, synced: LYRICS_SYNCED };
      return l;
    },

    async statsLibrary() {
      await wait();
      const top: RankedTrack[] = [...tracks].sort((a, b) => (plays.get(b.id) ?? 0) - (plays.get(a.id) ?? 0)).slice(0, 5)
        .map((t) => ({ track: { ...t }, plays: plays.get(t.id) ?? 0, listenedMs: (plays.get(t.id) ?? 0) * t.durationMs * 0.85 }));
      const totalPlays = [...plays.values()].reduce((s, n) => s + n, 0);
      const listened = tracks.reduce((s, t) => s + (plays.get(t.id) ?? 0) * t.durationMs * 0.85, 0);
      const s: LibraryStats = {
        trackCount: tracks.length,
        downloadedCount: tracks.filter((t) => t.downloadStatus === 'done').length,
        playlistCount: playlists.length,
        artistCount: new Set(tracks.map((t) => t.artistId)).size,
        favoriteCount: tracks.filter((t) => t.isFavorite).length,
        totalDurationMs: tracks.reduce((a, t) => a + t.durationMs, 0),
        totalListenedMs: Math.round(listened),
        totalPlays,
        topTracks: top,
      };
      return s;
    },
    async statsWrapped({ year }) {
      await wait(250);
      const r = rng(year);
      const ranked: RankedTrack[] = tracks.map((t) => {
        const p = Math.floor(r() * r() * 80) + (t.isFavorite ? 12 : 0);
        return { track: { ...t }, plays: p, listenedMs: Math.round(p * t.durationMs * (0.7 + r() * 0.25)) };
      }).filter((x) => x.plays > 0).sort((a, b) => b.plays - a.plays);
      const artistMap = new Map<string, RankedArtist>();
      for (const rt of ranked) {
        if (!rt.track.artistId) continue;
        const cur = artistMap.get(rt.track.artistId) ?? { artistId: rt.track.artistId, name: rt.track.artistName ?? '', plays: 0, listenedMs: 0 };
        cur.plays += rt.plays; cur.listenedMs += rt.listenedMs;
        artistMap.set(rt.track.artistId, cur);
      }
      const total = ranked.reduce((s, x) => s + x.listenedMs, 0);
      const weights = Array.from({ length: 12 }, (_, i) => 0.4 + r() + (i === 10 ? 0.8 : 0));
      const wsum = weights.reduce((a, b) => a + b, 0);
      const byMonth = weights.map((w) => Math.round((w / wsum) * total));
      return {
        year,
        totalListenedMs: total,
        totalPlays: ranked.reduce((s, x) => s + x.plays, 0),
        distinctTracks: ranked.length,
        distinctArtists: artistMap.size,
        tracksAdded: Math.floor(10 + r() * 40),
        topTracks: ranked.slice(0, 5),
        topArtists: [...artistMap.values()].sort((a, b) => b.listenedMs - a.listenedMs).slice(0, 5),
        listenedMsByMonth: byMonth,
        busiestDay: { date: `${year}-11-${String(8 + Math.floor(r() * 18)).padStart(2, '0')}`, listenedMs: Math.round(3.5 * 60 * MIN) },
      } satisfies Wrapped;
    },
    async statsWrappedYears() { return [new Date().getFullYear(), new Date().getFullYear() - 1]; },
    async statsTaste() {
      const artists = ARTISTS.map((a) => ({ artistId: a.id, name: a.name, score: artistTracks(a.id).reduce((s, t) => s + (plays.get(t.id) ?? 0), 0) })).sort((a, b) => b.score - a.score);
      const genres = [...new Set(tracks.flatMap((t) => t.genres))].map((genre) => ({ genre, score: tracks.filter((t) => t.genres.includes(genre)).reduce((s, t) => s + (plays.get(t.id) ?? 0), 0) })).sort((a, b) => b.score - a.score);
      return { topArtists: artists.slice(0, 5), topGenres: genres.slice(0, 5), totalListenedMs: tracks.reduce((s, t) => s + (plays.get(t.id) ?? 0) * t.durationMs * 0.85, 0), skipRate: 0.12 };
    },
    async statsTrackScores(a) { // @req RF-25
      const list = a?.trackIds ? tracks.filter((t) => a.trackIds!.includes(t.id)) : tracks;
      return list.map((t, i) => ({ trackId: t.id, score: Math.round((plays.get(t.id) ?? 0) * 0.8 - i % 4 * 3), plays: plays.get(t.id) ?? 0, skips: i % 4, lastPlayedAt: Date.now() - (i + 1) * DAY }));
    },
    async statsDna({ windowDays = 30 } = {}) { // @req RF-25
      const taste = await api.statsTaste();
      const scale = windowDays / 30;
      const trends = taste.topArtists.slice(0, 5).map((a, i) => ({ kind: 'artist' as const, id: a.artistId, label: a.name, current: Math.round(a.score * scale), previous: Math.round(a.score * scale * (i % 2 ? 1.25 : 0.7)), delta: 0 }));
      trends.forEach((t) => { t.delta = t.current - t.previous; });
      return { windowDays, topArtists: taste.topArtists.map((a) => ({ ...a, score: Math.round(a.score * scale) })), topGenres: taste.topGenres.map((g) => ({ ...g, score: Math.round(g.score * scale) })), skipRate: Math.min(0.4, 0.08 + 0.01 * Math.log2(windowDays)), previousSkipRate: Math.min(0.4, 0.1 + 0.012 * Math.log2(windowDays)), listenedMs: Math.round(taste.totalListenedMs * scale / 6), previousListenedMs: Math.round(taste.totalListenedMs * scale / 7), rising: trends.filter((t) => t.delta > 0).sort((a, b) => b.delta - a.delta), falling: trends.filter((t) => t.delta < 0).sort((a, b) => a.delta - b.delta) } satisfies Dna;
    },
    async recommendationsList(a) { return recommendations(a?.playlistId, a?.limit); },
    async recommendationsRefresh(a) { await wait(400); const out = recommendations(a?.playlistId); emit('recommendations://changed', { count: out.length }); return out; },
    async recommendationsDismiss({ sourceId }) { dismissed.add(sourceId); emit('recommendations://changed', { count: recommendations().length }); },

    async profileGet() { await wait(30); return { ...profile }; },
    async profileUpdate({ profile: p }) {
      if (!p.displayName.trim()) err('invalid_input', 'O nome é obrigatório.');
      profile = { ...p };
      return { ...profile };
    },
    async profileSetAvatar({ image }) { // @req RF-21
      profile.avatarPath = image ? `data:${image.mimeType};base64,${image.dataBase64}` : null;
      return { ...profile };
    },
    async equalizerListPresets() { return presets.map(p => ({...p,gainsDb:[...p.gainsDb]})); },
    async equalizerSavePreset({preset}) {
      const name=preset.name.trim();
      if (!name || [...name].length>30 || preset.gainsDb.length!==8 || preset.gainsDb.some(v=>!Number.isFinite(v)||v < -12||v > 12) || (preset.id && presets.find(p=>p.id===preset.id)?.builtin)) err('invalid_input','Perfil inválido.');
      const existing=preset.id ? presets.find(p=>p.id===preset.id) : undefined;
      if (preset.id && !existing) err('not_found','Perfil não encontrado.');
      if (presets.some(p=>p.id!==preset.id && p.name.toLocaleLowerCase()===name.toLocaleLowerCase())) err('conflict','Nome já usado.');
      if (!preset.id && presets.length>=11) err('conflict','Limite de três perfis.');
      const saved={id:preset.id ?? `eq-${++eqSeq}`,name,gainsDb:[...preset.gainsDb],builtin:false};
      if (existing) Object.assign(existing,saved); else presets.push(saved);
      return {...saved};
    },
    async equalizerDeletePreset({presetId}) {
      const i=presets.findIndex(p=>p.id===presetId);
      if (i<0) err('not_found','Perfil não encontrado.');
      if (presets[i].builtin) err('invalid_input','Perfil pronto não pode ser apagado.');
      presets.splice(i,1);
      if(settings.eqPresetId===presetId) settings.eqPresetId='flat';
    },
    async settingsGet() { await wait(20); return { ...settings }; },
    async settingsUpdate({ settings: s }) {
      if (s.eqPresetId && !presets.some(p=>p.id===s.eqPresetId)) err('not_found','Perfil não encontrado.');
      settings = { ...settings, ...s };
      if (s.sidebarWidth !== undefined) settings.sidebarWidth = Math.max(220, Math.min(400, Math.round(s.sidebarWidth)));
      if (s.volume !== undefined) { player.volume = s.volume; pushState(); }
      return { ...settings };
    },
    async systemGpuStatus() { return { mode: settings.gpuMode, envOverride: null, hardware: true, reason: 'GPU simulada no navegador', restartRequired: false }; },
    async systemGpuStartupConfirmed() {}, // @req RNF-09

    async on(event, handler) {
      let set = listeners.get(event);
      if (!set) listeners.set(event, (set = new Set()));
      set.add(handler as (p: never) => void);
      return () => { set!.delete(handler as (p: never) => void); };
    },
    resolveAsset(path) { return path; },
  };
  return api;
}

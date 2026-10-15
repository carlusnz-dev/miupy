import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MiupyApi, PlayerState, Track } from '../ipc';

const track: Track = { id: 'one', title: 'Uma faixa', artistId: null, artistName: 'Artista', album: null, durationMs: 120000, filePath: null, coverPath: null, source: 'youtube', sourceId: 'yt1', codec: null, bitrate: null, isFavorite: false, downloadStatus: 'none', addedAt: 1, genres: [], releaseYear: null, loudnessLufs: null };
const state: PlayerState = { status: 'playing', track, positionMs: 0, durationMs: track.durationMs, volume: .8, muted: false, queue: ['one'], queueIndex: 0, streaming: true, shuffle: 'off' };
let handlers: Record<string, (value: unknown) => void>;
let fake: MiupyApi;

beforeEach(() => {
  vi.resetModules();
  handlers = {};
  fake = {
    kind: 'mock',
    on: vi.fn(async (event, handler) => { handlers[event] = handler as (value: unknown) => void; return () => { delete handlers[event]; }; }),
    playerGetState: vi.fn(async () => ({ ...state, status: 'paused' })),
    playerGetQueue: vi.fn(async () => [track]),
    playerPlay: vi.fn(async () => ({ ...state, status: 'loading' })),
    playerPause: vi.fn(async () => ({ ...state, status: 'paused' })),
    playerResume: vi.fn(async () => state),
    playerNext: vi.fn(async () => ({ ...state, queueIndex: 1 })),
    playlistList: vi.fn(async () => []),
    downloadList: vi.fn(async () => []),
    downloadClearFinished: vi.fn(async () => 1),
    recommendationsList: vi.fn(async () => [{ sourceId: 'r1', title: 'Nova', artist: 'Artista', durationMs: 1000, thumbnailUrl: null, score: .9, reasons: [{ kind: 'artist', label: 'Artista' }] }]),
    recommendationsRefresh: vi.fn(async () => []),
    recommendationsDismiss: vi.fn(async () => undefined),
    youtubeAddToLibrary: vi.fn(async () => [track]),
    libraryGetTrack: vi.fn(async () => track),
    resolveAsset: vi.fn((path) => path),
  } as unknown as MiupyApi;
  vi.doMock('../ipc', () => ({ api: fake }));
  vi.doMock('./toast.svelte', () => ({ notifyError: vi.fn() }));
});

describe('player', () => {
  // @test RF-01 RF-03 RF-18
  it('toca por streaming, carrega a fila completa e reage aos eventos', async () => {
    const { player } = await import('./player.svelte');
    await player.start();
    await player.play('one', ['one']);
    expect(player.isLoading).toBe(true);
    expect(player.state.streaming).toBe(true);
    await vi.waitFor(() => expect(player.queueTracks).toHaveLength(1));
    handlers['player://state'](state);
    handlers['player://progress']({ positionMs: 30000, durationMs: 120000 });
    expect(player.isPlaying).toBe(true);
    expect(player.progress).toBe(.25);
    await player.pause();
    expect(player.state.status).toBe('paused');
    await player.resume();
    expect(player.isPlaying).toBe(true);
    await player.next();
    expect(fake.playerNext).toHaveBeenCalledOnce();
  });
  // @test RF-18
  it('explica o erro de rede sem perder a faixa atual', async () => {
    const { notifyError } = await import('./toast.svelte');
    fake.playerPlay = vi.fn(async () => { throw { code: 'network', message: 'network' }; });
    const { player } = await import('./player.svelte');
    await player.start();
    await player.play('one');
    expect(notifyError).toHaveBeenCalledWith({ message: 'Sem conexão: baixe a faixa para ouvir offline.' });
    expect(player.track?.id).toBe('one');
  });
});

describe('player e library://changed', () => {
  // @test RNF-13
  it('só relê a faixa atual quando o motivo pode mudá-la', async () => {
    const { player } = await import('./player.svelte');
    await player.start();
    (fake.libraryGetTrack as ReturnType<typeof vi.fn>).mockClear();
    handlers['library://changed']({ reason: 'artist_photo_updated' });
    handlers['library://changed']({ reason: 'playlist_reordered' });
    await Promise.resolve();
    expect(fake.libraryGetTrack).not.toHaveBeenCalled();
    handlers['library://changed']({ reason: 'favorite' });
    await vi.waitFor(() => expect(fake.libraryGetTrack).toHaveBeenCalledTimes(1));
  });

  // @test RNF-13
  it('aplicar o mesmo estado não troca a faixa nem a fila', async () => {
    const { player } = await import('./player.svelte');
    await player.start();
    const t = player.state.track, q = player.state.queue;
    handlers['player://state']({ ...state, status: 'playing', track: { ...track }, queue: [...state.queue] });
    expect(player.state.track).toBe(t);
    expect(player.state.queue).toBe(q);
    expect(player.state.status).toBe('playing');
  });
});

describe('library', () => {
  // @test RF-05 RF-06
  it('atualiza playlists após library://changed, com intervalo mínimo entre consultas', async () => {
    vi.useFakeTimers();
    const { library } = await import('./library.svelte');
    await library.start();
    fake.playlistList = vi.fn(async () => [{ id: 'pl', name: 'Lista', description: null, coverPath: null, mosaicPaths: [], isFavorites: false, autoDownload: false, trackCount: 0, totalDurationMs: 0, createdAt: 0, updatedAt: 0 }]) as MiupyApi['playlistList'];
    handlers['library://changed']({ reason: 'playlist' });
    await vi.advanceTimersByTimeAsync(600);
    expect(library.playlists[0].id).toBe('pl');
    vi.useRealTimers();
  });
});

describe('downloads', () => {
  // @test RF-10
  it('acompanha progresso e limpa apenas os finalizados', async () => {
    const { downloads } = await import('./downloads.svelte');
    await downloads.start();
    handlers['download://progress']({ id: 'job', trackId: 'one', status: 'downloading', progress: .5, error: null });
    expect(downloads.activeCount).toBe(1);
    handlers['download://progress']({ id: 'job', trackId: 'one', status: 'done', progress: 1, error: null });
    expect(downloads.forTrack('one')?.progress).toBe(1);
    expect(await downloads.clearFinished()).toBe(1);
    expect(downloads.jobs).toHaveLength(0);
  });
});

describe('recommendations', () => {
  // @test RF-19
  it('carrega contexto da playlist, dispensa, adiciona e reage ao evento', async () => {
    const { RecommendationsStore } = await import('./recommendations.svelte');
    const store = new RecommendationsStore('pl');
    await store.start();
    expect(fake.recommendationsList).toHaveBeenCalledWith({ playlistId: 'pl' });
    await store.dismiss('r1');
    expect(store.items).toHaveLength(0);
    handlers['recommendations://changed']({ count: 1 });
    await vi.waitFor(() => expect(store.items).toHaveLength(1));
    await store.add('r1');
    expect(fake.youtubeAddToLibrary).toHaveBeenCalledWith({ sourceIds: ['r1'], playlistId: 'pl' });
    expect(store.items).toHaveLength(0);
    fake.recommendationsRefresh = vi.fn(async () => { throw { code: 'network' }; });
    await store.refresh();
    expect(store.error).toContain('Sem conexão');
    store.stop();
  });
});

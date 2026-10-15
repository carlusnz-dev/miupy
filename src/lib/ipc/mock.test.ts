import { describe, expect, it } from 'vitest';
import { createMockApi } from './mock';

// @test RF-31
describe('download automático no mock', () => {
  it('enfileira apenas o que falta, avisa uma vez e não cancela ao desligar', async () => {
    const api = createMockApi();
    const p = (await api.playlistList())[0];
    const tracks = await api.playlistGetTracks({ playlistId: p.id });
    expect(tracks.length).toBeGreaterThan(1);
    const events: { playlistId: string; count: number; reason: string }[] = [];
    const off = await api.on('download://auto', (e) => events.push(e));
    await api.playlistUpdate({ playlistId: p.id, autoDownload: true });
    expect((await api.playlistList())[0].autoDownload).toBe(true);
    expect(events).toEqual([{ playlistId: p.id, count: tracks.filter((t) => t.downloadStatus !== 'done').length, reason: 'enabled' }]);
    const before = await api.downloadList();
    expect(before.length).toBeGreaterThan(0);
    await api.playlistUpdate({ playlistId: p.id, autoDownload: false });
    expect(await api.downloadList()).toHaveLength(before.length);
    const extra = (await api.libraryListTracks()).find((t) => !tracks.some((x) => x.id === t.id) && t.downloadStatus === 'none')!;
    expect(extra).toBeTruthy();
    await api.playlistUpdate({ playlistId: p.id, autoDownload: true });
    await api.playlistAddTracks({ playlistId: p.id, trackIds: [extra.id, extra.id] });
    expect(events.at(-1)).toEqual({ playlistId: p.id, count: 1, reason: 'added' });
    expect((await api.downloadList()).filter((j) => j.trackId === extra.id)).toHaveLength(1);
    off();
  });
});

describe('favoritas no mock', () => {
  // @test RF-04 RF-30
  it('segue a playlist marcada, copia sem repetir e recria após exclusão', async () => {
    const api = createMockApi();
    const old = (await api.playlistList()).find((p) => p.isFavorites)!;
    const first = (await api.playlistGetTracks({ playlistId: old.id }))[0];
    const target = await api.playlistCreate({ name: 'Nova lista' });
    await api.playlistAddTracks({ playlistId: target.id, trackIds: [first.id] });
    await api.favoritesSetPlaylist({ playlistId: target.id });
    expect((await api.playlistList()).find((p) => p.id === target.id)?.isFavorites).toBe(true);
    expect((await api.playlistGetTracks({ playlistId: target.id })).filter((t) => t.id === first.id)).toHaveLength(1);
    expect((await api.playlistList()).find((p) => p.id === old.id)?.isFavorites).toBe(false);
    await api.playlistDelete({ playlistId: target.id });
    expect((await api.settingsGet()).favoritesPlaylistId).toBeNull();
    expect((await api.libraryGetTrack({ trackId: first.id })).isFavorite).toBe(false);
    await api.librarySetFavorite({ trackId: first.id, favorite: true });
    expect((await api.playlistList()).find((p) => p.isFavorites)?.name).toBe('Favoritas');
  });
});

// @test RF-32
describe('reprocessamento de metadados no mock', () => {
  it('publica início, andamento e fim; mantém o último lote para consulta', async () => {
    const api = createMockApi();
    const seen: Awaited<ReturnType<typeof api.metadataRefreshStatus>>[] = [];
    const un = await api.on('metadata://progress', (status) => seen.push({ ...status }));
    expect(await api.metadataRefreshStatus()).toMatchObject({ running: false, startedAt: null });
    const total = await api.metadataRefreshLibrary({ onlyMissing: true });
    expect(total).toBeGreaterThan(0);
    expect(await api.metadataRefreshStatus()).toMatchObject({ running: true, done: 0, total });
    await expect(api.metadataRefreshLibrary({ onlyMissing: false })).rejects.toMatchObject({ code: 'conflict' });
    await new Promise<void>((resolve) => {
      void api.on('metadata://progress', (status) => { if (!status.running) resolve(); });
    });
    const end = await api.metadataRefreshStatus();
    expect(end).toMatchObject({ running: false, done: total, total, failed: 0, currentTrackId: null });
    expect(end.finishedAt).toBeGreaterThanOrEqual(end.startedAt!);
    expect(seen).toHaveLength(total + 2);
    expect(seen[0]).toMatchObject({ running: true, done: 0 });
    expect(seen.at(-1)).toEqual(end);
    un();
  });
});

// @test RF-34
describe('histórico de buscas no mock', () => {
  it('grava ao buscar, sobe repetidas sem duplicar, limita a 10 e limpa', async () => {
    const api = createMockApi();
    await api.youtubeSearchHistoryClear();
    expect(await api.youtubeSearchHistory()).toEqual([]);
    await api.youtubeSearch({ query: 'Daft Punk' });
    await api.youtubeSearch({ query: 'Björk' });
    await api.youtubeSearch({ query: ' daft PUNK ' });
    expect(await api.youtubeSearchHistory()).toEqual(['daft PUNK', 'Björk']);
    await api.youtubeSearch({ query: 'bjork' });
    expect(await api.youtubeSearchHistory()).toEqual(['bjork', 'daft PUNK']);
    await api.youtubeSearch({ query: '  daft    punk ' });
    expect(await api.youtubeSearchHistory()).toEqual(['daft punk', 'bjork']);
    await api.youtubeSearch({ query: 'Björk' });
    expect(await api.youtubeSearchHistory()).toEqual(['Björk', 'daft punk']);
    await api.youtubeSearch({ query: 'daft PUNK' });
    await api.youtubeSuggest({ query: 'sugestao' });
    expect(await api.youtubeSearchHistory()).toEqual(['daft PUNK', 'Björk']);
    await Promise.all(Array.from({ length: 12 }, (_, i) => api.youtubeSearch({ query: `busca ${i}` })));
    expect(await api.youtubeSearchHistory()).toHaveLength(10);
    await api.youtubeSearchHistoryClear();
    expect(await api.youtubeSearchHistory()).toEqual([]);
  }, 20_000);
});

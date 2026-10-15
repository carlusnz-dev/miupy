import { describe, expect, it } from 'vitest';
import { createMockApi } from './mock';

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

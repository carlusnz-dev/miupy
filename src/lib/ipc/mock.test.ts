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

import { describe, expect, it } from 'vitest';
import { affectsCurrentTrack, affectsScores, tagsFor } from './libraryEvents';

describe('library://changed por motivo', () => {
  // @test RNF-11 RNF-13
  it('foto de artista não atualiza listas de faixas nem estatísticas', () => {
    const tags = tagsFor('artist_photo_updated');
    expect(tags).not.toContain('tracks');
    expect(tags).not.toContain('stats');
    expect(tags).toContain('artist');
  });

  // @test RNF-11 RNF-13
  it('fim de download atualiza faixas e estatísticas, mas não playlists', () => {
    expect(tagsFor('download_done')).toEqual(['tracks', 'stats']);
    expect(tagsFor('download_queued')).toEqual(['tracks']);
  });

  // @test RNF-11
  it('motivo desconhecido ou ausente atualiza tudo (o seguro)', () => {
    expect([...tagsFor('algo_novo')].sort()).toEqual(['artist', 'playlists', 'stats', 'tracks']);
    expect(tagsFor(undefined)).toHaveLength(4);
  });

  // @test RNF-13
  it('só favorito e exclusão recalculam a pontuação', () => {
    expect(affectsScores('favorite')).toBe(true);
    expect(affectsScores('track_deleted')).toBe(true);
    expect(affectsScores('metadata_refreshed')).toBe(false);
    expect(affectsScores('artist_photo_updated')).toBe(false);
  });

  // @test RNF-13
  it('a faixa atual só é relida quando o motivo pode mudá-la', () => {
    expect(affectsCurrentTrack('favorite')).toBe(true);
    expect(affectsCurrentTrack('metadata_refreshed')).toBe(true);
    expect(affectsCurrentTrack('playlist_reordered')).toBe(false);
    expect(affectsCurrentTrack('artist_photo_updated')).toBe(false);
  });
});

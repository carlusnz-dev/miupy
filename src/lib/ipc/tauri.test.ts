// @test RF-05
import { describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { createTauriApi } from './tauri';

// @test RF-49
describe('comandos de controle da fila', () => {
  it('encaminha nomes e argumentos ao Tauri', async () => {
    const api = createTauriApi();
    await api.playerEnqueue({ trackIds: ['a'], next: true });
    await api.playerQueueRemove({ index: 1 });
    await api.playerQueueMove({ from: 2, to: 0 });
    expect(invoke).toHaveBeenCalledWith('player_enqueue', { trackIds: ['a'], next: true });
    expect(invoke).toHaveBeenCalledWith('player_queue_remove', { index: 1 });
    expect(invoke).toHaveBeenCalledWith('player_queue_move', { from: 2, to: 0 });
  });
});

const convertFileSrc = vi.hoisted(() => vi.fn((path: string) =>
  `asset://localhost/${encodeURIComponent(path)}`));

vi.mock('@tauri-apps/api/core', () => ({ convertFileSrc, invoke: vi.fn() }));
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }));

describe('resolveAsset', () => {
  it('converte um caminho local de capa pelo protocolo do Tauri', () => {
    const path = '/home/teste/.local/share/dev.miupy.app/covers/albuns/capa 1.jpg';
    expect(createTauriApi().resolveAsset(path)).toBe(
      'asset://localhost/%2Fhome%2Fteste%2F.local%2Fshare%2Fdev.miupy.app%2Fcovers%2Falbuns%2Fcapa%201.jpg',
    );
    expect(convertFileSrc).toHaveBeenCalledWith(path);
  });

  it('preserva URLs remotas e ausência de capa', () => {
    const api = createTauriApi();
    convertFileSrc.mockClear();
    expect(api.resolveAsset('https://example.org/capa.jpg')).toBe('https://example.org/capa.jpg');
    expect(api.resolveAsset(null)).toBeNull();
    expect(convertFileSrc).not.toHaveBeenCalled();
  });
});

// @test RF-34
describe('histórico de buscas', () => {
  it('chama os comandos do contrato', async () => {
    vi.mocked(invoke).mockResolvedValue(['a']);
    const api = createTauriApi();
    expect(await api.youtubeSearchHistory()).toEqual(['a']);
    await api.youtubeSearchHistoryClear();
    expect(invoke).toHaveBeenCalledWith('youtube_search_history', undefined);
    expect(invoke).toHaveBeenCalledWith('youtube_search_history_clear', undefined);
  });
});

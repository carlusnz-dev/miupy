// @test RF-05
import { describe, expect, it, vi } from 'vitest';
import { createTauriApi } from './tauri';

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

import { describe, expect, it, vi } from 'vitest';
import { nextShuffle, shuffleLabel } from './shuffle';
import { mosaicCells } from './mosaic';
import { readImageUpload } from './imageUpload';

describe('modo aleatório', () => {
  // @test RF-24
  it('percorre os três estados e volta ao inicial', () => {
    expect(nextShuffle('off')).toBe('random');
    expect(nextShuffle('random')).toBe('surfle');
    expect(nextShuffle('surfle')).toBe('off');
    expect(shuffleLabel('surfle')).toBe('Surfle');
  });
});
describe('mosaico', () => {
  // @test RF-22
  it('usa uma capa inteira e preenche os demais arranjos sem criar caminhos', () => {
    expect(mosaicCells([])).toEqual([]);
    expect(mosaicCells(['a'])).toEqual(['a']);
    expect(mosaicCells(['a', 'b'])).toEqual(['a', 'b', 'b', 'a']);
    expect(mosaicCells(['a', 'b', 'c'])).toEqual(['a', 'b', 'c', 'a']);
    expect(mosaicCells(['a', 'b', 'c', 'd', 'e'])).toEqual(['a', 'b', 'c', 'd']);
  });
});
describe('envio de imagem', () => {
  // @test RF-21 RF-22
  it('recusa formatos inválidos e arquivo acima de 10 MB antes da leitura', async () => {
    await expect(readImageUpload(new File(['x'], 'x.gif', { type: 'image/gif' }))).rejects.toThrow('JPEG');
    const large = new File([new Uint8Array(10 * 1024 * 1024 + 1)], 'x.png', { type: 'image/png' });
    await expect(readImageUpload(large)).rejects.toThrow('10 MB');
  });
  // @test RF-21 RF-22
  it('separa os bytes base64 do cabeçalho data URL', async () => {
    const original = globalThis.FileReader;
    class Reader {
      result = 'data:image/png;base64,YWJj';
      onload: (() => void) | null = null;
      onerror: (() => void) | null = null;
      readAsDataURL() { this.onload?.(); }
    }
    vi.stubGlobal('FileReader', Reader);
    try { expect(await readImageUpload(new File(['abc'], 'a.png', { type: 'image/png' }))).toEqual({ dataBase64: 'YWJj', mimeType: 'image/png' }); }
    finally { vi.stubGlobal('FileReader', original); }
  });
});

import { describe, expect, it } from 'vitest';
import { dominantColors, meshTriple } from './palette';

const px = (r: number, g: number, b: number, n: number, a = 255) => Array.from({ length: n }, () => [r, g, b, a]).flat();

describe('dominantColors', () => {
  // @test RF-40
  it('devolve as cores mais frequentes, a mais comum primeiro', () => {
    const data = [...px(200, 30, 30, 50), ...px(30, 30, 200, 20), ...px(30, 200, 30, 5)];
    expect(dominantColors(data)).toEqual(['#c81e1e', '#1e1ec8', '#1ec81e']);
  });

  // @test RF-40
  it('ignora preto, branco, cinza e transparente', () => {
    const data = [...px(0, 0, 0, 40), ...px(255, 255, 255, 40), ...px(120, 120, 120, 40), ...px(200, 30, 30, 40, 0)];
    expect(dominantColors(data)).toEqual([]);
  });

  // @test RF-40
  it('junta tons parecidos numa só cor', () => {
    const data = [...px(200, 30, 30, 30), ...px(210, 35, 30, 30)];
    expect(dominantColors(data)).toHaveLength(1);
  });

  // @test RF-40
  it('respeita o limite pedido', () => {
    const data = [...px(200, 30, 30, 9), ...px(30, 30, 200, 8), ...px(30, 200, 30, 7)];
    expect(dominantColors(data, 2)).toHaveLength(2);
  });

  // @test RF-40
  it('limita o brilho de capas claras', () => {
    const [hex] = dominantColors(px(255, 255, 40, 20));
    const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
    expect(0.299 * r + 0.587 * g + 0.114 * b).toBeLessThanOrEqual(141);
  });
});

describe('meshTriple', () => {
  // @test RF-40
  it('completa com a cor anterior e devolve undefined sem paleta', () => {
    expect(meshTriple(['#111111'])).toEqual(['#111111', '#111111', '#111111']);
    expect(meshTriple(['#111111', '#222222'])).toEqual(['#111111', '#222222', '#222222']);
    expect(meshTriple([])).toBeUndefined();
    expect(meshTriple(null)).toBeUndefined();
  });
});

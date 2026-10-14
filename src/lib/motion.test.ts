import { describe, expect, it } from 'vitest';
import { flipTransform, registerSharedEntry, SharedSnapshot } from './motion';

describe('layout presence', () => {
  // @test RNF-10
  it('calcula deslocamento e escala inversos entre retângulos', () => {
    expect(flipTransform(
      { left: 10, top: 20, width: 100, height: 50 },
      { left: 40, top: 60, width: 200, height: 100 },
    )).toEqual({ x: -30, y: -40, scaleX: 0.5, scaleY: 0.5 });
  });

  // @test RNF-10
  it('mantém instâncias simultâneas da chave e remove o registro vazio', () => {
    const entries = new Map<string, Set<object>>();
    const first = {};
    const second = {};
    const removeFirst = registerSharedEntry(entries, 'cover-1', first);
    const removeSecond = registerSharedEntry(entries, 'cover-1', second);
    expect([...entries.get('cover-1') ?? []]).toEqual([first, second]);
    removeFirst();
    expect([...entries.get('cover-1') ?? []]).toEqual([second]);
    removeSecond();
    expect(entries.has('cover-1')).toBe(false);
  });

  // @test RNF-10
  it('substitui a captura anterior quando a navegação é interrompida', () => {
    const snapshot = new SharedSnapshot<number>();
    snapshot.replace([['cover-1', 10]]);
    snapshot.replace([['cover-2', 24]]);
    expect(snapshot.take('cover-1')).toBeUndefined();
    expect(snapshot.take('cover-2')).toBe(24);
  });
});

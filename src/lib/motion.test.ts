import { describe, expect, it } from 'vitest';
import { enterDelay, enterStyle, flipTransform, registerSharedEntry, SharedSnapshot, spring, springCollapse } from './motion';

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

describe('springCollapse', () => {
  // @test RNF-04
  it('vai da altura zero à altura medida do elemento', () => {
    const node = { getBoundingClientRect: () => ({ height: 84 }) } as unknown as Element;
    const { css } = springCollapse(node);
    expect(css?.(0, 1)).toContain('height:0px');
    expect(Number(/height:([\d.]+)px/.exec(css?.(1, 0) ?? '')?.[1])).toBeCloseTo(84, 0);
  });
});

describe('mola suave', () => {
  // @test RNF-08
  it('começa em 0, termina em 1 e passa de 1 no máximo 1%', () => {
    const values = Array.from({ length: 201 }, (_, i) => spring(i / 200));
    expect(values[0]).toBe(0);
    expect(values[200]).toBe(1);
    expect(Math.max(...values)).toBeGreaterThan(1);
    expect(Math.max(...values)).toBeLessThanOrEqual(1.01);
  });

  // @test RNF-08
  it('escalona só as primeiras linhas da lista', () => {
    expect(enterDelay(0)).toBe(0);
    expect(enterDelay(3)).toBe(60);
    expect(enterDelay(14)).toBe(0);
    expect(enterStyle(2, 40)).toBe('--enter-delay:80ms');
    expect(enterStyle(0)).toBeUndefined();
  });
});

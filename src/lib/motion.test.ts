import { describe, expect, it } from 'vitest';
import { enterDelay, enterStyle, spring, springCollapse } from './motion';

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

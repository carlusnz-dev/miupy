import { describe, expect, it } from 'vitest';
import { adaptStep, nextShown } from './chunked.svelte';

describe('renderização em blocos', () => {
  // @test RNF-13
  it('avança um bloco por vez sem passar do total', () => {
    expect(nextShown(20, 1500, 8)).toBe(28);
    expect(nextShown(1495, 1500, 8)).toBe(1500);
    expect(nextShown(0, 0, 8)).toBe(0);
  });

  // @test RNF-13
  it('encolhe o bloco quando o quadro estoura e cresce quando sobra folga', () => {
    expect(adaptStep(16, 40)).toBe(8);
    expect(adaptStep(4, 100)).toBe(4);
    expect(adaptStep(8, 16.7)).toBe(10);
    expect(adaptStep(40, 16.7)).toBe(40);
    expect(adaptStep(10, 21)).toBe(10);
  });
});

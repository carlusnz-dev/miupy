import { describe, expect, it } from 'vitest';
import { burstSchedule, meetsTarget, percentile, summarizeFrames } from './perfStats';

describe('medidor de fluidez', () => {
  // @test RNF-13
  it('calcula percentis, máximo e quadros acima de 50 ms', () => {
    const s = summarizeFrames([16.7, 16.6, 16.8, 17, 60, 16.7, 16.6, 120, 16.7, 16.7]);
    expect(s.frames).toBe(10);
    expect(s.maxMs).toBe(120);
    expect(s.over50).toBe(2);
    expect(s.p50Ms).toBe(16.7);
    expect(s.p95Ms).toBe(120);
  });

  // @test RNF-13
  it('aceita a oscilação normal do rAF a 60 Hz como quadro rápido', () => {
    const s = summarizeFrames(Array.from({ length: 100 }, (_, i) => (i % 2 ? 17.4 : 16.6)));
    expect(s.fastRatio).toBe(1);
    expect(meetsTarget(s)).toBe(true);
  });

  // @test RNF-13
  it('reprova com um quadro acima de 50 ms ou menos de 95% de quadros rápidos', () => {
    expect(meetsTarget(summarizeFrames([...Array(99).fill(16.7), 51]))).toBe(false);
    expect(meetsTarget(summarizeFrames([...Array(90).fill(16.7), ...Array(10).fill(30)]))).toBe(false);
    expect(meetsTarget(summarizeFrames([...Array(95).fill(16.7), ...Array(5).fill(30)]))).toBe(true);
  });

  // @test RNF-13
  it('trata lista vazia e percentil de lista vazia', () => {
    expect(summarizeFrames([]).frames).toBe(0);
    expect(meetsTarget(summarizeFrames([]))).toBe(false);
    expect(percentile([], 95)).toBe(0);
  });

  // @test RNF-13
  it('espalha a rajada de eventos de ponta a ponta', () => {
    expect(burstSchedule(10, 2000)).toEqual([0, 222, 444, 667, 889, 1111, 1333, 1556, 1778, 2000]);
    expect(burstSchedule(1, 2000)).toEqual([0]);
  });
});

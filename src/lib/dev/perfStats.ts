// @req RNF-13
// Estatística pura do medidor de fluidez (sem DOM), para testar e reaproveitar no roteiro.

/** Metas do RNF-13. */
export const TARGET = {
  maxFrameMs: 50,
  fastFrameMs: 16.7,
  /** O rAF em 60 Hz oscila entre 16,6 e 17,4 ms sem perder quadro; sem folga, metade dos quadros "falharia". */
  jitterMs: 1,
  fastRatio: 0.95,
} as const;

export interface FrameSummary {
  frames: number;
  meanMs: number;
  p50Ms: number;
  p95Ms: number;
  p99Ms: number;
  maxMs: number;
  /** Quadros acima de `TARGET.maxFrameMs`. */
  over50: number;
  /** Fração (0 a 1) de quadros abaixo de `TARGET.fastFrameMs + TARGET.jitterMs`. */
  fastRatio: number;
}

/** Percentil por posição mais próxima (p em 0..100) sobre uma lista já ordenada. */
export function percentile(sorted: readonly number[], p: number): number {
  if (!sorted.length) return 0;
  const i = Math.min(sorted.length - 1, Math.max(0, Math.ceil((p / 100) * sorted.length) - 1));
  return sorted[i];
}

const round = (n: number) => Math.round(n * 100) / 100;

/** Resume os intervalos entre quadros (ms). */
export function summarizeFrames(deltas: readonly number[]): FrameSummary {
  if (!deltas.length) return { frames: 0, meanMs: 0, p50Ms: 0, p95Ms: 0, p99Ms: 0, maxMs: 0, over50: 0, fastRatio: 1 };
  const sorted = [...deltas].sort((a, b) => a - b);
  const sum = deltas.reduce((s, d) => s + d, 0);
  return {
    frames: deltas.length,
    meanMs: round(sum / deltas.length),
    p50Ms: round(percentile(sorted, 50)),
    p95Ms: round(percentile(sorted, 95)),
    p99Ms: round(percentile(sorted, 99)),
    maxMs: round(sorted[sorted.length - 1]),
    over50: deltas.filter((d) => d > TARGET.maxFrameMs).length,
    fastRatio: round(deltas.filter((d) => d < TARGET.fastFrameMs + TARGET.jitterMs).length / deltas.length),
  };
}

/** Passa na meta quando nenhum quadro passa de 50 ms e 95% ou mais ficam abaixo de 16,7 ms. */
export function meetsTarget(s: FrameSummary): boolean {
  return s.frames > 0 && s.over50 === 0 && s.fastRatio >= TARGET.fastRatio;
}

/** Instantes de disparo de uma rajada de `count` eventos espalhados por `spanMs` (inclui 0 e `spanMs`). */
export function burstSchedule(count: number, spanMs: number): number[] {
  if (count <= 1) return [0];
  return Array.from({ length: count }, (_, i) => Math.round((i * spanMs) / (count - 1)));
}

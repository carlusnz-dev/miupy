// @req RNF-13
// Renderização em blocos: listas longas (milhares de faixas) montam as primeiras linhas no mesmo
// quadro da navegação e o resto aos poucos, um bloco por quadro. Montar 1500 linhas de uma vez
// passava de 500 ms de thread principal (ver docs/contribuicao/desempenho.md).

/** Altura de uma linha de faixa (capa de 44 px + padding), para o espaçador manter a barra de rolagem. */
export const ROW_HEIGHT = 56;
const MIN_STEP = 4;
const MAX_STEP = 40;

/** Quantas linhas devem estar montadas depois de mais um bloco. */
export function nextShown(shown: number, total: number, step: number): number {
  return Math.max(0, Math.min(total, shown + step));
}

/**
 * Ajusta o tamanho do bloco ao tempo do último quadro: sobra folga (quadro rápido) cresce devagar;
 * quadro lento encolhe pela metade. Mantém cada quadro perto do orçamento de 16,7 ms.
 */
export function adaptStep(step: number, frameMs: number): number {
  if (frameMs > 24) return Math.max(MIN_STEP, Math.floor(step / 2));
  if (frameMs < 19) return Math.min(MAX_STEP, step + 2);
  return step;
}

export class Chunked {
  shown = $state(0);
  private step = 8;
  private last = 0;
  constructor(initial = 20) { this.shown = initial; }

  /** Chame dentro de um `$effect` com o tamanho atual da lista: avança um bloco por quadro. */
  advance(total: number): (() => void) | undefined {
    if (this.shown >= total) return;
    const id = requestAnimationFrame((now) => {
      if (this.last && now - this.last < 100) this.step = adaptStep(this.step, now - this.last);
      this.last = now;
      this.shown = nextShown(this.shown, total, this.step);
    });
    return () => cancelAnimationFrame(id);
  }

  /** Altura (px) do que ainda não foi montado, para a lista manter o tamanho final desde o início. */
  rest(total: number): number { return Math.max(0, total - this.shown) * ROW_HEIGHT; }
}

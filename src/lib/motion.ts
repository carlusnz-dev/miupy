// @req RNF-08 RNF-09
import type { TransitionConfig } from 'svelte/transition';

const ZETA = 0.83;
const OMEGA = 8;
const OMEGA_D = OMEGA * Math.sqrt(1 - ZETA * ZETA);

/** Mola subamortecida normalizada (t de 0 a 1): um único overshoot de ~1%. Mesma curva de `--ease-spring`. */
export function spring(t: number): number {
  if (t <= 0) return 0;
  if (t >= 1) return 1;
  return 1 - Math.exp(-ZETA * OMEGA * t) * (Math.cos(OMEGA_D * t) + ((ZETA * OMEGA) / OMEGA_D) * Math.sin(OMEGA_D * t));
}

/** Atraso de escalonamento de uma entrada: só as `limit` primeiras linhas atrasam, as demais entram juntas. */
export function enterDelay(index: number, step = 20, limit = 14): number {
  return index < limit ? index * step : 0;
}

/** Uso: `<li class="enter" style={enterStyle(i)}>`. */
export function enterStyle(index: number, step?: number, limit?: number): string | undefined {
  const ms = enterDelay(index, step, limit);
  return ms ? `--enter-delay:${ms}ms` : undefined;
}

/** Entrada/saída de páginas, toasts e menus: fade + deslocamento em Y na mola suave. */
export function springTransition(_node: Element, { y = 8, delay = 0, duration = 360 }: { y?: number; delay?: number; duration?: number } = {}): TransitionConfig {
  const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return {
    delay: reduced ? 0 : delay,
    duration: reduced ? 90 : duration,
    css: (t) => {
      if (reduced) return `opacity:${t}`;
      return `opacity:${Math.min(1, t * 1.6)};transform:translate3d(0,${(1 - spring(t)) * y}px,0)`;
    },
  };
}

/** Recolhe e libera a altura do elemento com a mesma mola, sem salto de layout. */
export function springCollapse(node: Element, { duration = 360 }: { duration?: number } = {}): TransitionConfig {
  const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const height = node.getBoundingClientRect().height;
  return {
    duration: reduced ? 90 : duration,
    css: (t) => {
      if (reduced) return `opacity:${t}`;
      return `height:${height * Math.min(1, spring(t))}px;overflow:hidden;opacity:${Math.min(1, t * 1.6)}`;
    },
  };
}

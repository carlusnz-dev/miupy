// @req RNF-08 RNF-09
import type { TransitionConfig } from 'svelte/transition';

/** Curva de mola amortecida usada por páginas, listas e toasts. */
export function springTransition(_node: Element, { y = 10, delay = 0, duration = 250 }: { y?: number; delay?: number; duration?: number } = {}): TransitionConfig {
  const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  return {
    delay: reduced ? 0 : delay,
    duration: reduced ? 90 : duration,
    css: (t) => {
      if (reduced) return `opacity:${t}`;
      const spring = 1 - Math.exp(-7 * t) * Math.cos(11 * t);
      return `opacity:${Math.min(1, t * 1.6)};transform:translate3d(0,${(1 - spring) * y}px,0)`;
    },
  };
}

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

/** Recolhe e libera a altura do elemento com a mesma mola, sem salto de layout. */
export function springCollapse(node: Element, { duration = 250 }: { duration?: number } = {}): TransitionConfig {
  const reduced = typeof window !== 'undefined' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  const height = node.getBoundingClientRect().height;
  return {
    duration: reduced ? 90 : duration,
    css: (t) => {
      if (reduced) return `opacity:${t}`;
      const spring = Math.max(0, 1 - Math.exp(-7 * t) * Math.cos(11 * t));
      return `height:${height * Math.min(1, spring)}px;overflow:hidden;opacity:${Math.min(1, t * 1.6)}`;
    },
  };
}

export type SharedRect = { left: number; top: number; width: number; height: number };

// @req RNF-10
export function flipTransform(from: SharedRect, to: SharedRect) {
  return {
    x: from.left - to.left,
    y: from.top - to.top,
    scaleX: to.width ? from.width / to.width : 1,
    scaleY: to.height ? from.height / to.height : 1,
  };
}

// @req RNF-10
export function registerSharedEntry<T>(entries: Map<string, Set<T>>, key: string, value: T) {
  const group = entries.get(key) ?? new Set<T>();
  group.add(value);
  entries.set(key, group);
  return () => {
    group.delete(value);
    if (group.size === 0) entries.delete(key);
  };
}

// @req RNF-10
export class SharedSnapshot<T> {
  private values = new Map<string, T>();
  replace(values: Iterable<readonly [string, T]>) { this.values = new Map(values); }
  take(key: string) {
    const value = this.values.get(key);
    this.values.delete(key);
    return value;
  }
}

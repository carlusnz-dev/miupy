// @req RNF-13
// Comparação e fusão de listas: uma resposta nova só troca o que mudou, e o que não mudou mantém a
// mesma referência. Assim o Svelte não reavalia nem repinta linhas que continuam iguais.

/** Igualdade estrutural para dados vindos do IPC (JSON puro: objetos, listas e primitivos). */
export function deepEqual(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== 'object' || typeof b !== 'object' || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) if (!deepEqual(a[i], b[i])) return false;
    return true;
  }
  const ka = Object.keys(a), kb = Object.keys(b);
  if (ka.length !== kb.length) return false;
  for (const k of ka) {
    if (!Object.prototype.hasOwnProperty.call(b, k)) return false;
    if (!deepEqual((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k])) return false;
  }
  return true;
}

/**
 * Devolve `prev` se `next` for igual (mesma ordem e mesmo conteúdo). Senão devolve `next` reaproveitando,
 * por chave, a referência do item anterior quando o item não mudou.
 */
export function reconcileList<T>(prev: T[] | undefined, next: T[], key: (item: T) => string): T[] {
  if (!prev) return next;
  const old = new Map<string, T>();
  for (const item of prev) old.set(key(item), item);
  let same = prev.length === next.length;
  const merged = next.map((item, i) => {
    const before = old.get(key(item));
    const keep = before !== undefined && deepEqual(before, item) ? before : item;
    if (same && prev[i] !== keep) same = false;
    return keep;
  });
  return same ? prev : merged;
}

/** `merge` para `Resource` quando a resposta é uma lista com chave (ex.: faixas por `id`). */
export function mergeList<T>(key: (item: T) => string) {
  return (prev: T[] | undefined, next: T[]) => reconcileList(prev, next, key);
}

/** `merge` genérico: mantém a referência anterior se nada mudou. */
export function mergeDeep<T>(prev: T | undefined, next: T): T {
  return prev !== undefined && deepEqual(prev, next) ? prev : next;
}

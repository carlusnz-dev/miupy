import { describe, expect, it } from 'vitest';
import { deepEqual, mergeDeep, reconcileList } from './diff';

type Row = { id: string; title: string; genres: string[] };
const a: Row = { id: 'a', title: 'A', genres: ['x'] };
const b: Row = { id: 'b', title: 'B', genres: [] };
const key = (r: Row) => r.id;
const clone = (r: Row): Row => ({ ...r, genres: [...r.genres] });

describe('diff de listas', () => {
  // @test RNF-13
  it('deepEqual compara objetos, listas e primitivos', () => {
    expect(deepEqual(a, clone(a))).toBe(true);
    expect(deepEqual(a, { ...a, genres: ['y'] })).toBe(false);
    expect(deepEqual([1, 2], [1, 2, 3])).toBe(false);
    expect(deepEqual(null, {})).toBe(false);
    expect(deepEqual({ x: undefined }, { y: undefined })).toBe(false);
  });

  // @test RNF-13
  it('devolve a lista anterior quando a nova é igual (nada a repintar)', () => {
    const prev = [a, b];
    expect(reconcileList(prev, [clone(a), clone(b)], key)).toBe(prev);
  });

  // @test RNF-13
  it('troca só o item que mudou e mantém a referência dos outros', () => {
    const prev = [a, b];
    const next = [clone(a), { ...b, title: 'B2' }];
    const out = reconcileList(prev, next, key);
    expect(out).not.toBe(prev);
    expect(out[0]).toBe(a);
    expect(out[1]).toBe(next[1]);
  });

  // @test RNF-13
  it('acompanha entrada, saída e reordenação por chave', () => {
    const c: Row = { id: 'c', title: 'C', genres: [] };
    const out = reconcileList([a, b], [clone(b), clone(c), clone(a)], key);
    expect(out.map(key)).toEqual(['b', 'c', 'a']);
    expect(out[0]).toBe(b);
    expect(out[2]).toBe(a);
    expect(reconcileList(undefined, [a], key)).toEqual([a]);
  });

  // @test RNF-13
  it('mergeDeep mantém a referência quando nada mudou', () => {
    const prev = { n: 1, list: [1, 2] };
    expect(mergeDeep(prev, { n: 1, list: [1, 2] })).toBe(prev);
    const next = { n: 2, list: [1, 2] };
    expect(mergeDeep(prev, next)).toBe(next);
    expect(mergeDeep(undefined, next)).toBe(next);
  });
});

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mergeList } from '../util/diff';
import { toasts } from './toast.svelte';
import { clearCache, invalidate, MIN_INTERVAL_MS, Resource } from './resource.svelte';

type Row = { id: string; n: number };
const rows = (...ns: number[]): Row[] => ns.map((n, i) => ({ id: `r${i}`, n }));

beforeEach(() => { clearCache(); toasts.items = []; vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); });

describe('cache entre telas', () => {
  // @test RNF-11
  it('sem cache fica carregando; depois da resposta mostra os dados', async () => {
    const r = new Resource<Row[]>(['tracks']);
    expect(r.loading).toBe(true);
    await r.load('k', async () => rows(1, 2));
    expect(r.loading).toBe(false);
    expect(r.data).toHaveLength(2);
  });

  // @test RNF-11
  it('uma tela nova mostra o cache na hora e atualiza em segundo plano', async () => {
    const first = new Resource<Row[]>(['tracks']);
    await first.load('k', async () => rows(1, 2));

    const second = new Resource<Row[]>(['tracks']);
    second.prime('k');
    expect(second.loading).toBe(false); // sem esqueleto
    expect(second.data).toEqual(rows(1, 2));

    let release!: (v: Row[]) => void;
    const pending = second.load('k', () => new Promise<Row[]>((res) => { release = res; }));
    expect(second.refreshing).toBe(true);
    expect(second.data).toEqual(rows(1, 2)); // ainda o cache enquanto a rede responde
    release(rows(1, 3));
    await pending;
    expect(second.refreshing).toBe(false);
    expect(second.data).toEqual(rows(1, 3));
  });

  // @test RNF-13
  it('resposta igual não troca a referência (nada a repintar); mudança troca só o item', async () => {
    const r = new Resource<Row[]>(['tracks'], mergeList((x: Row) => x.id));
    await r.load('k', async () => rows(1, 2));
    const before = r.data!;
    await r.load('k', async () => rows(1, 2));
    expect(r.data).toBe(before);
    await r.load('k', async () => rows(1, 9));
    expect(r.data).not.toBe(before);
    expect(r.data![0]).toBe(before[0]);
    expect(r.data![1].n).toBe(9);
  });

  // @test RNF-11
  it('erro com dados em cache mantém a tela; erro sem dados vira erro', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('k', async () => { throw { message: 'falhou' }; });
    expect(r.loading).toBe(false);
    expect(r.error).toBe('falhou');
    expect(r.data).toBeUndefined();
    await r.load('k', async () => rows(1));
    expect(r.error).toBeNull();
    await r.load('k', async () => { throw { message: 'de novo' }; });
    expect(r.data).toEqual(rows(1)); // continua mostrando o último bom
  });

  // @test RNF-11
  it('descarta a resposta de uma consulta que já foi trocada por outra', async () => {
    const r = new Resource<Row[]>(['tracks']);
    let slow!: (v: Row[]) => void;
    const old = r.load('a', () => new Promise<Row[]>((res) => { slow = res; }));
    await r.load('b', async () => rows(7));
    slow(rows(1));
    await old;
    expect(r.data).toEqual(rows(7));
    expect(r.dataKey).toBe('b');
  });
});

describe('troca de consulta', () => {
  // @test RNF-11
  it('sem cache para a nova chave volta ao esqueleto em vez de mostrar a lista antiga', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('a', async () => rows(1, 2));
    expect(r.dataKey).toBe('a');

    let release!: (v: Row[]) => void;
    const pending = r.load('b', () => new Promise<Row[]>((res) => { release = res; }));
    expect(r.loading).toBe(true);
    expect(r.data).toBeUndefined();
    expect(r.dataKey).toBe('');
    release(rows(5));
    await pending;
    expect(r.loading).toBe(false);
    expect(r.dataKey).toBe('b');
    expect(r.data).toEqual(rows(5));
  });

  // @test RNF-11
  it('com cache para a nova chave mostra o cache na hora', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('a', async () => rows(1));
    await r.load('b', async () => rows(2));
    const pending = r.load('a', () => new Promise<Row[]>(() => {}));
    expect(r.loading).toBe(false);
    expect(r.dataKey).toBe('a');
    expect(r.data).toEqual(rows(1));
    void pending;
  });

  // @test RNF-11
  it('o erro da consulta anterior não vaza para a nova', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('a', async () => { throw { message: 'falhou' }; });
    expect(r.error).toBe('falhou');
    void r.load('b', () => new Promise<Row[]>(() => {}));
    expect(r.error).toBeNull();
    expect(r.loading).toBe(true);
  });
});

describe('falha ao atualizar com cache', () => {
  // @test RNF-11
  it('avisa uma vez, mantém os dados e some o aviso quando volta a funcionar', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('k', async () => rows(1));
    expect(toasts.items).toHaveLength(0);

    await r.load('k', async () => { throw { message: 'sem rede' }; });
    expect(r.data).toEqual(rows(1));
    expect(r.stale).toBe(true);
    expect(toasts.items).toHaveLength(1);
    expect(toasts.items[0].text).toMatch(/atualizar/);

    await r.load('k', async () => { throw { message: 'sem rede' }; });
    expect(toasts.items).toHaveLength(1); // não repete a cada tentativa

    await r.load('k', async () => rows(2));
    expect(r.stale).toBe(false);
    await r.load('k', async () => { throw { message: 'sem rede' }; });
    expect(toasts.items).toHaveLength(2);
  });

  // @test RNF-11
  it('sem dados em cache o erro continua sendo a tela de erro, sem aviso', async () => {
    const r = new Resource<Row[]>(['tracks']);
    await r.load('k', async () => { throw { message: 'falhou' }; });
    expect(r.error).toBe('falhou');
    expect(r.stale).toBe(false);
    expect(toasts.items).toHaveLength(0);
  });
});

describe('invalidação por evento', () => {
  // @test RNF-11 RNF-13
  it('atualiza só os recursos abertos que dependem do grupo do evento', async () => {
    const tracks = new Resource<Row[]>(['tracks']);
    const artist = new Resource<Row[]>(['artist']);
    const fetchTracks = vi.fn(async () => rows(1));
    const fetchArtist = vi.fn(async () => rows(2));
    await tracks.load('t', fetchTracks);
    await artist.load('a', fetchArtist);
    const off = [tracks.attach(), artist.attach()];

    invalidate(['tracks']);
    await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS + 10);
    expect(fetchTracks).toHaveBeenCalledTimes(2);
    expect(fetchArtist).toHaveBeenCalledTimes(1);
    off.forEach((f) => f());
  });

  // @test RNF-13
  it('uma rajada de 10 eventos em 2 s vira poucas consultas, não 10', async () => {
    const r = new Resource<Row[]>(['tracks']);
    const fetcher = vi.fn(async () => rows(1));
    await r.load('t', fetcher);
    const off = r.attach();
    for (let i = 0; i < 10; i++) {
      invalidate(['tracks']);
      await vi.advanceTimersByTimeAsync(200);
    }
    await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS + 10);
    const calls = fetcher.mock.calls.length - 1;
    expect(calls).toBeGreaterThanOrEqual(1);
    expect(calls).toBeLessThanOrEqual(5);
    off();
  });

  // @test RNF-11
  it('tela fechada (detach) não consulta mais', async () => {
    const r = new Resource<Row[]>(['tracks']);
    const fetcher = vi.fn(async () => rows(1));
    await r.load('t', fetcher);
    r.attach()();
    invalidate(['tracks']);
    await vi.advanceTimersByTimeAsync(MIN_INTERVAL_MS + 10);
    expect(fetcher).toHaveBeenCalledTimes(1);
  });
});

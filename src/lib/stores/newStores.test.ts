import { beforeEach, describe, expect, it, vi } from 'vitest';
let suggest: ReturnType<typeof vi.fn>;
let dnaCall: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.resetModules(); vi.useFakeTimers();
  suggest = vi.fn(); dnaCall = vi.fn();
  vi.doMock('../ipc', () => ({ api: { youtubeSuggest: suggest, statsDna: dnaCall } }));
});

describe('sugestões', () => {
  // @test RF-23
  it('espera 200 ms e usa só a última consulta', async () => {
    const { SuggestionsStore } = await import('./suggestions.svelte');
    const store = new SuggestionsStore();
    store.update('ma');
    await vi.advanceTimersByTimeAsync(150);
    store.update('mag');
    await vi.advanceTimersByTimeAsync(199);
    expect(suggest).not.toHaveBeenCalled();
    suggest.mockResolvedValue(['Magma']);
    await vi.advanceTimersByTimeAsync(1);
    expect(suggest).toHaveBeenCalledExactlyOnceWith({ query: 'mag' });
    expect(store.items).toEqual(['Magma']);
    store.dispose(); vi.useRealTimers();
  });
  // @test RF-23
  it('descarta resposta atrasada e erro da consulta antiga', async () => {
    const { SuggestionsStore } = await import('./suggestions.svelte');
    const store = new SuggestionsStore();
    let oldResolve!: (items: string[]) => void;
    suggest.mockImplementationOnce(() => new Promise<string[]>((resolve) => { oldResolve = resolve; })).mockResolvedValueOnce(['nova']);
    store.update('velha'); await vi.advanceTimersByTimeAsync(200);
    store.update('nova'); await vi.advanceTimersByTimeAsync(200);
    oldResolve(['velha']); await Promise.resolve();
    expect(store.items).toEqual(['nova']);
    store.dispose(); vi.useRealTimers();
  });
});
describe('DNA', () => {
  // @test RF-25
  it('troca a janela e preserva a resposta mais recente', async () => {
    const { DnaStore } = await import('./dna.svelte');
    const store = new DnaStore();
    let oldResolve!: (data: { windowDays: number }) => void;
    dnaCall.mockImplementationOnce(() => new Promise((resolve) => { oldResolve = resolve; })).mockResolvedValueOnce({ windowDays: 90 });
    const first = store.load(7); const second = store.load(90);
    await second; oldResolve({ windowDays: 7 }); await first;
    expect(dnaCall).toHaveBeenNthCalledWith(2, { windowDays: 90 });
    expect(store.windowDays).toBe(90);
    expect(store.data?.windowDays).toBe(90);
    vi.useRealTimers();
  });
});

describe('pontuação', () => {
  // @test RF-25
  it('junta os pedidos de várias linhas numa chamada e pede de novo depois de invalidar', async () => {
    vi.useRealTimers();
    const scoresCall = vi.fn(async ({ trackIds }: { trackIds: string[] }) => trackIds.map((trackId) => ({ trackId, score: 1.25, plays: 1, skips: 0, lastPlayedAt: null })));
    vi.doMock('../ipc', () => ({ api: { statsTrackScores: scoresCall } }));
    const { scores, fmtScore } = await import('./scores.svelte');
    await Promise.all([scores.load(['a']), scores.load(['b']), scores.load(['a', 'c'])]);
    expect(scoresCall).toHaveBeenCalledTimes(1);
    expect(scoresCall.mock.calls[0][0].trackIds.sort()).toEqual(['a', 'b', 'c']);
    await scores.load(['a']);
    expect(scoresCall).toHaveBeenCalledTimes(1);
    scores.invalidate();
    await scores.load(['a']);
    expect(scoresCall).toHaveBeenCalledTimes(2);
    expect(fmtScore(scores.get('a').score)).toBe('1,3');
  });
});

describe('sugestões depois de escolher', () => {
  // @test RF-23
  it('não reabre a lista para o texto escolhido nem dispara a consulta pendente', async () => {
    const { SuggestionsStore } = await import('./suggestions.svelte');
    const store = new SuggestionsStore();
    suggest.mockResolvedValue(['magma']);
    store.update('mag');
    store.close('magma');
    store.update('magma');
    await vi.advanceTimersByTimeAsync(400);
    expect(suggest).not.toHaveBeenCalled();
    expect(store.open).toBe(false);
    store.update('magma b');
    await vi.advanceTimersByTimeAsync(200);
    expect(suggest).toHaveBeenCalledExactlyOnceWith({ query: 'magma b' });
    store.dispose(); vi.useRealTimers();
  });
});

describe('tooltip da pontuação', () => {
  // @test RF-25
  it('flexiona play e pulo', async () => {
    const { scoreTooltip } = await import('./scores.svelte');
    expect(scoreTooltip({ trackId: 'a', score: 1, plays: 1, skips: 1, lastPlayedAt: null })).toBe('Pontuação: 1 play, 1 pulo, última vez: nunca');
    expect(scoreTooltip({ trackId: 'a', score: 1, plays: 2, skips: 0, lastPlayedAt: null })).toBe('Pontuação: 2 plays, 0 pulos, última vez: nunca');
    vi.useRealTimers();
  });
});

describe('histórico de buscas', () => {
  // @test RF-34
  it('carrega as buscas recentes, mostra erro e limpa', async () => {
    const history = vi.fn(); const clear = vi.fn().mockResolvedValue(undefined);
    vi.doMock('../ipc', () => ({ api: { youtubeSearchHistory: history, youtubeSearchHistoryClear: clear } }));
    const { SearchHistoryStore } = await import('./searchHistory.svelte');
    const store = new SearchHistoryStore();
    expect(store.ready).toBe(false);
    history.mockResolvedValueOnce(['Daft Punk', 'Björk']);
    await store.load();
    expect(store.ready).toBe(true);
    expect(store.items).toEqual(['Daft Punk', 'Björk']);
    history.mockRejectedValueOnce({ message: 'falhou' });
    await store.load();
    expect(store.error).toBe('falhou');
    expect(store.items).toEqual(['Daft Punk', 'Björk']);
    await store.clear();
    expect(clear).toHaveBeenCalledOnce();
    expect(store.items).toEqual([]);
    expect(store.error).toBeNull();
  });
  // @test RF-34
  it('descarta resposta atrasada depois de limpar', async () => {
    let resolveOld!: (v: string[]) => void;
    const history = vi.fn(() => new Promise<string[]>((r) => { resolveOld = r; }));
    vi.doMock('../ipc', () => ({ api: { youtubeSearchHistory: history, youtubeSearchHistoryClear: vi.fn().mockResolvedValue(undefined) } }));
    const { SearchHistoryStore } = await import('./searchHistory.svelte');
    const store = new SearchHistoryStore();
    const loading = store.load();
    await store.clear();
    resolveOld(['velha']);
    await loading;
    expect(store.items).toEqual([]);
  });
});

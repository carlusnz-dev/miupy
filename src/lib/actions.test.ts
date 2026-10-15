import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Track } from './ipc';

const setFavorite = vi.fn();
vi.mock('./ipc', () => ({ api: { librarySetFavorite: (a: unknown) => setFavorite(a) } }));

const track = (isFavorite: boolean) => ({ id: 't1', isFavorite }) as Track;

beforeEach(() => { vi.useFakeTimers(); setFavorite.mockReset(); });

describe('favoritar com atualização otimista', () => {
  // @test RF-04
  it('mostra o coração na hora, antes de o comando responder, e segura até a lista chegar', async () => {
    const { toggleFavorite } = await import('./actions');
    const { favorites, CONFIRM_HOLD_MS } = await import('./stores/favorites.svelte');
    let done!: () => void;
    setFavorite.mockReturnValue(new Promise<void>((res) => { done = res; }));
    const t = track(false);

    const call = toggleFavorite(t);
    expect(favorites.is(t)).toBe(true); // sem esperar o library://changed
    expect(setFavorite).toHaveBeenCalledWith({ trackId: 't1', favorite: true });
    done();
    await call;
    expect(favorites.is(t)).toBe(true); // continua até a lista nova trazer o valor
    await vi.advanceTimersByTimeAsync(CONFIRM_HOLD_MS + 1);
    expect(favorites.is(track(true))).toBe(true);
    expect(favorites.is(t)).toBe(false); // sem sobreposição, vale o que o backend mandou
  });

  // @test RF-04
  it('se o comando falha, volta ao estado anterior e mostra um erro', async () => {
    const { toggleFavorite } = await import('./actions');
    const { favorites } = await import('./stores/favorites.svelte');
    const { toasts } = await import('./stores/toast.svelte');
    toasts.items = [];
    setFavorite.mockRejectedValue({ message: 'sem espaço' });
    const t = track(true);

    const call = toggleFavorite(t);
    expect(favorites.is(t)).toBe(false); // otimista: desfavoritou na hora
    await call;
    expect(favorites.is(t)).toBe(true); // reverteu
    expect(toasts.items.at(-1)).toMatchObject({ kind: 'error', text: 'sem espaço' });
  });

  // @test RF-04
  it('dois toques seguidos: o segundo parte do valor mostrado e a falha do primeiro não o desfaz', async () => {
    const { toggleFavorite } = await import('./actions');
    const { favorites } = await import('./stores/favorites.svelte');
    let fail!: (e: unknown) => void;
    setFavorite.mockReturnValueOnce(new Promise<void>((_, rej) => { fail = rej; })).mockResolvedValueOnce(undefined);
    const t = track(false);

    const first = toggleFavorite(t);
    const second = toggleFavorite(t);
    expect(setFavorite).toHaveBeenLastCalledWith({ trackId: 't1', favorite: false });
    fail({ message: 'falhou' });
    await Promise.all([first, second]);
    expect(favorites.is(t)).toBe(false); // vale o toque mais novo
  });
});

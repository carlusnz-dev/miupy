import { beforeEach, describe, expect, it, vi } from 'vitest';

let entries: { hash: string; state: Record<string, unknown> }[];
let position: number;
let onHashChange: () => void;
let back: ReturnType<typeof vi.fn>;

function visit(path: string) {
  entries.splice(position + 1);
  entries.push({ hash: `#${path}`, state: {} });
  position++;
  onHashChange();
}

beforeEach(() => {
  vi.resetModules();
  position = 0;
  entries = [{ hash: '', state: {} }];
  vi.stubGlobal('location', {
    get hash() { return entries[position].hash; },
    set hash(value: string) { visit(value); },
  });
  back = vi.fn(() => { position--; onHashChange(); });
  vi.stubGlobal('history', {
    get state() { return entries[position].state; },
    replaceState(state: Record<string, unknown>) { entries[position].state = state; },
    back,
  });
  vi.stubGlobal('window', { addEventListener: (_event: string, callback: () => void) => { onHashChange = callback; } });
});

describe('voltar à tela anterior', () => {
  // @test RF-43
  it('mostra a seta só para links e restaura a origem ao voltar e avançar', async () => {
    const { router, navigate, paths } = await import('./router.svelte');
    expect(router.canGoBack).toBe(false);
    expect(router.fromLink).toBe(false);
    navigate(paths.artist('a'));
    expect(router.current.name).toBe('artist');
    expect(router.fromLink).toBe(true);
    expect(router.canGoBack).toBe(true);
    router.menuNavigate(paths.playlists);
    expect(router.fromLink).toBe(false);
    router.back();
    expect(back).toHaveBeenCalledOnce();
    expect(router.current.name).toBe('artist');
    expect(router.fromLink).toBe(true);
    position++;
    onHashChange();
    expect(router.current.name).toBe('playlists');
    expect(router.fromLink).toBe(false);
  });

  // @test RF-43
  it('não volta além da abertura e esconde a seta ao escolher a mesma tela no menu', async () => {
    const { router, navigate, paths } = await import('./router.svelte');
    router.back();
    expect(back).not.toHaveBeenCalled();
    navigate(paths.library);
    router.menuNavigate(paths.library);
    expect(router.fromLink).toBe(false);
    router.back();
    expect(router.current.name).toBe('home');
    expect(router.canGoBack).toBe(false);
    router.back();
    expect(back).toHaveBeenCalledOnce();
  });

  // @test RF-43
  it('esconde a seta ao voltar de um link para uma tela aberta pelo menu', async () => {
    const { router, navigate, paths } = await import('./router.svelte');
    router.menuNavigate(paths.library);
    navigate(paths.artist('a'));
    expect(router.fromLink).toBe(true);
    router.back();
    expect(router.current.name).toBe('library');
    expect(router.fromLink).toBe(false);
    expect(router.canGoBack).toBe(true);
  });
});

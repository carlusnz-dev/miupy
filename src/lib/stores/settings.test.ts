import { beforeEach, describe, expect, it, vi } from 'vitest';

const update = vi.fn(async ({ settings }: { settings: { sidebarWidth: number } }) => ({ sidebarWidth: settings.sidebarWidth }));
vi.mock('../ipc', () => ({ api: { settingsUpdate: update } }));

beforeEach(() => {
  update.mockClear();
  vi.stubGlobal('document', { documentElement: { style: { setProperty: vi.fn() } } });
  vi.stubGlobal('localStorage', { setItem: vi.fn() });
});

describe('configurações do menu', () => {
  // @test RF-33
  it('só envia settings_update no salvamento após a pré-visualização', async () => {
    const { settings } = await import('./settings.svelte');
    document.documentElement.style.setProperty('--sidebar-w', '340px');
    expect(update).not.toHaveBeenCalled();
    await settings.saveSidebarWidth(340);
    expect(update).toHaveBeenCalledOnce();
    expect(update).toHaveBeenCalledWith({ settings: { sidebarWidth: 340 } });
    expect(settings.sidebarWidth).toBe(340);
  });
});

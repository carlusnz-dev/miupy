// @test RF-06
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Track } from './ipc';

const del = vi.fn(async () => {});
vi.mock('./ipc', () => ({ api: { libraryDeleteTrack: del } }));

const t = (id: string, filePath: string | null) => ({ id, title: id, filePath }) as Track;

beforeEach(() => { del.mockClear(); vi.resetModules(); });

async function run(tracks: Track[], answer: boolean, checked = false) {
  const { removeTracks } = await import('./actions');
  const { ui } = await import('./stores/ui.svelte');
  const p = removeTracks(tracks);
  await Promise.resolve();
  if (ui.confirmState) ui.confirmState.checked = checked;
  ui.answer(answer);
  return { done: await p, option: ui.confirmState };
}

describe('removeTracks', () => {
  it('cancelar não remove nada', async () => {
    expect((await run([t('a', null)], false)).done).toBe(false);
    expect(del).not.toHaveBeenCalled();
  });
  it('remove cada faixa e só apaga o arquivo se a caixa estiver marcada', async () => {
    expect((await run([t('a', '/a.opus'), t('b', null)], true, true)).done).toBe(true);
    expect(del).toHaveBeenCalledWith({ trackId: 'a', deleteFile: true });
    expect(del).toHaveBeenCalledWith({ trackId: 'b', deleteFile: true });
  });
  it('sem arquivo baixado não oferece a caixa e não apaga arquivo', async () => {
    await run([t('a', null)], true);
    expect(del).toHaveBeenCalledWith({ trackId: 'a', deleteFile: false });
  });
});

// @test RF-32
import { describe, expect, it } from 'vitest';
import { createMockApi } from './mock';

describe('reprocessamento de metadados no mock', () => {
  it('publica início, andamento e fim; mantém o último lote para consulta', async () => {
    const api = createMockApi();
    const seen: Awaited<ReturnType<typeof api.metadataRefreshStatus>>[] = [];
    const un = await api.on('metadata://progress', (status) => seen.push({ ...status }));
    expect(await api.metadataRefreshStatus()).toMatchObject({ running: false, startedAt: null });
    const total = await api.metadataRefreshLibrary({ onlyMissing: true });
    expect(total).toBeGreaterThan(0);
    expect(await api.metadataRefreshStatus()).toMatchObject({ running: true, done: 0, total });
    await expect(api.metadataRefreshLibrary({ onlyMissing: false })).rejects.toMatchObject({ code: 'conflict' });
    await new Promise<void>((resolve) => {
      void api.on('metadata://progress', (status) => { if (!status.running) resolve(); });
    });
    const end = await api.metadataRefreshStatus();
    expect(end).toMatchObject({ running: false, done: total, total, failed: 0, currentTrackId: null });
    expect(end.finishedAt).toBeGreaterThanOrEqual(end.startedAt!);
    expect(seen).toHaveLength(total + 2);
    expect(seen[0]).toMatchObject({ running: true, done: 0 });
    expect(seen.at(-1)).toEqual(end);
    un();
  });
});

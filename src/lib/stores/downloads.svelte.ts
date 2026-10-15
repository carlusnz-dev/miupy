// @req RF-10 RF-11 RNF-13
import { SvelteMap } from 'svelte/reactivity';
import { api, type DownloadJob } from '../ipc';
import { notify } from './toast.svelte';

class DownloadsStore {
  jobs = $state<DownloadJob[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);
  private started = false;
  /** Job mais recente de cada faixa. Cada linha de faixa lê só a própria entrada (antes varria todos os jobs). */
  private readonly latest = new SvelteMap<string, DownloadJob>();

  get active() { return this.jobs.filter((j) => j.status === 'queued' || j.status === 'downloading'); }
  get activeCount() { return this.active.length; }

  async start() {
    if (this.started) return;
    this.started = true;
    await api.on('download://progress', (job) => {
      const i = this.jobs.findIndex((j) => j.id === job.id);
      if (i >= 0) this.jobs[i] = job; else this.jobs.push(job);
      this.syncLatest(job.trackId);
    });
    // @req RF-31
    await api.on('download://auto', ({ count, reason }) => {
      if (!count) return;
      if (reason === 'offline') notify(`Sem rede: ${count} ${count === 1 ? 'download esperando' : 'downloads esperando'}`);
      else notify(`${count} ${count === 1 ? 'faixa na fila de download' : 'faixas na fila de download'}`);
    });
    await this.reload();
  }

  async reload() {
    try { this.jobs = await api.downloadList(); this.error = null; this.rebuildLatest(); }
    catch (e) { this.error = (e as { message?: string })?.message ?? 'Falha ao carregar downloads.'; }
    finally { this.loading = false; }
  }

  async clearFinished() { // @req RF-10
    const count = await api.downloadClearFinished();
    this.jobs = this.jobs.filter((j) => j.status !== 'done' && (j.status !== 'error' || j.error?.startsWith('auto_offline:')));
    this.rebuildLatest();
    return count;
  }

  private syncLatest(trackId: string) {
    for (let i = this.jobs.length - 1; i >= 0; i--) if (this.jobs[i].trackId === trackId) { this.latest.set(trackId, this.jobs[i]); return; }
    this.latest.delete(trackId);
  }
  private rebuildLatest() {
    const last = new Map<string, DownloadJob>();
    for (const j of this.jobs) last.set(j.trackId, j);
    for (const id of [...this.latest.keys()]) if (!last.has(id)) this.latest.delete(id);
    for (const [id, j] of last) if (this.latest.get(id) !== j) this.latest.set(id, j);
  }

  /** Para o trackId, devolve o job mais recente. */
  forTrack(trackId: string) { return this.latest.get(trackId); }
}
export const downloads = new DownloadsStore();

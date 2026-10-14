// @req RF-10 RF-11
import { api, type DownloadJob } from '../ipc';

class DownloadsStore {
  jobs = $state<DownloadJob[]>([]);
  loading = $state(true);
  error = $state<string | null>(null);
  private started = false;

  get active() { return this.jobs.filter((j) => j.status === 'queued' || j.status === 'downloading'); }
  get activeCount() { return this.active.length; }

  async start() {
    if (this.started) return;
    this.started = true;
    await api.on('download://progress', (job) => {
      const i = this.jobs.findIndex((j) => j.id === job.id);
      if (i >= 0) this.jobs[i] = job; else this.jobs.push(job);
    });
    await this.reload();
  }

  async reload() {
    try { this.jobs = await api.downloadList(); this.error = null; }
    catch (e) { this.error = (e as { message?: string })?.message ?? 'Falha ao carregar downloads.'; }
    finally { this.loading = false; }
  }

  async clearFinished() { // @req RF-10
    const count = await api.downloadClearFinished();
    this.jobs = this.jobs.filter((j) => j.status !== 'done' && j.status !== 'error');
    return count;
  }

  /** Para o trackId, devolve o job mais recente. */
  forTrack(trackId: string) {
    for (let i = this.jobs.length - 1; i >= 0; i--) if (this.jobs[i].trackId === trackId) return this.jobs[i];
    return undefined;
  }
}
export const downloads = new DownloadsStore();

// @req RF-31
import type { DownloadJob } from './types';

/** Job automático persistido como erro enquanto aguarda a rede. */
export function isWaitingNetwork(job: DownloadJob): boolean {
  return job.status === 'error' && (job.error?.startsWith('auto_offline:') ?? false);
}

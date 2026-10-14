// @req RF-25
import { api, type TrackScore } from '../ipc';

/** Pontuação das faixas. Pedidos feitos no mesmo ciclo (uma lista renderizando) viram uma chamada só. */
class ScoresStore {
  values = $state<Record<string, TrackScore>>({});
  /** Lido por `load`: quem chama dentro de um `$effect` pede de novo depois de `invalidate`. */
  version = $state(0);
  private pending = new Set<string>();
  private requested = new Set<string>();
  private flushing: Promise<void> | null = null;

  load(ids: string[]) {
    void this.version;
    for (const id of ids) if (!this.requested.has(id)) this.pending.add(id);
    if (!this.pending.size) return this.flushing ?? Promise.resolve();
    this.flushing ??= Promise.resolve().then(() => this.flush());
    return this.flushing;
  }
  private async flush() {
    const ids = [...this.pending];
    this.pending.clear();
    this.flushing = null;
    ids.forEach((id) => this.requested.add(id));
    try {
      const scores = await api.statsTrackScores({ trackIds: ids });
      this.values = { ...this.values, ...Object.fromEntries(scores.map((s) => [s.trackId, s])) };
    } catch {
      ids.forEach((id) => this.requested.delete(id));
    }
  }
  /** Esquece o que foi carregado (ex.: depois de tocar ou favoritar) para pedir de novo. */
  invalidate() { this.requested.clear(); this.version++; }
  get(id: string) { return this.values[id]; }
}
export const scores = new ScoresStore();

export function fmtScore(n: number) {
  return n.toLocaleString('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 });
}

/** Texto do tooltip da pontuação: "3 plays, 1 pulo, última vez: 08/10/2026". */
export function scoreTooltip(s: TrackScore) {
  const plays = `${s.plays} ${s.plays === 1 ? 'play' : 'plays'}`;
  const skips = `${s.skips} ${s.skips === 1 ? 'pulo' : 'pulos'}`;
  const last = s.lastPlayedAt ? new Date(s.lastPlayedAt).toLocaleDateString('pt-BR') : 'nunca';
  return `Pontuação: ${plays}, ${skips}, última vez: ${last}`;
}

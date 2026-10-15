// @req RF-25 RNF-13
import { SvelteMap } from 'svelte/reactivity';
import { api, type TrackScore } from '../ipc';
import { deepEqual } from '../util/diff';

/**
 * Pontuação das faixas. Pedidos feitos no mesmo ciclo (uma lista renderizando) viram uma chamada só.
 * Cada faixa tem a própria entrada reativa: chegar a pontuação de uma não repinta as outras linhas (RNF-13).
 */
class ScoresStore {
  private readonly map = new SvelteMap<string, TrackScore>();
  private pending = new Set<string>();
  private requested = new Set<string>();
  private flushing: Promise<void> | null = null;

  load(ids: string[]) {
    for (const id of ids) if (!this.requested.has(id)) { this.requested.add(id); this.pending.add(id); }
    return this.schedule();
  }
  private schedule() {
    if (!this.pending.size) return this.flushing ?? Promise.resolve();
    this.flushing ??= Promise.resolve().then(() => this.flush());
    return this.flushing;
  }
  private async flush() {
    const ids = [...this.pending];
    this.pending.clear();
    this.flushing = null;
    try {
      const scores = await api.statsTrackScores({ trackIds: ids });
      for (const s of scores) if (!deepEqual(this.map.get(s.trackId), s)) this.map.set(s.trackId, s);
    } catch {
      ids.forEach((id) => this.requested.delete(id));
    }
  }
  /**
   * Pede de novo, numa chamada só, a pontuação de `ids` (ou de tudo que já foi carregado). Sem isso,
   * tocar uma faixa recarregava a lista inteira por causa de uma linha (RNF-13).
   */
  invalidate(ids?: string[]) {
    for (const id of ids ?? this.requested) if (this.requested.has(id)) this.pending.add(id);
    void this.schedule();
  }
  /** Quem lê checa antes de usar (`{#if scores.get(id)}`); o tipo não inclui `undefined` por compatibilidade. */
  get(id: string) { return this.map.get(id) as TrackScore; }
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

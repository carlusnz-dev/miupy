// @req RF-01 RF-02 RF-03 RF-04
import { api, type PlayerState, type Track } from '../ipc';
import { notifyError } from './toast.svelte';

const EMPTY: PlayerState = { status: 'stopped', track: null, positionMs: 0, durationMs: 0, volume: 0.8, muted: false, queue: [], queueIndex: -1 };

class PlayerStore {
  state = $state<PlayerState>({ ...EMPTY });
  /** Faixas da fila resolvidas (a fila do contrato só traz ids). */
  queueTracks = $state<Track[]>([]);
  private started = false;
  private lastQueueKey = '';

  get track() { return this.state.track; }
  get isPlaying() { return this.state.status === 'playing'; }
  get isLoading() { return this.state.status === 'loading'; }
  get active() { return this.state.track !== null; }
  get progress() { return this.state.durationMs > 0 ? this.state.positionMs / this.state.durationMs : 0; }
  get hasNext() { return this.state.queueIndex >= 0 && this.state.queueIndex + 1 < this.state.queue.length; }

  async start() {
    if (this.started) return;
    this.started = true;
    await api.on('player://state', (s) => this.apply(s));
    await api.on('player://progress', (p) => {
      this.state.positionMs = p.positionMs;
      this.state.durationMs = p.durationMs;
    });
    // favorito alterado fora do player (biblioteca) reflete na faixa atual
    await api.on('library://changed', () => { void this.refreshCurrent(); });
    try { this.apply(await api.playerGetState()); } catch (e) { notifyError(e); }
  }

  private apply(s: PlayerState) {
    this.state = s;
    const key = s.queue.join(',');
    if (key !== this.lastQueueKey) { this.lastQueueKey = key; void this.resolveQueue(s.queue); }
  }

  private async resolveQueue(ids: string[]) {
    const out: Track[] = [];
    for (const id of ids) {
      try { out.push(await api.libraryGetTrack({ trackId: id })); } catch { /* faixa removida */ }
    }
    if (ids.join(',') === this.lastQueueKey) this.queueTracks = out;
  }

  private async refreshCurrent() {
    const t = this.state.track;
    if (!t) return;
    try {
      const fresh = await api.libraryGetTrack({ trackId: t.id });
      if (this.state.track?.id === fresh.id) this.state.track = fresh;
      this.queueTracks = this.queueTracks.map((q) => (q.id === fresh.id ? fresh : q));
    } catch { /* ignora */ }
  }

  private async run(p: Promise<PlayerState>) {
    try { this.apply(await p); } catch (e) { notifyError(e); }
  }

  play(trackId: string, queue?: string[]) { return this.run(api.playerPlay({ trackId, queue })); }
  pause() { return this.run(api.playerPause()); }
  resume() { return this.run(api.playerResume()); }
  stop() { return this.run(api.playerStop()); }
  toggle() {
    if (this.state.status === 'playing' || this.state.status === 'loading') return this.pause();
    if (this.state.status === 'paused') return this.resume();
    return Promise.resolve();
  }
  next() { return this.run(api.playerNext()); }
  previous() { return this.run(api.playerPrevious()); }
  seek(positionMs: number) {
    this.state.positionMs = positionMs; // otimista; o evento confirma
    return this.run(api.playerSeek({ positionMs }));
  }
  seekBy(deltaMs: number) {
    return this.seek(Math.max(0, Math.min(this.state.durationMs, this.state.positionMs + deltaMs)));
  }
  setVolume(volume: number) {
    const v = Math.max(0, Math.min(1, volume));
    this.state.volume = v;
    return this.run(api.playerSetVolume({ volume: v }));
  }
  setMuted(muted: boolean) { return this.run(api.playerSetMuted({ muted })); }
  toggleMuted() { return this.setMuted(!this.state.muted); }
}
export const player = new PlayerStore();
